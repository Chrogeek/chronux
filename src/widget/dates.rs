use skia_safe::{region::Cliperator, utils::text_utils::Align, Canvas, ClipOp, Font, FontMetrics, Paint, Rect};

use crate::{
    config::dates::DatesConfig,
    util::{ChronuxResult, fontmgr::get_font, parse_css_color},
};

const DURATION_IN_OUT: f32 = 2.0;
const DURATION_SHORT: f32 = 0.5;
const DURATION_LONG: f32 = 1.0;

pub fn draw(
    canvas: &Canvas,
    config: &DatesConfig,
    keyframes: &[(f32, String, String)],
    t: f32,
    position_date: (f32, f32),
    position_year: (f32, f32),
    clip_date: &Rect,
    clip_year: &Rect,
    fallback_font_family: &str,
) -> ChronuxResult<()> {
    // 1. 判断 t 处于哪两个 keyframe 之间 (keyframes已按时间升序排序)
    let mut start = None;
    let mut end = None;
    for (i, (time, year, date)) in keyframes.iter().enumerate() {
        if *time <= t {
            start = Some((*time, year.clone(), date.clone()));
        } else {
            end = Some((*time, year.clone(), date.clone()));
            break;
        }
    }

    // 2. 如果 start=end=None，则什么都不绘制
    if start.is_none() && end.is_none() {
        return Ok(());
    }

    // 3. 解包位置参数
    let (date_x, date_y) = position_date;
    let (year_x, year_y) = position_year;

    // 4. 保存裁剪区
    let clip_token = canvas.save();

    // 5. 绘制dates

    // 5.1. 设置裁剪区域
    canvas.clip_rect(clip_date, ClipOp::Intersect, true);

    // 5.2. 动画插值
    let t_dates = if let Some((end_time, _, _)) = end {
        (1.0 - (end_time - t) / DURATION_LONG).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let t_dates = config.easing.evaluate(t_dates, DURATION_LONG);

    let y_start =
        date_y - t_dates * config.line_height * config.font_size_date;
    let y_end = y_start + config.line_height * config.font_size_date;

    // 5.3. 绘制日期
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(parse_css_color(&config.fill_color, 1.0)?);
    let font = get_font(
        &config
            .font_family
            .clone()
            .unwrap_or(fallback_font_family.to_string()),
        config.font_size_date,
        config.font_weight_date as i32,
    )?;
    if let Some((_, _, date_start)) = &start {
        draw_centered_text(canvas, date_start, (date_x, y_start), &font, &paint);
    }
    if let Some((_, _, date_end)) = &end {
        draw_centered_text(canvas, date_end, (date_x, y_end), &font, &paint);
    }

    canvas.restore_to_count(clip_token); // 恢复裁剪区

    // 6. 绘制年份

    // 6.1. 设置裁剪区域(TODO)

    // 6.2. 分解年份四位数

    Ok(())
}

fn draw_centered_text(
    canvas: &Canvas,
    text: &str,
    p: (f32, f32),
    font: &Font,
    paint: &Paint,
) {
    let (_, rect) = font.measure_str(text, Some(paint));
    canvas.draw_text_align(text, (p.0, p.1 - (rect.bottom + rect.top) * 0.5), &font, &paint, Align::Center);
}
