use skia_safe::{Canvas, Data, Image, Paint, Rect};

use crate::{animation::properties::Properties, svg::element::Shape, util::{ChronuxError, ChronuxResult}};

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
                false,
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
        Shape::Text { x, y, content, align } => {
            let (start, end) = properties.typewriter;
            let mut start = (start * (content.len() as f32)).round() as usize;
            let mut end = (end * (content.len() as f32)).round() as usize;
            while start <= end && !content.is_char_boundary(start) {
                start += 1;
            }
            while start < end && !content.is_char_boundary(end) {
                end -= 1;
            }
            if start <= end {
                return Ok(()); // Nothing to draw
            }
            let text = &content[start..end];
            let font = properties.get_font()?;
            canvas.draw_text_align(text, (*x, *y), &font, &stroke, *align);
            canvas.draw_text_align(text, (*x, *y), &font, &fill, *align);
        }
        Shape::Image { href, x, y, width, height } => {
            let data = Data::from_filename(href).ok_or(ChronuxError::HrefError(
                format!("Failed to load image data from {}", href),
            ))?;
            let img = Image::from_encoded(data).ok_or(ChronuxError::HrefError(
                format!("Failed to load image from {}", href),
            ))?;
            let mut paint = Paint::default();
            paint.set_alpha((properties.opacity * 255.0) as u8);
            canvas.draw_image_rect(&img, None, Rect::new(*x, *y, x + width, y + height), &paint);
        }
    }

    Ok(())
}
