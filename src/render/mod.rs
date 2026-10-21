use std::collections::HashMap;

use skia_safe::{
    Canvas, ClipOp, Matrix, Paint, PaintCap, PaintStyle, Path, PathDirection, PathMeasure, Point,
    RRect, Rect, StrokeRec, TileMode, Vector,
    canvas::{SaveLayerFlags, SaveLayerRec},
    image_filters::blur,
};

use crate::{
    animation::properties::Properties,
    config::counter::Counters,
    svg::{defs::SvgDef, element::Shape},
    util::{
        EPS, TextDrawPass,
        error::{ChronuxError, ChronuxResult},
        sort_paints,
    },
};

pub fn draw(
    canvas: &Canvas,
    properties: &Properties,
    min_stroke_width: f32,
    counters: &Counters,
    viewport_rect: &Rect,
    defs: &HashMap<String, SvgDef>,
    element_transform: Matrix,
) -> ChronuxResult<()> {
    if !properties.visible || properties.opacity < EPS {
        return Ok(());
    }

    let restore_handle = canvas.save();

    let mat = canvas.local_to_device_as_3x3();
    let min_stroke_width = mat
        .invert()
        .ok_or_else(|| {
            log::error!("Non-invertible viewport transform: {mat:?}");
            log::error!("min_stroke_width: {min_stroke_width}");
            ChronuxError::ArgError("Viewport transform must be invertible".to_string())
        })?
        .map_vector(Vector::new(min_stroke_width, 0.0))
        .length();

    let fill = properties.get_fill_paint(defs, element_transform);
    let mut stroke = properties.get_stroke_paint(defs, element_transform);
    stroke.set_stroke_width(stroke.stroke_width().max(min_stroke_width));

    let mut shadow = properties.get_shadow_paint();

    let paints = sort_paints(properties.paint_order, stroke, fill);

    let bounding_box = get_bounding_box(properties);
    let bboxcenter = bounding_box.map(|(l, t, r, b)| ((l + r) * 0.5, (t + b) * 0.5));

    // 圆形展开效果 (pie_in)
    if properties.pie_in <= 0.0 {
        canvas.restore_to_count(restore_handle);
        return Ok(());
    }

    if properties.pie_in < 1.0
        && let Some((bl, bt, br, bb)) = bounding_box
    {
        let pie_radius = (br - bl).hypot(bb - bt)
            * 0.5
            * properties.pie_in
            * properties.scale_x.max(properties.scale_y);
        let pie_center = Vector::new(
            (bl + br) * 0.5 + properties.translate_x,
            (bt + bb) * 0.5 + properties.translate_y,
        );
        let mut pie_clip = Path::new();
        pie_clip.add_circle(pie_center, pie_radius, PathDirection::CW);
        canvas.clip_path(&pie_clip, ClipOp::Intersect, true);
    }

    // 将画布绕 bboxcenter 点旋转 properties.rotate 角度(deg)
    if properties.rotate.abs() > EPS
        && let Some((cx, cy)) = bboxcenter
    {
        let cx = cx + properties.translate_x;
        let cy = cy + properties.translate_y;
        canvas.rotate(properties.rotate, Some((cx, cy).into()));
    }

    match &properties.shape {
        Shape::None => {}
        Shape::Path { d } => {
            let mut paints = paints;
            if let Some(e) = properties.get_effects() {
                paints[0].set_path_effect(e.clone());
                paints[1].set_path_effect(e.clone());
                shadow = shadow.map(|mut s| {
                    s.set_path_effect(e);
                    s
                });
            }

            let restore_handle = canvas.save();
            canvas.translate(Vector::new(properties.translate_x, properties.translate_y));

            // TODO: 这里的缩放操作可能与预期不同，后期使用时需要检查
            canvas.scale((properties.scale_x, properties.scale_y));

            shadow.inspect(|sh| {
                canvas.draw_path(&d, &sh);
            });
            properties.blur.inspect(|bl| {
                let bl = bl * properties.opacity;
                let blur_bounds = d.bounds();
                blur((bl, bl), TileMode::Clamp, None, blur_bounds).inspect(|filter| {
                    canvas.clip_path(&d, ClipOp::Intersect, true);
                    let save_layer_rec = SaveLayerRec::default()
                        .bounds(d.bounds())
                        .backdrop(&filter)
                        .flags(SaveLayerFlags::INIT_WITH_PREVIOUS);
                    canvas.save_layer(&save_layer_rec);
                });
            });

            let mut d = d.clone();
            d.set_fill_type(properties.fill_rule);

            let (ts, te) = properties.trim;

            if (ts >= 0.0 && ts <= 1.0) || (te >= 0.0 && te <= 1.0) || (ts <= 0.0 && te >= 1.0) {
                // 正常绘制路径
                canvas.draw_path(&d, &paints[0]);
                canvas.draw_path(&d, &paints[1]);
            } else {
                // 裁切区段不可见时，尝试绘制端头
                match properties.stroke_linecap {
                    PaintCap::Round => {
                        // 绘制端头
                        let mut pm = PathMeasure::new(&d, false, Some(2.0));
                        let path_len = pm.length();
                        let stroke_width = properties.stroke_width;
                        let cap_ratio = (stroke_width * 0.5) / path_len;

                        let (alpha, beta, pt) = if te <= 0.0 && te >= -cap_ratio {
                            let (pt, tan) = pm
                                .pos_tan(0.0)
                                .expect("Failed to get pos/tan at start of path");
                            let beta = tan.y.atan2(tan.x).to_degrees();
                            let alpha = (-te / cap_ratio).asin().to_degrees();

                            (alpha, beta, pt)
                        } else if ts >= 1.0 && ts <= 1.0 + cap_ratio {
                            let (pt, tan) = pm
                                .pos_tan(path_len)
                                .expect("Failed to get pos/tan at end of path");
                            let beta = tan.y.atan2(tan.x).to_degrees() + 180.0;
                            let alpha = ((ts - 1.0) / cap_ratio).asin().to_degrees();

                            (alpha, beta, pt)
                        } else {
                            (-1.0, 0.0, Point::new(0.0, 0.0))
                        };

                        if alpha >= 0.0 {
                            for paint in paints {
                                if paint.style() == PaintStyle::Stroke {
                                    let mut npt = paint.clone();
                                    npt.set_style(PaintStyle::Fill);
                                    canvas.draw_arc(
                                        Rect::new(
                                            pt.x - stroke_width * 0.5,
                                            pt.y - stroke_width * 0.5,
                                            pt.x + stroke_width * 0.5,
                                            pt.y + stroke_width * 0.5,
                                        ),
                                        alpha * 2.0 + beta,
                                        360.0 - 4.0 * alpha,
                                        false,
                                        &npt,
                                    );
                                }
                            }
                        }
                    }
                    PaintCap::Square => {} // TODO: 其他端点类型暂不支持
                    PaintCap::Butt => {}   // 无线头，什么都不用画
                }
            }

            canvas.restore_to_count(restore_handle);
        }
        Shape::Circle { cx, cy, r } => {
            let (cx, cy, rx, ry) = (
                cx + properties.translate_x,
                cy + properties.translate_y,
                r * properties.scale_x,
                r * properties.scale_y,
            );
            let (sweep_start, sweep_end) = properties.sweep;
            shadow.inspect(|sh| {
                canvas.draw_arc(
                    Rect::new(cx - rx, cy - ry, cx + rx, cy + ry),
                    -90.0 + sweep_start * 360.0,
                    (sweep_end - sweep_start) * 360.0,
                    false,
                    &sh,
                );
            });
            properties.blur.inspect(|_| {
                log::warn!("Blur effect for Circle is not implemented, skipping blur");
            });
            for paint in paints {
                canvas.draw_arc(
                    Rect::new(cx - rx, cy - ry, cx + rx, cy + ry),
                    -90.0 + sweep_start * 360.0,
                    (sweep_end - sweep_start) * 360.0,
                    match paint.style() {
                        PaintStyle::Fill => true,
                        PaintStyle::Stroke | PaintStyle::StrokeAndFill => false,
                    },
                    &paint,
                );
            }
        }
        Shape::Rect {
            x,
            y,
            width,
            height,
            rx,
            ry,
        } => {
            let (x, y) = (x + properties.translate_x, y + properties.translate_y);
            let (rx, ry) = (rx * properties.scale_x, ry * properties.scale_y);
            let (w, h) = (width * properties.scale_x, height * properties.scale_y);
            let rect = Rect::new(
                x + width * 0.5 - w * 0.5,
                y + height * 0.5 - h * 0.5,
                x + width * 0.5 + w * 0.5,
                y + height * 0.5 + h * 0.5,
            );
            shadow.inspect(|sh| {
                if rx > EPS || ry > EPS {
                    canvas.draw_round_rect(rect, rx, ry, &sh);
                } else {
                    canvas.draw_rect(rect, &sh);
                }
            });
            properties.blur.inspect(|bl| {
                let bl = bl * properties.opacity;
                let blur_bounds = rect;
                blur((bl, bl), TileMode::Clamp, None, blur_bounds).inspect(|filter| {
                    if rx > EPS || ry > EPS {
                        canvas.clip_rrect(
                            &RRect::new_rect_xy(rect, rx, ry),
                            ClipOp::Intersect,
                            true,
                        );
                    } else {
                        canvas.clip_rect(rect, ClipOp::Intersect, true);
                    }
                    let save_layer_rec = SaveLayerRec::default()
                        .bounds(&rect)
                        .backdrop(&filter)
                        .flags(SaveLayerFlags::INIT_WITH_PREVIOUS);
                    canvas.save_layer(&save_layer_rec);
                });
            });
            for paint in paints {
                if rx > EPS || ry > EPS {
                    canvas.draw_round_rect(rect, rx, ry, &paint);
                } else {
                    canvas.draw_rect(rect, &paint);
                }
            }
        }
        Shape::Text { children, .. } => {
            // TODO: 文本绘制暂不支持缩放 (scale_x, scale_y)

            let (start, end) = properties.typewriter;
            let total_len = children.iter().map(|s| s.length).sum::<f32>();

            let start = start * total_len;
            let end = end * total_len;

            properties.blur.inspect(|_| {
                log::warn!("Blur effect for Text is not implemented, skipping blur");
            });

            // 文本强制按照 描边-填充 顺序绘制
            for pass in [
                TextDrawPass::Shadow,
                TextDrawPass::Stroke,
                TextDrawPass::Fill,
            ] {
                let mut drawn_length = 0.0;
                for c in children.iter() {
                    let l = start.max(drawn_length);
                    let r = end.min(drawn_length + c.length);
                    if l >= r {
                        // 当前文本段不在范围内，跳过
                    } else {
                        // 当前文本段在范围内，计算绘制比例
                        c.draw_text(
                            canvas,
                            l - drawn_length,
                            r - drawn_length,
                            properties.clone(),
                            counters,
                            defs,
                            element_transform,
                            pass,
                        )?;
                    }
                    drawn_length += c.length;
                }
            }
        }
        Shape::Image {
            x,
            y,
            width,
            height,
            skia_img,
        } => {
            if properties.opacity > EPS {
                let (w, h) = (width * properties.scale_x, height * properties.scale_y);
                let dst_rect = Rect::new(
                    x + properties.translate_x + width * 0.5 - w * 0.5,
                    y + properties.translate_y + height * 0.5 - h * 0.5,
                    x + properties.translate_x + width * 0.5 + w * 0.5,
                    y + properties.translate_y + height * 0.5 + h * 0.5,
                );
                shadow.inspect(|sh| {
                    canvas.draw_rect(dst_rect, &sh);
                });
                properties.blur.inspect(|bl| {
                    let bl = bl * properties.opacity;
                    let blur_bounds = dst_rect;
                    blur((bl, bl), TileMode::Clamp, None, blur_bounds).inspect(|filter| {
                        canvas.clip_rect(dst_rect, ClipOp::Intersect, true);
                        let save_layer_rec = SaveLayerRec::default()
                            .bounds(&dst_rect)
                            .backdrop(&filter)
                            .flags(SaveLayerFlags::INIT_WITH_PREVIOUS);
                        canvas.save_layer(&save_layer_rec);
                    });
                });
                let mut paint = Paint::default();
                paint.set_alpha_f(properties.opacity);
                canvas.draw_image_rect(skia_img, None, dst_rect, &paint);
            }
        }
        Shape::Group { objects } => {
            for obj in objects {
                draw(
                    canvas,
                    &Properties::merge(&obj, properties),
                    min_stroke_width,
                    counters,
                    viewport_rect,
                    defs,
                    element_transform,
                )?;
            }
        }
    }

    canvas.restore_to_count(restore_handle);
    Ok(())
}

