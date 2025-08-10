use skia_safe::{Canvas, Paint, PaintStyle, Rect, Vector};

use crate::{
    animation::properties::Properties,
    config::counter::Counters,
    svg::element::Shape,
    util::{ChronuxError, ChronuxResult, sort_paints},
};

pub fn draw(
    canvas: &Canvas,
    shape: &Shape,
    properties: &Properties,
    min_stroke_width: f32,
    counters: &Counters,
    viewport_rect: &Rect,
) -> ChronuxResult<()> {
    if !properties.visible || properties.opacity <= 1e-3 {
        return Ok(());
    }

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

    let fill = properties.get_fill_paint();
    let mut stroke = properties.get_stroke_paint();
    stroke.set_stroke_width(stroke.stroke_width().max(min_stroke_width));

    let paints = sort_paints(properties.paint_order, stroke, fill);

    match shape {
        Shape::Path { d } => {
            let effects = properties.get_effects();
            let mut paints = paints;
            for e in effects.into_iter() {
                paints[0].set_path_effect(e.clone());
                paints[1].set_path_effect(e);
            }

            let restore_handle = canvas.save();
            canvas.translate(Vector::new(properties.translate_x, properties.translate_y));

            canvas.draw_path(d, &paints[0]);
            canvas.draw_path(d, &paints[1]);

            canvas.restore_to_count(restore_handle);
        }
        Shape::Circle { cx, cy, r } => {
            let (cx, cy, r) = (
                *cx + properties.translate_x,
                *cy + properties.translate_y,
                *r,
            );
            let (sweep_start, sweep_end) = properties.sweep;
            for paint in paints {
                canvas.draw_arc(
                    Rect::new(cx - r, cy - r, cx + r, cy + r),
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
        Shape::Text { children, .. } => {
            let (start, end) = properties.typewriter;
            let total_len = children.iter().map(|s| s.length).sum::<f32>();

            let start = start * total_len;
            let end = end * total_len;

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
                    )?;
                }
                drawn_length += c.length;
            }
        }
        Shape::Image {
            x,
            y,
            width,
            height,
            skia_img,
        } => {
            let dst_rect = Rect::new(
                *x + properties.translate_x,
                *y + properties.translate_y,
                x + width + properties.translate_x,
                y + height + properties.translate_y,
            );
            if !viewport_rect.intersects(dst_rect) {
                return Ok(()); // 不在视口内，跳过绘制
            }
            let mut paint = Paint::default();
            paint.set_alpha((properties.opacity * 255.0) as u8);
            canvas.draw_image_rect(
                skia_img,
                None,
                dst_rect,
                &paint,
            );
        }
        Shape::Group { objects } => {
            for obj in objects {
                draw(
                    canvas,
                    &obj.shape,
                    &Properties::merge(&obj.style, properties),
                    min_stroke_width,
                    counters,
                    viewport_rect,
                )?;
            }
        }
    }

    Ok(())
}

pub fn get_bounding_box(shape: &Shape, properties: &Properties) -> Option<(f32, f32, f32, f32)> {
    match shape {
        Shape::Path { d } => {
            const SEGMENTS: usize = 20;
            let mut pm = skia_safe::PathMeasure::new(d, false, None);
            let length = pm.length();
            let (tl, tr) = properties.trim;
            if tr - tl < 1e-3 {
                return None; // 没有有效的长度
            }
            let mut rect: Option<(f32, f32, f32, f32)> = None;
            (0..SEGMENTS).for_each(|i| {
                let dis = (i as f32 / (SEGMENTS - 1) as f32 * (tr - tl) + tl) * length;
                let pt = pm.pos_tan(dis).unwrap().0;
                if let Some((rl, rt, rr, rb)) = rect {
                    rect = Some((rl.min(pt.x), rt.min(pt.y), rr.max(pt.x), rb.max(pt.y)));
                } else {
                    rect = Some((pt.x, pt.y, pt.x, pt.y));
                }
            });
            rect
        }
        _ => {
            log::warn!(
                "Bounding box calculation for {shape:?} is not implemented"
            );
            None
        }
    }
}
