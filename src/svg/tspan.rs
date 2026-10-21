use std::{collections::HashMap, mem::MaybeUninit};

use roxmltree::Node;
use skia_bindings::SkTextBlob_Iter_ExperimentalRun;
use skia_safe::{
    Canvas, Font, FourByteTag, Matrix, Path, Point, Rect, Shaper, TextBlobIter,
    shaper::{Feature, TextBlobBuilderRunHandler},
    textlayout::TextDirection,
    utils::text_utils::Align,
    wrapper::ValueWrapper,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    animation::properties::Properties,
    config::counter::Counters,
    svg::{
        attribute_f32, attribute_str, attribute_transform, defs::SvgDef, parse_font_features,
        placeholder_text::PlaceholderText,
    },
    util::{
        EPS, TRANSPARENT, TextAnchor, TextDrawPass, error::ChronuxResult, paint::ColorPaint,
        trim_control_chars,
    },
};

#[derive(Debug, Clone)]
pub struct Tspan {
    pub x: f32,
    pub y: f32,
    pub text: TspanText,
    pub style: Properties,
    pub length: f32,
    pub preshaped: PreshapedText,
}

#[derive(Debug, Clone)]
pub enum TspanText {
    Text(String),
    Placeholder(PlaceholderText),
}

#[derive(Debug, Clone)]
pub struct PreshapedText {
    pub ascent: f32,
    pub bounds: skia_safe::Rect,
    pub glyphs: Vec<PreshapedGlyph>,
}

#[derive(Debug, Clone)]
pub struct PreshapedGlyph {
    pub path: Path,
    pub pos: Point,
}

impl Tspan {
    pub fn from_subtree(
        node: &Node,
        mat: Matrix,
        x: f32,
        y: f32,
        base_style: Properties,
        placeholder: &Option<PlaceholderText>,
    ) -> ChronuxResult<Vec<Tspan>> {
        fn dfs_node(
            node: &Node,
            mat: Matrix,
            x: f32,
            y: f32,
            base_style: Properties,
            result: &mut Vec<Tspan>,
            placeholder: &Option<PlaceholderText>,
            parent_features: &[Feature],
        ) -> ChronuxResult<()> {
            let style_str = attribute_str(node, "style").unwrap_or("".to_string());

            let base_style = match attribute_str(node, "style") {
                Ok(style) => Properties::from_style(&style, mat, base_style)?,
                Err(_) => base_style,
            };

            for child in node.children() {
                let x = attribute_f32(&child, "x").unwrap_or(x);
                let y = attribute_f32(&child, "y").unwrap_or(y);
                let dir = attribute_str(&child, "dir")
                    .ok()
                    .and_then(|s| match s.as_str() {
                        "ltr" => Some(TextDirection::LTR),
                        "rtl" => Some(TextDirection::RTL),
                        _ => None,
                    })
                    .unwrap_or(base_style.dir);
                if child.is_text() {
                    let text = child.text().unwrap_or("").to_string();
                    let text = trim_control_chars(&text);
                    let Point { x, y } = mat.map_point((x, y));
                    if !text.is_empty() {
                        let font = base_style.get_font()?;
                        // TODO: 这里的feature继承并不完全正确，后续应该根据MDN的文档进行调整（继承和覆盖）
                        let features = parse_font_features(&style_str, 0, text.len());
                        let features = parent_features
                            .iter()
                            .cloned()
                            .chain(features.into_iter())
                            .collect::<Vec<_>>();
                        let (preshaped, width) =
                            Tspan::preshape_text_align(&text, &font, dir, &features);
                        let tspan = Tspan {
                            x,
                            y,
                            style: base_style.clone(),
                            length: width,
                            text: match placeholder {
                                Some(formatter) => TspanText::Placeholder(formatter.clone()),
                                None => TspanText::Text(text.to_string()),
                            },
                            preshaped,
                        };
                        result.push(tspan);
                    }
                    continue;
                } else {
                    let features = parse_font_features(&style_str, 0, 0);
                    let features = parent_features
                        .iter()
                        .cloned()
                        .chain(features.into_iter())
                        .collect::<Vec<_>>();
                    let mat = mat * attribute_transform(&child);
                    dfs_node(
                        &child,
                        mat,
                        x,
                        y,
                        base_style.clone(),
                        result,
                        placeholder,
                        &features,
                    )?;
                }
            }
            Ok(())
        }

        let mut result = Vec::new();

        let coeff = match base_style.text_anchor {
            TextAnchor::Start => 0.0,
            TextAnchor::Middle => 0.5,
            TextAnchor::End => 1.0,
        };

        dfs_node(
            node,
            mat,
            x,
            y,
            base_style,
            &mut result,
            placeholder,
            &vec![],
        )?;

        if result.is_empty() {
            return Ok(result);
        }

        let mut new_xy = Vec::new();
        let mut total_length = 0.0;
        let mut start_id = 0;
        let (mut sx, mut _sy) = (result[0].x, result[0].y);

        for (id, tsp) in result.iter().enumerate() {
            total_length += tsp.length;
            if id + 1 == result.len() || tsp.x != result[id + 1].x || tsp.y != result[id + 1].y {
                let mut cx = match tsp.style.dir {
                    TextDirection::LTR => sx - total_length * coeff,
                    TextDirection::RTL => sx + total_length * coeff,
                };
                for ret in result.iter().take(id + 1).skip(start_id) {
                    // start_id..=id
                    new_xy.push((cx + ret.length * coeff, ret.y));
                    cx += ret.length;
                }

                total_length = 0.0;
                start_id = id + 1;
                if id + 1 < result.len() {
                    sx = result[id + 1].x;
                    _sy = result[id + 1].y;
                }
            }
        }

        for (i, (x, y)) in new_xy.iter().enumerate() {
            result[i].x = *x;
            result[i].y = *y;
        }

        Ok(result)
    }

