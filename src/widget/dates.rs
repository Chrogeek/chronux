use skia_safe::{Canvas, ClipOp, Font, Paint, Rect, utils::text_utils::Align};

use crate::{
    config::dates::{DatesAnimation, DatesConfig},
    util::{ChronuxResult, fontmgr::get_font, parse_css_color},
};

const DURATION_IN_OUT_SCALE: f32 = 2.0;
const DURATION_SHORT: f32 = 0.5;
const DURATION_LONG: f32 = 1.0;
const DURATION_OFFSET_UNIT: f32 = 5.0 / 60.0;

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
    for (time, year, date) in keyframes.iter() {
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

    match config.animation {
        DatesAnimation::Rolling => {
            // 5. 绘制dates

            // 5.1. 设置裁剪区域
            canvas.clip_rect(clip_date, ClipOp::Intersect, true);

            // 5.2. 动画插值
            let date_duration =
                if start.clone().map(|s| s.2).is_none() || end.clone().map(|e| e.2).is_none() {
                    DURATION_SHORT * DURATION_IN_OUT_SCALE
                } else {
                    DURATION_SHORT
                };

            let t_dates = if let Some((end_time, _, _)) = end {
                (1.0 - (end_time - t) / date_duration).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let t_dates = config.easing.evaluate(t_dates, date_duration);

            let y_start = date_y - t_dates * config.line_height * config.font_size_date;
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
            let date_start = start.as_ref().map(|(_, _, date)| date);
            let date_end = end.as_ref().map(|(_, _, date)| date);

            if date_start != date_end {
                if let Some(date_start) = &date_start {
                    draw_centered_text(canvas, date_start, (date_x, y_start), &font, &paint);
                }
                if let Some(date_end) = &date_end {
                    draw_centered_text(canvas, date_end, (date_x, y_end), &font, &paint);
                }
            } else if let Some(date_start) = &date_start {
                draw_centered_text(canvas, date_start, (date_x, date_y), &font, &paint);
            }

            canvas.restore_to_count(clip_token); // 恢复裁剪区

            // 6. 绘制年份

            // 6.1. 设置裁剪区域
            canvas.clip_rect(clip_year, ClipOp::Intersect, true);

            let font = get_font(
                &config
                    .font_family
                    .clone()
                    .unwrap_or(fallback_font_family.to_string()),
                config.font_size_year,
                config.font_weight_year as i32,
            )?;

            let digit_spacing = font.measure_str("8", None).0;

            // 6.2. 分解年份四位数
            let year_start = start
                .as_ref()
                .map(|(_, year, _)| format!("{year:>4}"))
                .unwrap_or("    ".to_string())
                .chars()
                .collect::<Vec<_>>();
            let year_end = end
                .as_ref()
                .map(|(_, year, _)| format!("{year:>4}"))
                .unwrap_or("    ".to_string())
                .chars()
                .collect::<Vec<_>>();
            assert_eq!(year_start.len(), 4);
            assert_eq!(year_end.len(), 4);

            let is_start_or_end = year_start == [' ', ' ', ' ', ' '] || year_end == [' ', ' ', ' ', ' '];

            // 如果 start.is_none 则从左至右依次演示，如果 end.is_none 或者均为 some 则从右至左依次演示

            let mut delay = DURATION_OFFSET_UNIT * 3.0;
            let mut start_offset = [0.0; 4];
            let mut dur = [0.0; 4];
            let mut duration = 0.0f32;

            if start.clone().map(|s| s.1).is_none() {
                for i in 0..4 {
                    let c1 = year_start[i];
                    let c2 = year_end[i];
                    if c1 != c2 {
                        start_offset[i] = if i == 0 {
                            0.0
                        } else {
                            start_offset[i - 1] + delay
                        };
                        delay -= DURATION_OFFSET_UNIT;
                    } else if i > 0 {
                        start_offset[i] = start_offset[i - 1];
                    }
                }
            } else {
                for i in (0..4).rev() {
                    let c1 = year_start[i];
                    let c2 = year_end[i];
                    if c1 != c2 {
                        start_offset[i] = if i == 3 {
                            0.0
                        } else {
                            start_offset[i + 1] + delay
                        };
                        delay -= DURATION_OFFSET_UNIT;
                    } else if i < 3 {
                        start_offset[i] = start_offset[i + 1];
                    }
                }
            }

            for i in 0..4 {
                let c1 = year_start[i];
                let c2 = year_end[i];
                dur[i] = if c1 != c2 {
                    if is_start_or_end {
                        DURATION_LONG
                    } else if !c1.is_ascii_digit()
                        || !c2.is_ascii_digit()
                        || (10 + u32::from(c2) - u32::from(c1)) % 10 == 1
                    {
                        DURATION_SHORT
                    } else {
                        DURATION_LONG
                    }
                } else {
                    0.0
                };
                duration = duration.max(start_offset[i] + dur[i]);
            }

            // 依次绘制年份的每一位
            for i in 0..4 {
                let c1 = year_start[i];
                let c2 = year_end[i];
                let year_x = year_x + (i as f32 - 1.5) * digit_spacing;
                if c1 != c2 {
                    // 6.3. 动画插值
                    let t_years = if let Some((end_time, _, _)) = end {
                        ((t - (end_time - duration + start_offset[i])) / dur[i]).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };

                    let t_years = config.easing.evaluate(t_years, dur[i]);

                    // 6.4. 绘制年份
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_color(parse_css_color(&config.fill_color, 1.0)?);
                    let font = get_font(
                        &config
                            .font_family
                            .clone()
                            .unwrap_or(fallback_font_family.to_string()),
                        config.font_size_year,
                        config.font_weight_year as i32,
                    )?;

                    if c1.is_ascii_digit() && c2.is_ascii_digit() {
                        // 绘制数字
                        let c1 = c1.to_digit(10).unwrap();
                        let c2 = c2.to_digit(10).unwrap();
                        let c2 = if c2 < c1 { c2 + 10 } else { c2 };

                        let y_start = year_y
                            - t_years
                                * config.line_height
                                * config.font_size_year
                                * (c2 - c1) as f32;
                        for c in c1..=c2 {
                            let y = y_start
                                + (c - c1) as f32 * config.line_height * config.font_size_year;
                            draw_centered_text(
                                canvas,
                                &(c % 10).to_string(),
                                (year_x, y),
                                &font,
                                &paint,
                            );
                        }
                    } else {
                        let y_start = year_y - t_years * config.line_height * config.font_size_year;
                        let y_end = y_start + config.line_height * config.font_size_year;
                        draw_centered_text(
                            canvas,
                            &c1.to_string(),
                            (year_x, y_start),
                            &font,
                            &paint,
                        );
                        draw_centered_text(canvas, &c2.to_string(), (year_x, y_end), &font, &paint);
                    }
                } else {
                    draw_centered_text(canvas, &c1.to_string(), (year_x, year_y), &font, &paint);
                }
            }

            canvas.restore_to_count(clip_token); // 恢复裁剪区
        }
    }

    Ok(())
}

fn draw_centered_text(canvas: &Canvas, text: &str, p: (f32, f32), font: &Font, paint: &Paint) {
    let (_, rect) = font.measure_str(text, Some(paint));
    canvas.draw_text_align(
        text,
        (p.0, p.1 - (rect.bottom + rect.top) * 0.5),
        font,
        paint,
        Align::Center,
    );
}
