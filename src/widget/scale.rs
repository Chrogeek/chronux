use skia_safe::{Canvas, Point, utils::text_utils::Align};

use crate::{
    config::{ScaleAnchor, ScaleConfig},
    util::{fontmgr::get_font, ChronuxResult},
    widget::WIDGET_CANVAS_WIDTH,
};

pub fn draw(
    canvas: &Canvas,
    scale: &ScaleConfig,
    viewbox_width: f32,
    font_family: &str,
    x: f32,
    y: f32,
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
        1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0,
        20000.0, 50000.0, 100000.0,
    ];
    let suboptions = [
        0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0,
        10000.0, 20000.0,
    ];

    let size = scale.size;

    let meters = options
        .into_iter()
        .enumerate()
        .find(|(_, r)| {
            let px = size / (*r / resolution);
            (1.5..=3.0).contains(&px)
        })
        .map(|(i, _)| (options[i], suboptions[i]));

    if meters.is_none() {
        // 找不到合适的比例尺，无法绘制
        return Ok(());
    }

    let (major, minor) = meters.unwrap();
    let maj_len = major / resolution;
    let min_len = minor / resolution;

    let font = get_font(font_family, 20.0, 400)?;
    let height = font.measure_str("0", Some(&fill)).1.height();

    // 在画布的指定位置绘制比例尺
    let (x1, x2, dir) = match scale.anchor {
        ScaleAnchor::Left => (x + scale.padding, x + scale.padding + scale.size, 1.0),
        ScaleAnchor::Right => (x - scale.padding, x - scale.size - scale.padding, -1.0),
    };

    let mut current_x = x1;
    let mut loops = 0;

    // 绘制主要刻度
    loop {
        if (current_x - x2) * dir > 0.0 {
            // 超出范围，停止绘制
            break;
        }

        canvas.draw_line((current_x, y), (current_x, y - height), &stroke_scale);

        // 在刻度下方绘制文本，居中对齐
        let text = match major * loops as f32 {
            0.0 => "0".to_string(),
            x if x < 1000.0 => format!("{x} m"),
            x if x >= 1000.0 => format!("{} km", x / 1000.0),
            _ => unreachable!("Invalid meter value: {}", major),
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

        current_x += maj_len * dir;
        loops += 1;
    }

    // 绘制次要刻度
    let mut minor_x = x1;
    loop {
        minor_x += min_len * dir;
        if (minor_x - x2) * dir >= 0.0 {
            // 超出范围，停止绘制
            break;
        }

        // 判断是否与主要刻度重合，重合则跳过
        if ((minor_x - x1) / maj_len).fract() < f32::EPSILON {
            continue;
        }

        // 绘制次要刻度线
        canvas.draw_line((minor_x, y), (minor_x, y - height / 2.0), &stroke_scale);
    }

    // 绘制比例尺线段
    canvas.draw_line((x1, y), (x2, y), &stroke_scale);

    Ok(())
}
