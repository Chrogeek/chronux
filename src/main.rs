use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::process::ExitCode;
use std::time::Instant;
use std::{path::PathBuf, str::FromStr};

use clap::Parser;
use indicatif::{ParallelProgressIterator, ProgressFinish, ProgressIterator, ProgressStyle};
use pariter::IteratorExt;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use skia_safe::utils::text_utils::Align;
use skia_safe::{
    AlphaType, ColorSpace, ColorType, ImageInfo, Rect, SurfaceProps, surfaces, trim_path_effect,
};

use crate::args::OutputWriter;
use crate::config::counter::add_sum_counter_item;
use crate::render::get_bounding_box;
use crate::svg::element::Shape;
use crate::util::fontmgr::get_font;
use crate::util::version::CHRONUX_VERSION;
use crate::util::{ChronuxResult, parse_css_color, timecode_rt};
use crate::widget::WIDGET_CANVAS_WIDTH;
use crate::{
    animation::Animation, args::Args, config::Config, svg::Svg, util::ChronuxError, writer::Writer,
};

mod animation;
mod args;
mod config;
mod render;
mod svg;
mod util;
mod widget;
mod writer;

fn main() -> ExitCode {
    let args = Args::parse();

    let log_level = match std::env::var("RUST_LOG") {
        Ok(level) => log::LevelFilter::from_str(&level).expect("Invalid log level"),
        Err(_) => log::LevelFilter::Info,
    };

    env_logger::builder().filter_level(log_level).init();
    log::info!("Welcome to Chronux!");
    log::info!("Version {CHRONUX_VERSION}");

    match run(args) {
        Ok(_) => {
            log::info!("Rendering completed successfully");
            ExitCode::SUCCESS
        }
        Err(e) => {
            log::error!("Rendering failed with error:");
            log::error!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<(), ChronuxError> {
    let input = args.config;
    let output = args.output;

    // 加载动画配置
    let mut conf = Config::load(&input)?;
    log::info!("Loaded config from file `{input}`");

    // 拆分所有动画组
    conf.flatten_groups()?;

    // 提取所需的SVG元素
    let ids = conf.extract_objects()?;
    log::trace!("IDs: {ids:?}");

    // 解析SVG
    let svg_file = PathBuf::from_str(&input)
        .unwrap()
        .parent()
        .unwrap()
        .join(&conf.input.svg);
    let svg = Svg::from_file(svg_file.clone())?;
    log::info!("Loaded SVG from file `{}`", conf.input.svg);

    // 读取 SVG 中的路径、圆形和文本
    let mut objs = svg.search_labels(
        &ids,
        svg_file
            .parent()
            .ok_or(ChronuxError::HrefError("incorrect href".to_string()))?
            .to_path_buf(),
        true,
    )?;
    log::info!("Extracted {} objects in SVG", objs.len());

    // 解析所有 defs
    let mut defs = svg.get_defs();

    // 解析包装SVG
    if let Some(overlay_svg) = &conf.input.overlay_svg {
        let overlay_svg_file = PathBuf::from_str(&input)
            .unwrap()
            .parent()
            .unwrap()
            .join(overlay_svg);
        let osvg = Svg::from_file(overlay_svg_file.clone())?;
        log::info!(
            "Loaded overlay SVG from file `{}`",
            overlay_svg_file.display()
        );

        // 读取 osvg 中的图形元素
        let osvg_objs = osvg.search_labels(
            &ids,
            overlay_svg_file
                .parent()
                .ok_or(ChronuxError::HrefError("incorrect href".to_string()))?
                .to_path_buf(),
            false,
        )?;

        objs.extend(osvg_objs);

        // 解析包装SVG中的 defs
        defs.extend(osvg.get_defs());
    }

    if let Some(es) = conf.drawing.extra_stroke.as_ref() {
        // 根据 es 中指定的路径组，将 objs 重排顺序，使得每个路径组的对象都在一起
        // 首先给每个对象指定一个编号：如果不在任何一个路径组中则编号等于 objs 中的下标，否则编号指定为该路径组中所有对象的最大下标

        let obj_id = objs
            .iter()
            .enumerate()
            .map(|(i, o)| (o.id.clone(), i))
            .collect::<HashMap<_, _>>();

        let obj_id = objs
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let myid = es
                    .path_groups
                    .iter()
                    .position(|g| g.contains(&o.id))
                    .map(|group_index| {
                        es.path_groups[group_index]
                            .iter()
                            .map(|item| obj_id[item])
                            .max()
                            .unwrap_or(i)
                    })
                    .unwrap_or(i);
                (o.id.clone(), myid)
            })
            .collect::<HashMap<_, _>>();

        // 按照路径组编号排序
        objs.sort_by_key(|o| obj_id[&o.id]);
    }

    let objs = objs;

    // 计算每个动画的起始时间
    conf.calc_start_times(&objs)?;
    log::info!("Calculated start times for objects");

    // 计算所有动画全部结束的时间
    let time_end = conf
        .events
        .iter()
        .fold(0.0, |acc: f32, (_, e)| acc.max(e.start + e.duration));

    let animation_start = if conf.output.time_start.is_nan() {
        0.0
    } else {
        conf.output.time_start.max(0.0)
    };
    let animation_end = if conf.output.time_end.is_nan() {
        time_end
    } else {
        conf.output.time_end.min(time_end)
    };

    let obj_map = objs
        .iter()
        .enumerate()
        .map(|(i, o)| (o.id.clone(), i))
        .collect::<HashMap<_, _>>();

    if args.encoder == OutputWriter::Timeline {
        // 只需要计算时间线而不渲染帧
        let mut events = conf
            .events
            .into_iter()
            .map(|(id, e)| {
                (
                    e.start,
                    e.duration,
                    obj_map
                        .get(&e.object)
                        .map(|index| objs.get(*index))
                        .flatten()
                        .map(|o| o.get_length())
                        .flatten(),
                    id.clone(),
                )
            })
            .collect::<Vec<_>>();
        events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut file = File::create(&output).map_err(ChronuxError::IoError)?;
        writeln!(file, "id,start,duration,length,").map_err(ChronuxError::IoError)?;
        for (start, duration, length, id) in events {
            writeln!(
                file,
                "{},{},{},{}",
                id,
                timecode_rt(start, conf.output.fps),
                duration,
                length
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| String::new()),
            )
            .map_err(ChronuxError::IoError)?;
        }
        return Ok(());
    }

    // 计算日期时间
    let dates_keyframes = conf
        .drawing
        .dates
        .as_ref()
        .map(|timeline| timeline.parse_timekeys(&conf.events))
        .transpose()?;

    // 计算镜头路径跟踪动画
    let viewport_sequence = conf.calc_viewport_tracks(&objs)?;

    // 计算每个对象的时间线
    let mut timeline = conf.calc_timeline()?;

    if args.encoder == OutputWriter::Objects {
        // 只输出对象列表
        let mut file = File::create(&output).map_err(ChronuxError::IoError)?;
        for o in timeline.keys() {
            writeln!(file, "{}", o).map_err(ChronuxError::IoError)?;
        }
        return Ok(());
    }

    // 计算线路的地上段
    let elevated = if let Some(e) = conf.drawing.elevated.as_ref() {
        Some(e.parse_points(&objs)?)
    } else {
        None
    };

    // 设置每个对象时间线的起始属性，如有出现(appear)动画则将起始状态置为默认值（未开始）
    for o in objs.iter() {
        if let Some(tl) = timeline.get_mut(&o.id) {
            tl.start = o.style.clone();
            for e in tl.events.iter() {
                match e.animation {
                    Animation::TrimPath { .. } => {
                        tl.start.trim = (0.0, 0.0);
                        break;
                    }
                    Animation::Typewriter { .. } => {
                        tl.start.typewriter = (0.0, 0.0);
                        break;
                    }
                    Animation::Sweep => {
                        tl.start.sweep = (0.0, 0.0);
                        break;
                    }
                    Animation::FadeIn => {
                        tl.start.opacity = 0.0;
                        break;
                    }
                    Animation::StretchIn => {
                        tl.start.scale_x = 0.0;
                        tl.start.scale_y = 0.0;
                        break;
                    }
                    // 下面这些不是出现动画，什么都不用做
                    Animation::TypewriterOut { .. }
                    | Animation::SweepOut
                    | Animation::Tween(_)
                    | Animation::FadeOut
                    | Animation::TrimPathOut { .. }
                    | Animation::Move { .. } => {}
                }
            }
        }
    }
    log::info!("Calculated timeline for objects");

    // 渲染动画
    let frame_start = (animation_start * conf.output.fps as f32) as usize;
    let frame_end = (animation_end * conf.output.fps as f32) as usize;
    log::info!("Frames to render: {frame_start} - {frame_end}");

    let max_frames = (time_end * conf.output.fps as f32) as usize;

    let writer = match args.encoder {
        OutputWriter::Ffmpeg => Writer::new_ffmpeg(
            &output,
            conf.output.width as usize,
            conf.output.height as usize,
            conf.output.fps as usize,
        )?,
        OutputWriter::PngSequence => Writer::new_png_sequence(
            &output,
            conf.output.width as usize,
            conf.output.height as usize,
            max_frames,
        )?,
        OutputWriter::Timeline | OutputWriter::Objects => {
            unreachable!("Timeline / Objects writer should not be used here")
        }
    };

    let pstyle = ProgressStyle::with_template(&format!(
        "{{percent:>3}}% {{wide_bar}} {{pos:>{}}}/{{len}} [{{elapsed_precise}}<{{eta_precise}}]",
        (frame_end - frame_start).to_string().len()
    ))
    .unwrap();

    log::info!("Calculating frame bounds...");

    let bounds = if let Some(minimap) = conf.drawing.minimap.as_ref() {
        Some({
            let bounds = (frame_start..frame_end)
                .progress_with_style(pstyle.clone())
                .map(|f| {
                    let t = f as f32 / conf.output.fps as f32;
                    // 计算所有需要显示在小地图中的对象的边界框
                    let mut bounding_box: Option<(f32, f32, f32, f32)> = None;
                    for o in objs.iter() {
                        if minimap.paths.contains(&o.id) {
                            let properties = match timeline.get(&o.id) {
                                Some(tl) => tl.interpolate(t),
                                None => {
                                    return Err(ChronuxError::ArgError(format!(
                                        "Object '{}' in minimap.paths not in timeline",
                                        o.id
                                    )));
                                }
                            };
                            let ret_bb = get_bounding_box(&o.shape, &properties);
                            if let Some((bl, bt, br, bb)) = bounding_box
                                && let Some(ret_bb) = ret_bb
                            {
                                bounding_box = Some((
                                    bl.min(ret_bb.0),
                                    bt.min(ret_bb.1),
                                    br.max(ret_bb.2),
                                    bb.max(ret_bb.3),
                                ));
                            } else if bounding_box.is_none() {
                                bounding_box = ret_bb;
                            }
                        }
                    }
                    Ok(bounding_box)
                })
                .collect::<ChronuxResult<Vec<_>>>()?;
            let mut new_bounds = vec![];
            for i in 0..bounds.len() {
                if i == 0 {
                    new_bounds.push(bounds[i]);
                } else {
                    match (new_bounds[i - 1], bounds[i]) {
                        (None, _) | (_, None) => new_bounds.push(bounds[i]),
                        (Some((bl1, bt1, br1, bb1)), Some((bl2, bt2, br2, bb2))) => {
                            let max_delta = minimap.expand_speed / conf.output.fps as f32;
                            new_bounds.push(Some((
                                limit_frame_bound(bl1, bl2, max_delta),
                                limit_frame_bound(bt1, bt2, max_delta),
                                limit_frame_bound(br1, br2, max_delta),
                                limit_frame_bound(bb1, bb2, max_delta),
                            )));
                        }
                    }
                }
            }
            new_bounds
        })
    } else {
        None
    };

    log::info!("Start rendering...");

    let render_routine = move |f| {
        // 获取当前时间戳
        let render_start = Instant::now();

        // 当前时间点（秒数）
        let t = f as f32 / conf.output.fps as f32;

        // 计算计数器
        let mut counters = conf.get_counters(t);

        // 添加求和计数器
        for (sum_counter, sub_counters) in &conf.drawing.sum_counters {
            add_sum_counter_item(&mut counters, sum_counter, sub_counters);
        }

        // 计算当前镜头位置
        let vb = viewport_sequence.interpolate_vp(t)?;

        let (cx, cy, vw, vh) = (
            vb.0,
            vb.1,
            vb.2,
            vb.2 * conf.output.height as f32 / conf.output.width as f32,
        );
        let (vx, vy) = (cx - vw / 2.0, cy - vh / 2.0);
        let viewport_rect = Rect::new(vx, vy, vx + vw, vy + vh);

        // 创建 Skia 画布
        let ii = ImageInfo::new(
            (conf.output.width as i32, conf.output.height as i32),
            ColorType::RGBA8888,
            AlphaType::Premul,
            ColorSpace::new_srgb(),
        );
        let mut surface = surfaces::raster(&ii, 0_usize, Some(&SurfaceProps::default())).ok_or(
            ChronuxError::SkiaError("failed to create Skia surface".into()),
        )?;
        surface.canvas().clear(conf.output.background_color); // 设置背景颜色

        let (sx, sy) = (
            conf.output.width as f32 / vw,
            conf.output.height as f32 / vh,
        );

        // 应用镜头变换
        surface.canvas().scale((sx, sy)).translate((-vx, -vy));

        // 为 extra-stroke 准备数据结构
        let mut es = if let Some(es) = &conf.drawing.extra_stroke {
            let mut counts = es
                .path_groups
                .iter()
                .map(|g| (HashSet::<String>::from_iter(g.iter().cloned()), g.len()))
                .collect::<Vec<_>>();

            // 有些路径需要绘制 extra-stroke，但没有出现在路径组中，它们自成一组，需要由此扩充 es
            for path in es.paths.iter() {
                if !counts.iter().any(|(paths, _)| paths.contains(path)) {
                    counts.push((HashSet::from([path.clone()]), 1));
                }
            }

            Some(counts)
        } else {
            None
        };

        // 固定对象直接绘制
        for o in objs.iter() {
            if o.apply_transform && conf.input.pinned.contains(&o.id) {
                let properties = o.style.clone();
                render::draw(
                    surface.canvas(),
                    &o.shape,
                    &properties,
                    0.0,
                    &counters,
                    &viewport_rect,
                    &conf.drawing.extra_stroke,
                    &defs,
                    o.transform,
                )?;
            }
        }

        // 绘制地上段（elevated）路径
        if let Some(elevated) = &elevated {
            let elevated_config = conf.drawing.elevated.as_ref().unwrap();
            for o in objs.iter() {
                let path = &o.id;
                if let Some(intervals) = elevated.get(path) {
                    let o = &objs[obj_map[path]];
                    let d = match &o.shape {
                        Shape::Path { d } => d,
                        _ => {
                            return Err(ChronuxError::ArgError(format!(
                                "Invalid shape for elevated point: {}, must be a path",
                                o.id
                            )));
                        }
                    };

                    let path_props = timeline[path].interpolate(t);
                    let (ps, pe) = path_props.trim;

                    if path_props.stroke_width < 1e-3 || path_props.opacity < 1e-3 || pe - ps < 1e-3
                    {
                        // 没有描边/不可见就不需要绘制地上段
                        continue;
                    }

                    let mut paint_line = path_props.get_stroke_paint(&defs, o.transform);
                    paint_line.set_stroke_width(
                        path_props.stroke_width
                            + (elevated_config.gap + elevated_config.stroke_width) * 2.0,
                    );

                    for (is, ie) in intervals {
                        let start = ps.max(*is);
                        let end = pe.min(*ie);

                        if end - start > 1e-3 {
                            let effect =
                                trim_path_effect::new(start, end, trim_path_effect::Mode::Normal);

                            let mut paint_line = paint_line.clone();
                            paint_line.set_path_effect(effect);
                            surface.canvas().draw_path(d, &paint_line);
                        }
                    }
                }
            }

            for o in objs.iter() {
                let path = &o.id;
                if let Some(intervals) = elevated.get(path) {
                    let d = match &o.shape {
                        Shape::Path { d } => d,
                        _ => {
                            return Err(ChronuxError::ArgError(format!(
                                "Invalid shape for elevated point: {}, must be a path",
                                o.id
                            )));
                        }
                    };

                    let path_props = timeline[path].interpolate(t);
                    let (ps, pe) = path_props.trim;

                    if path_props.stroke_width < 1e-3 || path_props.opacity < 1e-3 || pe - ps < 1e-3
                    {
                        // 没有描边/不可见就不需要绘制地上段
                        continue;
                    }

                    let mut paint_gap = path_props.get_stroke_paint(&defs, o.transform);

                    paint_gap
                        .set_color(parse_css_color(
                            &elevated_config.gap_color,
                            path_props.opacity,
                        )?)
                        .set_stroke_width(path_props.stroke_width + elevated_config.gap * 2.0);

                    for (is, ie) in intervals {
                        let start = ps.max(*is);
                        let end = pe.min(*ie);

                        if end - start > 1e-3 {
                            let effect =
                                trim_path_effect::new(start, end, trim_path_effect::Mode::Normal);

                            let mut paint_gap = paint_gap.clone();
                            paint_gap.set_path_effect(effect.clone());
                            surface.canvas().draw_path(d, &paint_gap);
                        }
                    }
                }
            }
        }

        // 绘制每个图形（动态部分，要求 apply_transform = true）
        for o in objs.iter() {
            if conf.drawing.phantom.contains(&o.id) {
                // 忽略 phantom 对象
                continue;
            } else if !o.apply_transform {
                continue;
            } else if let Some(tl) = timeline.get(&o.id) {
                let properties = tl.interpolate(t);
                render::draw(
                    surface.canvas(),
                    &o.shape,
                    &properties,
                    0.0,
                    &counters,
                    &viewport_rect,
                    &conf.drawing.extra_stroke,
                    &defs,
                    o.transform,
                )?;
            }
            if let Some(extra_stroke) = conf.drawing.extra_stroke.as_ref() {
                for (paths, count) in es.as_mut().unwrap() {
                    if paths.contains(&o.id) {
                        *count -= 1;
                        if *count == 0 {
                            for path in paths.iter() {
                                let properties = timeline[path].interpolate(t);
                                let mut extra_paint =
                                    properties.get_stroke_paint(&defs, o.transform);
                                extra_paint
                                    .set_color(parse_css_color(&extra_stroke.stroke_color, 1.0)?)
                                    .set_stroke_width(extra_stroke.stroke_width);
                                if let Some(e) = properties.get_effects() {
                                    extra_paint.set_path_effect(e);
                                }
                                match &objs[obj_map[path]].shape {
                                    Shape::Path { d } => {
                                        surface.canvas().draw_path(d, &extra_paint);
                                    }
                                    _ => unreachable!("extra stroke only supports paths"),
                                }
                            }
                        }
                    }
                }
            }
        }

        let canvas_width = WIDGET_CANVAS_WIDTH;
        let canvas_height =
            WIDGET_CANVAS_WIDTH * conf.output.height as f32 / conf.output.width as f32;

        // 绘制包装前，需要重置 viewbox 变换，然后按照 1920x1080 大小重新缩放
        surface.canvas().reset_matrix().scale((
            conf.output.width as f32 / canvas_width,
            conf.output.height as f32 / canvas_height,
        ));

        // 绘制包装部分的图形元素（要求 apply_transform = false）
        // TODO: 目前要求包装 svg 的画幅大小为 1920x1080，暂不支持其他比例（会导致位置错乱）
        for o in objs.iter() {
            if !o.apply_transform && conf.input.pinned.contains(&o.id) {
                let properties = o.style.clone();
                render::draw(
                    surface.canvas(),
                    &o.shape,
                    &properties,
                    0.0,
                    &counters,
                    &viewport_rect,
                    &conf.drawing.extra_stroke,
                    &defs,
                    o.transform,
                )?;
            }
        }

        for o in objs.iter() {
            if conf.drawing.phantom.contains(&o.id) {
                // 忽略 phantom 对象
                continue;
            } else if o.apply_transform {
                continue;
            } else if let Some(tl) = timeline.get(&o.id) {
                let properties = tl.interpolate(t);
                render::draw(
                    surface.canvas(),
                    &o.shape,
                    &properties,
                    0.0,
                    &counters,
                    &viewport_rect,
                    &conf.drawing.extra_stroke,
                    &defs,
                    o.transform,
                )?;
            }
        }

        // 绘制小地图
        if let Some(minimap) = conf.drawing.minimap.as_ref() {
            let minimap_props = timeline[&minimap.position].interpolate(t);
            let (x, y) = objs[obj_map[&minimap.position]].get_position(&minimap_props);
            widget::minimap::draw(
                surface.canvas(),
                minimap,
                &timeline,
                &objs,
                t,
                &counters,
                &defs,
                x,
                y,
                bounds.as_ref().unwrap()[f - frame_start],
            )?;
        }

        // 绘制比例尺
        if let Some(scale) = conf.drawing.scale.as_ref() {
            let scale_props = timeline[&scale.position].interpolate(t);
            let (x, y) = objs[obj_map[&scale.position]].get_position(&scale_props);
            widget::scale::draw(surface.canvas(), scale, vw, &conf.drawing.ascii_font, x, y)?;
        }

        // 绘制日期时间
        if let Some(dates_config) = conf.drawing.dates.as_ref()
            && let Some(dates_keyframes) = dates_keyframes.as_ref()
        {
            let position_year_props = timeline[&dates_config.position_year].interpolate(t);
            let position_date_props = timeline[&dates_config.position_date].interpolate(t);
            let area_year_props = timeline[&dates_config.area_year].interpolate(t);
            let area_date_props = timeline[&dates_config.area_date].interpolate(t);

            let position_year =
                objs[obj_map[&dates_config.position_year]].get_position(&position_year_props);
            let position_date =
                objs[obj_map[&dates_config.position_date]].get_position(&position_date_props);
            let area_year = objs[obj_map[&dates_config.area_year]].get_bounds(&area_year_props);
            let area_date = objs[obj_map[&dates_config.area_date]].get_bounds(&area_date_props);

            widget::dates::draw(
                surface.canvas(),
                dates_config,
                dates_keyframes,
                t,
                position_date,
                position_year,
                &area_date,
                &area_year,
                &conf.drawing.ascii_font,
            )?;
        }

        // 调试模式下输出时间码
        if conf.drawing.debug_timecode {
            surface.canvas().reset_matrix();
            let timecode = timecode_rt(t, conf.output.fps);
            let mut paint = skia_safe::Paint::default();
            paint.set_anti_alias(true);
            paint.set_color(skia_safe::Color::BLACK);

            let sc = conf.output.width as f32 / 1920.0;
            let font = get_font(&conf.drawing.ascii_font, 20.0 * sc, 400)?;
            let (x, y) = (1900.0 * sc, 960.0 * sc);
            surface
                .canvas()
                .draw_text_align(&timecode, (x, y), &font, &paint, Align::Right);
        }

        // 输出图片
        let pixmap = surface.peek_pixels().ok_or(ChronuxError::SkiaError(
            "failed to peek pixels from Skia surface".into(),
        ))?;

        let raw = pixmap
            .bytes()
            .ok_or(ChronuxError::SkiaError(
                "failed to get bytes from Skia pixmap".into(),
            ))?
            .to_vec();

        let render_end = Instant::now();
        let render_time_us = render_end.duration_since(render_start).as_micros() as f32;

        Ok((f, raw, render_time_us))
    };

    let write_routine = |indexed_raw| match indexed_raw {
        Ok((f, raw, frame_time)) => {
            writer.write_frame(f, raw)?;
            Ok(frame_time)
        }
        Err(e) => Err(e),
    };

    let render_err_fold = |acc, res| match (acc, res) {
        (Ok(a), Ok(b)) => Ok(a + b),
        (Err(e), _) | (_, Err(e)) => Err(e),
    };

    let render_result = match writer {
        Writer::PngSequence { .. } => (frame_start..frame_end)
            .into_par_iter()
            .progress_with_style(pstyle)
            .with_finish(ProgressFinish::AndLeave)
            .map(render_routine)
            .map(write_routine)
            .reduce(|| Ok(0.0), render_err_fold),
        Writer::Ffmpeg { .. } => (frame_start..frame_end)
            .progress_with_style(pstyle)
            .with_finish(ProgressFinish::AndLeave)
            .parallel_map(render_routine)
            .map(write_routine)
            .fold(Ok(0.0), render_err_fold),
    };

    writer.finish()?;

    match render_result {
        Ok(total_time) => {
            log::info!(
                "Average frame time: {:.2} ms",
                total_time / (frame_end - frame_start) as f32 / 1000.0
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn limit_frame_bound(x: f32, y: f32, max_delta: f32) -> f32 {
    if x < y {
        y.min(x + max_delta)
    } else {
        y.max(x - max_delta)
    }
}
