use skia_safe::{Canvas, Point, utils::text_utils::Align};

use crate::{
    config::ScaleConfig,
    util::{ChronuxResult, fontmgr::get_font},
    widget::{WIDGET_CANVAS_HEIGHT, WIDGET_CANVAS_WIDTH},
};

pub fn draw(
    canvas: &Canvas,
    scale: &ScaleConfig,
    viewbox_width: f32,
    font_family: &str,
) -> ChronuxResult<()> {
    let mut fill = skia_safe::Paint::default();
    fill.set_anti_alias(true);
    fill.set_color(skia_safe::Color::BLACK);
    fill.set_style(skia_safe::PaintStyle::Fill);

    let mut stroke_text = skia_safe::Paint::default();
    stroke_text.set_anti_alias(true);
    stroke_text.set_color(skia_safe::Color::WHITE);
    stroke_text.set_stroke_width(2.0);
    stroke_text.set_style(skia_safe::PaintStyle::Stroke);

    let mut stroke_scale = skia_safe::Paint::default();
    stroke_scale.set_anti_alias(true);
    stroke_scale.set_color(skia_safe::Color::BLACK);
    stroke_scale.set_stroke_width(2.0);
    stroke_scale.set_style(skia_safe::PaintStyle::Stroke);
    stroke_scale.set_stroke_cap(skia_safe::PaintCap::Round);

    let resolution = 40_075.016 * 1000.0 / 512.0 * scale.lat.to_radians().cos()
        / (scale.zoom.exp2())
        * viewbox_width
        / WIDGET_CANVAS_WIDTH; // resolution, in meters per pixel

    let options = [
        1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000,
    ];

    let meters = options.into_iter().find(|r| {
        let px = *r as f32 / resolution;
        (100.0..=300.0).contains(&px)
    });

    if meters.is_none() {
        // 找不到合适的比例尺，无法绘制
        return Ok(());
    }

    let meters = meters.unwrap() as f32;
    let length = meters / resolution;

    let font = get_font(font_family, 20.0, 400)?;
    let height = font.measure_str("0", Some(&fill)).1.height();

    // 在画布的左下角绘制比例尺
    let x1 = scale.padding;
    let x2 = scale.padding + 300.0;
    let y = WIDGET_CANVAS_HEIGHT - scale.padding - height - 5.0 - 140.0;

    let mut current_x = x1;
    let mut loops = 0;
    while current_x <= x2 {
        // 绘制刻度
        canvas.draw_line((current_x, y), (current_x, y - height), &stroke_scale);

        // 在刻度下方绘制文本，居中对齐
        let text = match meters * loops as f32 {
            0.0 => "0".to_string(),
            x if x < 1000.0 => format!("{x} m"),
            x if x >= 1000.0 => format!("{} km", x / 1000.0),
            _ => unreachable!("Invalid meter value: {}", meters),
        };

        canvas.draw_text_align(
            &text,
            Point::new(current_x, y + height + 5.0),
            &font,
            &stroke_text,
            Align::Center,
        );
        canvas.draw_text_align(
            &text,
            Point::new(current_x, y + height + 5.0),
            &font,
            &fill,
            Align::Center,
        );

        current_x += length;
        loops += 1;
    }

    // 绘制比例尺线段
    canvas.draw_line((current_x - length, y), (x1, y), &stroke_scale);

    Ok(())
}
