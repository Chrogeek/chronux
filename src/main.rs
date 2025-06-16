use std::{path::PathBuf, str::FromStr};

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use skia_safe::surfaces;

use crate::{
    animation::Animation,
    args::Args,
    config::Config,
    render::viewport::{
        calc_viewport_keyframe_bezier, calc_viewport_times, interpolate_viewport, sort_viewports,
    },
    svg::Svg,
    util::{ChronuxError, timecode, vector::Vec3},
};

mod animation;
mod args;
mod config;
mod render;
mod svg;
mod util;

fn main() -> Result<(), ChronuxError> {
    let args = Args::parse();

    env_logger::init();
    log::info!("Welcome to Chronux!");

    run(args).or_else(|e| {
        log::error!("{:?}", e);
        Err(e)
    })
}

fn run(args: Args) -> Result<(), ChronuxError> {
    let input = args.config;
    let output_dir = args.output_dir;

    // 加载动画配置
    let mut conf = Config::load(&input)?;
    log::info!("Loaded config from file `{}`", input);

    // 拆分所有动画组
    conf.flatten_groups()?;

    // 提取所需的SVG元素
    let ids = conf.extract_objects()?;
    log::trace!("IDs: {:?}", ids);

    // 解析SVG
    let svg = Svg::from_file(&conf.input.svg)?;
    log::info!("Loaded SVG from file `{}`", conf.input.svg);

    // 读取 SVG 中的路径、圆形和文本
    let mut ids = ids;
    ids.extend(conf.input.pinned.iter().cloned());
    let objs = svg.search_labels(
        &ids,
        PathBuf::from_str(&conf.input.svg.clone())
            .unwrap()
            .parent()
            .ok_or(ChronuxError::HrefError("incorrect href".to_string()))?
            .to_path_buf(),
    )?;
    log::info!("Extracted {} objects in SVG", objs.len());

    // 计算每个动画的起始时间
    conf.calc_start_times(&objs)?;
    log::info!("Calculated start times for objects");

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
    let frame_start = (conf.output.time_start * conf.output.fps as f32) as u32;
    let frame_end = (conf.output.time_end * conf.output.fps as f32) as u32;
    log::info!("Frames rendered: {} - {}", frame_start, frame_end);

    std::fs::create_dir_all(&output_dir).map_err(|e| ChronuxError::IoError(e))?;
    log::info!("Created output directory `{}`", output_dir);

    log::info!("Start rendering frames...");

    let progressbar = ProgressBar::new(frame_end as u64 - frame_start as u64).with_style(
        ProgressStyle::with_template("{msg} {wide_bar} {pos}/{len} ({percent}%) [{elapsed}<{eta}]")
            .unwrap(),
    );

    for f in frame_start..frame_end {
        progressbar.set_message(timecode(f, conf.output.fps));

        // 当前时间点（秒数）
        let t = f as f32 / conf.output.fps as f32;

        // 计算当前镜头位置
        let vb = interpolate_viewport(&conf.viewports, t).unwrap_or(Vec3(
            0.0,
            0.0,
            conf.output.width as f32,
        ));
        let (vx, vy, vw, vh) = (
            vb.0,
            vb.1,
            vb.2,
            vb.2 * conf.output.height as f32 / conf.output.width as f32,
        );

        // 创建 Skia 画布
        let mut surface =
            surfaces::raster_n32_premul((conf.output.width as i32, conf.output.height as i32))
                .ok_or(ChronuxError::SkiaError(
                    "failed to create Skia surface".into(),
                ))?;
        surface.canvas().clear(conf.output.background_color); // 设置背景颜色

        let (sx, sy) = (
            conf.output.width as f32 / vw,
            conf.output.height as f32 / vh,
        );

        // 应用镜头变换
        surface
            .canvas()
            .scale((sx, sy))
            .translate((-vx, -vy));

        // 绘制每个图形
        for o in objs.iter() {
            let properties = match timeline.get(&o.id) {
                Some(tl) => tl.interpolate(t),
                None => o.style.clone(),
            };
            render::draw(&surface.canvas(), &o.shape, &properties)?;
        }

        // 输出图片
        let output = std::path::Path::new(&output_dir).join(format!("frame_{:04}.png", f));
        let mut file = std::fs::File::create(&output).map_err(|e| ChronuxError::IoError(e))?;

        let pixmap = surface.peek_pixels().ok_or(ChronuxError::SkiaError(
            "failed to peek pixels from Skia surface".into(),
        ))?;

        skia_safe::png_encoder::encode(
            &pixmap,
            &mut file,
            &skia_safe::png_encoder::Options::default(),
        );

        progressbar.inc(1);
    }
    progressbar.finish();

    Ok(())
}
