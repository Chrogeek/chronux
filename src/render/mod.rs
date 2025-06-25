use skia_safe::{Canvas, Data, Image, Paint, Rect};

use crate::{
    animation::properties::Properties,
    svg::element::Shape,
    util::{ChronuxError, ChronuxResult},
};

pub mod viewport;

pub fn draw(canvas: &Canvas, shape: &Shape, properties: &Properties) -> ChronuxResult<()> {
    if !properties.visible || properties.opacity <= 1e-3 {
        return Ok(());
    }

    let fill = properties.get_fill_paint();
    let stroke = properties.get_stroke_paint();

    match shape {
        Shape::Path { d } => {
            let effects = properties.get_effects();
            let (mut fill, mut stroke) = (fill, stroke);
            for e in effects.into_iter() {
                fill.set_path_effect(e.clone());
                stroke.set_path_effect(e);
            }

            canvas.draw_path(&d, &fill);
            canvas.draw_path(&d, &stroke);
        }
        Shape::Circle { cx, cy, r } => {
            let (cx, cy, r) = (*cx, *cy, *r);
            canvas.draw_arc(
                Rect::new(cx - r, cy - r, cx + r, cy + r),
                -90.0,
                properties.sweep * 360.0,
                true,
                &fill,
            );
            canvas.draw_arc(
                Rect::new(cx - r, cy - r, cx + r, cy + r),
                -90.0,
                properties.sweep * 360.0,
                false,
                &stroke,
            );
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
