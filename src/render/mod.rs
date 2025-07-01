use skia_safe::{Canvas, Data, Image, Paint, PaintStyle, Rect};

use crate::{
    animation::properties::Properties,
    svg::element::Shape,
    util::{ChronuxError, ChronuxResult, sort_paints},
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
            href,
            x,
            y,
            width,
            height,
        } => {
            let data = Data::from_filename(href).ok_or(ChronuxError::HrefError(format!(
                "Failed to load image data from {}",
                href
            )))?;
            let img = Image::from_encoded(data).ok_or(ChronuxError::HrefError(format!(
                "Failed to load image from {}",
                href
            )))?;
            let mut paint = Paint::default();
            paint.set_alpha((properties.opacity * 255.0) as u8);
            canvas.draw_image_rect(&img, None, Rect::new(*x, *y, x + width, y + height), &paint);
        }
    }

    Ok(())
}
