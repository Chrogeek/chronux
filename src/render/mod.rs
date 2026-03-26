use std::collections::HashMap;

use skia_safe::{
    Canvas, ClipOp, Matrix, Paint, PaintStyle, RRect, Rect, TileMode, Vector,
    canvas::{SaveLayerFlags, SaveLayerRec},
    image_filters::blur,
};

use crate::{
    animation::properties::Properties,
    config::{counter::Counters, extra_stroke::ExtraStrokeConfig},
    svg::{defs::SvgDef, element::Shape},
    util::{TextDrawPass, error::ChronuxError, error::ChronuxResult, sort_paints},
};

pub fn draw(
    canvas: &Canvas,
    properties: &Properties,
    min_stroke_width: f32,
    counters: &Counters,
    viewport_rect: &Rect,
    extra_strokes: &Vec<ExtraStrokeConfig>,
    defs: &HashMap<String, SvgDef>,
    element_transform: Matrix,
) -> ChronuxResult<()> {
    if !properties.visible || properties.opacity < 1e-4 {
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

    let paints = sort_paints(properties.paint_order, stroke, fill);

    let bboxcenter =
        get_bounding_box(properties).map(|(l, t, r, b)| ((l + r) * 0.5, (t + b) * 0.5));

    // 将画布绕 bboxcenter 点旋转 properties.rotate 角度(deg)
    if properties.rotate.abs() > 1e-4
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
                paints[1].set_path_effect(e);
            }

            let restore_handle = canvas.save();
            canvas.translate(Vector::new(properties.translate_x, properties.translate_y));

            // TODO: 这里的缩放操作可能与预期不同，后期使用时需要检查
            canvas.scale((properties.scale_x, properties.scale_y));

            properties.blur.inspect(|bl| {
                let blur_bounds = d.bounds();
                blur((*bl, *bl), TileMode::Clamp, None, blur_bounds).inspect(|filter| {
                    canvas.clip_path(&d, ClipOp::Intersect, true);
                    let save_layer_rec = SaveLayerRec::default()
                        .bounds(d.bounds())
                        .backdrop(&filter)
                        .flags(SaveLayerFlags::INIT_WITH_PREVIOUS);
                    canvas.save_layer(&save_layer_rec);
                });
            });

            canvas.draw_path(&d, &paints[0]);
            canvas.draw_path(&d, &paints[1]);

            canvas.restore_to_count(restore_handle);
        }
        Shape::Circle { cx, cy, r } => {
            let (cx, cy, rx, ry) = (
                cx + properties.translate_x,
                cy + properties.translate_y,
                r * properties.scale_x,
                r * properties.scale_y,
            );
            properties.blur.inspect(|_| {
                log::warn!("Blur effect for Circle is not implemented, skipping blur");
            });
            let (sweep_start, sweep_end) = properties.sweep;
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
            properties.blur.inspect(|bl| {
                let blur_bounds = rect;
                blur((*bl, *bl), TileMode::Clamp, None, blur_bounds).inspect(|filter| {
                    if rx > 1e-4 || ry > 1e-4 {
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
                if rx > 1e-4 || ry > 1e-4 {
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
            for pass in [TextDrawPass::Stroke, TextDrawPass::Fill] {
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
            if properties.opacity > 1e-4 {
                let (w, h) = (width * properties.scale_x, height * properties.scale_y);
                let dst_rect = Rect::new(
                    x + properties.translate_x + width * 0.5 - w * 0.5,
                    y + properties.translate_y + height * 0.5 - h * 0.5,
                    x + properties.translate_x + width * 0.5 + w * 0.5,
                    y + properties.translate_y + height * 0.5 + h * 0.5,
                );
                properties.blur.inspect(|bl| {
                    let blur_bounds = dst_rect;
                    blur((*bl, *bl), TileMode::Clamp, None, blur_bounds).inspect(|filter| {
                        canvas.clip_rect(dst_rect, ClipOp::Intersect, true);
                        let save_layer_rec = SaveLayerRec::default()
                            .bounds(&dst_rect)
                            .backdrop(&filter)
                            .flags(SaveLayerFlags::INIT_WITH_PREVIOUS);
                        canvas.save_layer(&save_layer_rec);
                    });
                });
                let mut paint = Paint::default();
                paint.set_alpha((properties.opacity * 255.0) as u8);
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
                    extra_strokes,
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
    if properties.opacity < 1e-4 || !properties.visible {
        return None; // 不可见，无边界框
    }
    match &properties.shape {
        Shape::Path { d } => {
            const SEGMENTS: usize = 20;
            let mut pm = skia_safe::PathMeasure::new(d, false, None);
            let length = pm.length();
            let (tl, tr) = properties.trim;
            if (tr - tl) * length < 1e-4 {
                return None; // 没有有效的长度
            }
            let mut rect: Option<(f32, f32, f32, f32)> = None;
            (0..SEGMENTS).for_each(|i| {
                let dis = (i as f32 / (SEGMENTS - 1) as f32 * (tr - tl) + tl) * length;
                let pt = if let Some(pt) = pm.pos_tan(dis) {
                    pt.0
                } else {
                    return;
                };
                if let Some((rl, rt, rr, rb)) = rect {
                    rect = Some((rl.min(pt.x), rt.min(pt.y), rr.max(pt.x), rb.max(pt.y)));
                } else {
                    rect = Some((pt.x, pt.y, pt.x, pt.y));
                }
            });
            rect
        }
        _ => {
            // log::warn!("Bounding box calculation for {shape:?} is not implemented");
            None
        }
    }
}
