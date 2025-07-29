use std::fs::File;
use std::io::Write;
use std::process::ExitCode;
use std::{path::PathBuf, str::FromStr};

use clap::Parser;
use indicatif::{ParallelProgressIterator, ProgressFinish, ProgressIterator, ProgressStyle};
use pariter::IteratorExt;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use skia_safe::utils::text_utils::Align;
use skia_safe::{AlphaType, ColorSpace, ColorType, ImageInfo, SurfaceProps, surfaces};

use crate::util::version::CHRONUX_VERSION;
use crate::util::{get_font, timecode_rt};
use crate::widget::{WIDGET_CANVAS_HEIGHT, WIDGET_CANVAS_WIDTH};
use crate::{
    animation::Animation,
    args::Args,
    config::Config,
    render::viewport::{
        calc_viewport_keyframe_bezier, calc_viewport_times, interpolate_viewport, sort_viewports,
    },
    svg::Svg,
    util::{ChronuxError, ChronuxResult, vector::Vec3},
    writer::Writer,
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

    env_logger::init();
    log::info!("Welcome to Chronux!");
    log::info!("Version {}", CHRONUX_VERSION);

    match run(args) {
        Ok(_) => {
            log::info!("Rendering completed successfully");
            ExitCode::SUCCESS
        }
        Err(e) => {
            log::error!("Rendering failed with error:");
            log::error!("{}", e);
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<(), ChronuxError> {
    let input = args.config;
    let output = args.output;

    // 加载动画配置
    let mut conf = Config::load(&input)?;
    log::info!("Loaded config from file `{}`", input);

    // 拆分所有动画组
    conf.flatten_groups()?;

    // 提取所需的SVG元素
    let ids = conf.extract_objects()?;
    log::trace!("IDs: {:?}", ids);

    // 解析SVG
    let svg_file = PathBuf::from_str(&input)
        .unwrap()
        .parent()
        .unwrap()
        .join(&conf.input.svg);
    let svg = Svg::from_file(svg_file.clone())?;
    log::info!("Loaded SVG from file `{}`", conf.input.svg);

    // 读取 SVG 中的路径、圆形和文本
    let mut ids = ids;
    ids.extend(conf.input.pinned.iter().cloned());
    let objs = svg.search_labels(
        &ids,
        svg_file
            .parent()
            .ok_or(ChronuxError::HrefError("incorrect href".to_string()))?
            .to_path_buf(),
    )?;
    log::info!("Extracted {} objects in SVG", objs.len());

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
        conf.output.time_start
    };
    let animation_end = if conf.output.time_end.is_nan() {
        time_end
    } else {
        conf.output.time_end
    };

    if args.timeline_only {
        // 只需要计算时间线而不渲染帧
        let mut filtered_events = conf
            .events
            .values()
            .into_iter()
            .filter(|e| e.timeline_marked)
            .collect::<Vec<_>>();
        filtered_events.sort_by(|a, b| {
            a.start
                .partial_cmp(&b.start)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut file = File::create(&output).map_err(|e| ChronuxError::IoError(e))?;
        writeln!(file, "| ID | Start Time | End Time | Duration |")
            .map_err(|e| ChronuxError::IoError(e))?;
        writeln!(file, "| :-- | --: | --: | --: |").map_err(|e| ChronuxError::IoError(e))?;
        for e in filtered_events {
            writeln!(
                file,
                "| {} | {} | {} | {} |",
                e.id,
                timecode_rt(e.start, conf.output.fps),
                timecode_rt(e.start + e.duration, conf.output.fps),
                timecode_rt(e.duration, conf.output.fps)
            )
            .map_err(|e| ChronuxError::IoError(e))?;
        }
        return Ok(());
    }

    let mut timeline = conf.calc_timeline()?;

    // 设置每个对象时间线的起始属性，如有出现(appear)动画则将起始状态置为默认值（未开始）
    for o in objs.iter() {
        if let Some(tl) = timeline.get_mut(&o.id) {
            tl.start = o.style.clone();
            for e in tl.events.iter() {
                match e.animation {
                    Animation::TrimPath { .. } => {
                        tl.start.trim = (0.0, 0.0);
                    }
                    Animation::Typewriter => {
                        tl.start.typewriter = (0.0, 0.0);
                    }
                    Animation::Sweep => {
                        tl.start.sweep = 0.0;
                    }
                    Animation::FadeIn => {
                        tl.start.opacity = 0.0;
                    }
                    // 下面这些不是出现动画，什么都不用做
                    Animation::TypewriterOut
                    | Animation::Tween(_)
                    | Animation::FadeOut
                    | Animation::TrimPathOut { .. } => {}
                }
            }
        }
    }
    log::info!("Calculated timeline for objects");

    // 处理镜头
    calc_viewport_times(&mut conf.viewports, &conf.events)?;
    sort_viewports(&mut conf.viewports);

    calc_viewport_keyframe_bezier(&mut conf.viewports);
    log::info!("Calculated viewport keyframes");

    // 渲染动画
    let frame_start = (animation_start * conf.output.fps as f32) as usize;
    let frame_end = (animation_end * conf.output.fps as f32) as usize;
    log::info!("Frames rendered: {} - {}", frame_start, frame_end);

    log::info!("Start rendering frames...");

    let writer = match args.encoder {
        args::OutputWriter::Ffmpeg => Writer::new_ffmpeg(
            &output,
            conf.output.width as usize,
            conf.output.height as usize,
            conf.output.fps as usize,
        )?,
        args::OutputWriter::PngSequence => Writer::new_png_sequence(
            &output,
            conf.output.width as usize,
            conf.output.height as usize,
        )?,
    };

    let pstyle = ProgressStyle::with_template(&format!(
        "{{percent:>3}}% {{wide_bar}} {{pos:>{}}}/{{len}} [{{elapsed_precise}}<{{eta_precise}}]",
        (frame_end - frame_start).to_string().len()
    ))
    .unwrap();

    let render_routine = move |f| {
        // 当前时间点（秒数）
        let t = f as f32 / conf.output.fps as f32;

        // 计算当前镜头位置
        let vb = interpolate_viewport(&conf.viewports, t).unwrap_or(Vec3(
            0.0,
            0.0,
            conf.output.width as f32,
        ));
        let (cx, cy, vw, vh) = (
            vb.0,
            vb.1,
            vb.2,
            vb.2 * conf.output.height as f32 / conf.output.width as f32,
        );
        let (vx, vy) = (cx - vw / 2.0, cy - vh / 2.0);

        // 创建 Skia 画布
        let ii = ImageInfo::new(
            (conf.output.width as i32, conf.output.height as i32),
            ColorType::RGBA8888,
            AlphaType::Premul,
            ColorSpace::new_srgb(),
        );
        let mut surface = surfaces::raster(&ii, (0) as usize, Some(&SurfaceProps::default()))
            .ok_or(ChronuxError::SkiaError(
                "failed to create Skia surface".into(),
            ))?;
        surface.canvas().clear(conf.output.background_color); // 设置背景颜色

        let (sx, sy) = (
            conf.output.width as f32 / vw,
            conf.output.height as f32 / vh,
        );

        // 应用镜头变换
        surface.canvas().scale((sx, sy)).translate((-vx, -vy));

        // 绘制每个图形
        for o in objs.iter() {
            let properties = match timeline.get(&o.id) {
                Some(tl) => tl.interpolate(t),
                None => o.style.clone(),
            };
            render::draw(&surface.canvas(), &o.shape, &properties)?;
        }

        // 绘制包装前，需要重置 viewbox 变换，然后按照 1920x1080 大小重新缩放
        surface.canvas().reset_matrix().scale((
            conf.output.width as f32 / WIDGET_CANVAS_WIDTH,
            conf.output.height as f32 / WIDGET_CANVAS_HEIGHT,
        ));

        // 绘制小地图
        if let Some(minimap) = conf.drawing.minimap.as_ref() {
            widget::minimap::draw(&surface.canvas(), minimap, &timeline, &objs, t)?;
        }

        // 绘制比例尺
        if let Some(scale) = conf.drawing.scale.as_ref() {
            widget::scale::draw(&surface.canvas(), scale, vw, &conf.drawing.ascii_font)?;
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

        Ok((f, raw))
    };

    let write_routine = |indexed_raw| {
        let res = match indexed_raw {
            Ok((f, raw)) => {
                writer.write_frame(f, raw)?;
                Ok(())
            }
            Err(e) => Err(e),
        };
        res
    };

    let render_err = match writer {
        Writer::PngSequence { .. } => (frame_start..frame_end)
            .into_par_iter()
            .progress_with_style(pstyle)
            .with_finish(ProgressFinish::AndLeave)
            .map(render_routine)
            .map(write_routine)
            .find_any(ChronuxResult::is_err),
        Writer::Ffmpeg { .. } => (frame_start..frame_end)
            .progress_with_style(pstyle)
            .with_finish(ProgressFinish::AndLeave)
            .parallel_map(render_routine)
            .map(write_routine)
            .find(ChronuxResult::is_err),
    };

    writer.finish()?;

    if let Some(ChronuxResult::Err(e)) = render_err {
        Err(e)
    } else {
        Ok(())
    }
}
