use skia_safe::{utils::text_utils::{draw_text, Align}, Canvas};

use crate::{animation::properties::Properties, svg::element::Shape, util::ChronuxResult};

pub fn draw(canvas: &Canvas, shape: &Shape, properties: &Properties) -> ChronuxResult<()> {
    if !properties.visible {
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
            let center = (*cx, *cy);
            let radius = *r;
            canvas.draw_circle(center, radius, &fill);
            canvas.draw_circle(center, radius, &stroke);
        }
        Shape::Text { x, y, content } => {
            let (_start, end) = properties.typewriter;
            let mut end = (end * content.len() as f32).round() as usize;
            while end > 0 && !content.is_char_boundary(end) {
                end -= 1;
            }
            let text = &content[0..end];
            let font = properties.get_font()?;
            draw_text(canvas, text, (*x, *y), &font, &stroke, Align::Left);
            draw_text(canvas, text, (*x, *y), &font, &fill, Align::Left);
        }
    }

    Ok(())
}
