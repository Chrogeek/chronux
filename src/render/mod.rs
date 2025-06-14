use skia_safe::Canvas;

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
        Shape::Text {
            x,
            y,
            content,
        } => {
            let (start, end) = properties.typewriter;
            todo!()
            // canvas.draw_text_align(content, (*x, *y), todo!(), &stroke, todo!());
            // canvas.draw_text_align(content, (*x, *y), todo!(), &fill, todo!());
        }
    }

    Ok(())
}