    pub fn draw_text(
        &self,
        canvas: &Canvas,
        tl: f32,
        tr: f32,
        properties: Properties,
        counters: &Counters,
        defs: &HashMap<String, SvgDef>,
        element_transform: Matrix,
        pass: TextDrawPass,
    ) -> ChronuxResult<()> {
        let tl = tl.max(0.0);
        let tr = tr.min(self.length);

        let (tx, ty) = (properties.translate_x, properties.translate_y);

        let color_style = if properties.tweening {
            properties
        } else {
            Properties::merge(&self.style, &properties)
        };

        let color_style = match pass {
            TextDrawPass::Shadow => Properties {
                fill_color: ColorPaint::Color(TRANSPARENT),
                stroke_color: ColorPaint::Color(TRANSPARENT),
                ..color_style
            },
            TextDrawPass::Stroke => Properties {
                fill_color: ColorPaint::Color(TRANSPARENT),
                ..color_style
            },
            TextDrawPass::Fill => Properties {
                stroke_color: ColorPaint::Color(TRANSPARENT),
                ..color_style
            },
        };

        let font = self.style.get_font()?;

        let paint = match pass {
            TextDrawPass::Shadow => {
                if let Some(sh) = color_style.get_shadow_paint() {
                    sh
                } else {
                    // 如果没有阴影，则不绘制任何内容
                    return Ok(());
                }
            }
            TextDrawPass::Stroke => color_style.get_stroke_paint(defs, element_transform),
            TextDrawPass::Fill => color_style.get_fill_paint(defs, element_transform),
        };

        let dir_coeff = match self.style.dir {
            TextDirection::LTR => 1.0,
            TextDirection::RTL => -1.0,
        };

        let offset = match self.style.text_anchor {
            TextAnchor::Start => 0.0,
            TextAnchor::Middle => self.length / 2.0,
            TextAnchor::End => self.length,
        };

        let text = match &self.text {
            TspanText::Text(text) => text.clone(),
            TspanText::Placeholder(placeholder) => placeholder.format(counters),
        };

        if tr - tl < EPS {
            // Avoid drawing if the text length is too small
            // Do nothing here
        } else if tl < EPS && tr > self.length - EPS {
            let (x, y) = (self.x, self.y);
            match &self.text {
                TspanText::Placeholder(placeholder) => {
                    let text = placeholder.format(counters);
                    if self.style.small_fraction != 1.0
                        && let Some(pos) = text.find(".")
                    {
                        // split integer and fractional parts
                        let (int_part, frac_part) = text.split_at(pos + 1);
                        let width_int = font.measure_str(int_part, None).0;

                        let font_frac = self.style.get_frac_font()?;
                        let width_frac = font_frac.measure_str(frac_part, None).0;

                        let total_width = width_int + width_frac;
                        let offset = match self.style.text_anchor {
                            TextAnchor::Start => 0.0,
                            TextAnchor::Middle => total_width / 2.0,
                            TextAnchor::End => total_width,
                        };

                        let sx = x + tx - offset;
                        canvas.draw_text_align(int_part, (sx, y + ty), &font, &paint, Align::Left);
                        canvas.draw_text_align(
                            frac_part,
                            (sx + width_int, y + ty),
                            &font_frac,
                            &paint,
                            Align::Left,
                        );
                    } else {
                        canvas.draw_text_align(
                            &text,
                            (x + tx, y + ty),
                            &font,
                            &paint,
                            self.style.text_anchor.to_align(self.style.dir),
                        );
                    }
                }
                TspanText::Text(_) => {
                    let PreshapedText {
                        ascent,
                        bounds: _,
                        glyphs,
                    } = &self.preshaped;
                    for PreshapedGlyph { path, pos, .. } in glyphs {
                        let mut translated_path = path.clone();
                        translated_path.transform(&Matrix::translate(
                            Point::new(x + tx, y + ty) + *pos + Point::new(-offset, *ascent),
                        ));
                        canvas.draw_path(&translated_path, &paint);
                    }
                }
            }
        } else {
            match &self.text {
                TspanText::Placeholder(_) => {
                    let mut dlen = 0.0;
                    for g in text.graphemes(true) {
                        let c = self.style.measure_string(g)?;

                        if dlen + c < tl || dlen > tr {
                            // draw nothing
                        } else {
                            let tl = tl.max(dlen);
                            let tr = tr.min(dlen + c);
                            let alpha = (tr - tl) / c;
                            let (x, y) = (self.x + (dlen - offset) * dir_coeff, self.y);

                            let mut new_paint = paint.clone();

                            new_paint.set_alpha((alpha * paint.alpha() as f32) as u8);
                            canvas.draw_text_align(
                                g,
                                (x + tx, y + ty),
                                &font,
                                &new_paint,
                                TextAnchor::Start.to_align(self.style.dir),
                            );
                        }

                        dlen += c;
                    }
                }
                TspanText::Text(_) => {
                    for g in self.preshaped.glyphs.iter() {
                        let dlen = g.pos.x + g.path.bounds().left;
                        let c = g.path.bounds().width();

                        if dlen + c < tl || dlen > tr {
                            // draw nothing
                        } else {
                            let tl = tl.max(dlen);
                            let tr = tr.min(dlen + c);
                            let alpha = (tr - tl) / c;

                            let (x, y) = (self.x, self.y);

                            let mut new_paint = paint.clone();
                            new_paint.set_alpha((alpha * paint.alpha() as f32) as u8);

                            let mut translated_path = g.path.clone();
                            translated_path.transform(&Matrix::translate(
                                Point::new(x + tx, y + ty)
                                    + g.pos
                                    + Point::new(-offset, self.preshaped.ascent),
                            ));
                            canvas.draw_path(&translated_path, &new_paint);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn preshape_text_align(
        text: &str,
        font: &Font,
        dir: TextDirection,
        features: &[Feature],
    ) -> (PreshapedText, f32) {
        let text = text.to_string() + "\u{200B}"; // 添加零宽空格以确保最后一个字符的宽度被正确计算

        let fontmgr = skia_safe::FontMgr::new();
        let shaper = Shaper::new(fontmgr.clone());

        // Unicode 双向性迭代器（需要 ICU）
        let mut bidi_iter = Shaper::new_bidi_run_iterator(
            &text,
            match dir {
                TextDirection::RTL => 1,
                TextDirection::LTR => 0,
            },
        )
        .unwrap();

        // 脚本迭代器
        // TODO: 目前写死了文种为 Hans，后续应考虑添加自定义文种的设置支持
        let script_code = FourByteTag::from_chars('H', 'a', 'n', 's');
        let mut script_iter = Shaper::new_script_run_iterator(&text, script_code);

        // 字体运行迭代器
        let mut font_run_iter = Shaper::new_font_mgr_run_iterator(&text, font, fontmgr);

        // 语言迭代器
        let mut lang_iter = Shaper::new_std_language_run_iterator(&text).unwrap();

        let mut rh = TextBlobBuilderRunHandler::new(&text, Point::new(0.0, 0.0));

        shaper.shape_with_iterators_and_features(
            &text,
            &mut font_run_iter,
            &mut bidi_iter,
            &mut script_iter,
            &mut lang_iter,
            &features,
            f32::INFINITY,
            &mut rh,
        );

        let preshaped = rh.make_blob().unwrap();

        let mut bounds = Rect::new_empty();

        let mut iter = TextBlobIter::new(&preshaped);
        let iter = iter.inner_mut();
        let mut glyphs = Vec::new();
        let mut ascent = 0.0f32;
        loop {
            let mut run = SkTextBlob_Iter_ExperimentalRun {
                font: font.clone().unwrap(),
                count: 0,
                glyphs: std::ptr::null(),
                positions: std::ptr::null(),
            };
            unsafe {
                if iter.experimentalNext(&mut run) {
                    let glyph_count = run.count;

                    let tff = run.font;

                    let run_glyphs = std::slice::from_raw_parts(run.glyphs, glyph_count as usize);
                    let paths = run_glyphs.iter().map(|g| {
                        let mut p = Path::new().unwrap();
                        tff.getPath(*g, &mut p);
                        Path::wrap(p)
                    });

                    #[allow(invalid_value)]
                    let mut metrics = MaybeUninit::uninit().assume_init();
                    tff.getMetrics(&mut metrics);

                    ascent = ascent.min(metrics.fAscent);

                    let pos = std::slice::from_raw_parts(run.positions, glyph_count as usize)
                        .iter()
                        .map(|p| Point::new(p.fX, p.fY))
                        .collect::<Vec<_>>();

                    for (p, pa) in pos.iter().zip(paths.clone()) {
                        bounds.join(Rect::new(
                            p.x,
                            p.y,
                            p.x + pa.bounds().width(),
                            p.y + pa.bounds().height(),
                        ));
                    }

                    glyphs.extend(
                        paths
                            .zip(pos.into_iter())
                            .map(|(path, pos)| PreshapedGlyph { pos, path }),
                    );
                } else {
                    break;
                }
            }
        }

        let width = glyphs.last().map(|g| g.pos.x).unwrap_or(0.0);
        glyphs.pop(); // 移除零宽空格对应的字形

        (
            PreshapedText {
                ascent,
                bounds,
                glyphs,
            },
            width,
        )
    }
}
