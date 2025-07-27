use skia_safe::{Canvas, Paint, PaintStyle, Rect};

use crate::{
    animation::properties::Properties,
    svg::element::Shape,
    util::{ChronuxResult, sort_paints},
};

pub mod viewport;

pub fn draw(canvas: &Canvas, shape: &Shape, properties: &Properties) -> ChronuxResult<()> {
    if !properties.visible || properties.opacity <= 1e-3 {
        return Ok(());
    }

    let fill = properties.get_fill_paint();
    let stroke = properties.get_stroke_paint();

    let paints = sort_paints(properties.paint_order, stroke, fill);

    match shape {
        Shape::Path { d } => {
            let effects = properties.get_effects();
            let mut paints = paints;
            for e in effects.into_iter() {
                paints[0].set_path_effect(e.clone());
                paints[1].set_path_effect(e);
            }

            canvas.draw_path(&d, &paints[0]);
            canvas.draw_path(&d, &paints[1]);
        }
        Shape::Circle { cx, cy, r } => {
            let (cx, cy, r) = (*cx, *cy, *r);
            for paint in paints {
                canvas.draw_arc(
                    Rect::new(cx - r, cy - r, cx + r, cy + r),
                    -90.0,
                    properties.sweep * 360.0,
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
            let mut paint = Paint::default();
            paint.set_alpha((properties.opacity * 255.0) as u8);
            canvas.draw_image_rect(
                skia_img,
                None,
                Rect::new(*x, *y, x + width, y + height),
                &paint,
            );
        }
    }

    Ok(())
}

pub fn get_bounding_box(shape: &Shape, properties: &Properties) -> (f32, f32, f32, f32) {
    match shape {
        Shape::Path { d } => {
            const SEGMENTS: usize = 20;
            let mut pm = skia_safe::PathMeasure::new(d, false, None);
            let length = pm.length();
            let (tl, tr) = properties.trim;
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
            rect.expect("")
        }
        _ => {
            log::warn!(
                "Bounding box calculation for {:?} is not implemented",
                shape
            );
            (0.0, 0.0, 0.0, 0.0) // Return a default value
        }
    }
}