pub fn get_bounding_box(properties: &Properties) -> Option<(f32, f32, f32, f32)> {
    if !properties.is_visible() {
        return None; // 不可见，无边界框
    }
    if properties.trim.1 <= 0.0
        || properties.trim.0 >= 1.0
        || properties.trim.1 - properties.trim.0 < EPS
    {
        return None; // 裁切区段不可见，无边界框
    }
    match &properties.shape {
        Shape::Path { d } => {
            if let Some(effects) = properties.get_effects() {
                let mut res = Path::new();
                effects.filter_path_inplace(
                    &mut res,
                    d,
                    &mut StrokeRec::new_hairline(),
                    Rect::new(-f32::INFINITY, -f32::INFINITY, f32::INFINITY, f32::INFINITY),
                );

                let bounds = res.bounds();
                if bounds.is_empty() {
                    None
                } else {
                    Some((bounds.left, bounds.top, bounds.right, bounds.bottom))
                }
            } else {
                let bounds = d.bounds();
                Some((bounds.left, bounds.top, bounds.right, bounds.bottom))
            }
        }
        Shape::Circle { cx, cy, r } => Some((cx - r, cy - r, cx + r, cy + r)),
        Shape::Rect {
            x,
            y,
            width,
            height,
            ..
        }
        | Shape::Image {
            x,
            y,
            width,
            height,
            ..
        } => Some((*x, *y, x + width, y + height)),
        Shape::Group { objects } => {
            let mut bbox: Option<(f32, f32, f32, f32)> = None;
            for obj in objects {
                let child_bbox = get_bounding_box(&obj)?;
                bbox = Some(match bbox {
                    None => child_bbox,
                    Some((l, t, r, b)) => {
                        let (cl, ct, cr, cb) = child_bbox;
                        (l.min(cl), t.min(ct), r.max(cr), b.max(cb))
                    }
                });
            }
            bbox
        }
        _ => {
            // log::warn!(
            //     "Bounding box calculation for {:?} is not implemented",
            //     properties.shape
            // );
            None
        }
    }
}
