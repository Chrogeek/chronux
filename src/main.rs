use std::collections::HashSet;

use clap::Parser;
use indicatif::ProgressIterator;
use skia_safe::surfaces;

use crate::{
    animation::Animation, args::Args, config::Config, svg::Svg, util::{timecode, ChronuxError}
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

    // 提取所需的SVG元素及时间线
    let mut timeline = conf.calc_timeline()?;

    let ids = timeline.keys().cloned().collect::<HashSet<_>>();
    log::trace!("Timeline IDs: {:?}", ids);

    // 解析SVG
    let svg = Svg::from_file(&conf.input.svg)?;
    log::info!("Loaded SVG from file `{}`", conf.input.svg);

    log::info!(
        "SVG dimensions: {}x{}",
        svg.dimensions.width,
        svg.dimensions.height
    );
    log::info!("SVG viewbox: {:?}", svg.dimensions.viewbox);

    // 读取 SVG 中的路径、圆形和文本
    let objs = svg.search_labels(&ids)?;
    log::info!("Extracted {} objects in SVG", objs.len());

    for o in objs.iter() {
        let tl = timeline.get_mut(&o.id).unwrap();

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

    conf.calc_start_times(&objs)?;

    // 渲染动画
    let frame_start = (conf.output.time_start * conf.output.fps as f32) as u32;
    let frame_end = (conf.output.time_end * conf.output.fps as f32) as u32;
    log::info!("Frames rendered: {} - {}", frame_start, frame_end);

    std::fs::create_dir_all(&output_dir).map_err(|e| ChronuxError::IoError(e))?;
    log::info!("Created output directory `{}`", output_dir);

    log::info!("Rendering animation...");
    for f in (frame_start..frame_end).progress() {
        log::trace!(
            "Rendering frame {} (timecode {})",
            f,
            timecode(f, conf.output.fps)
        );

        let t = f as f32 / conf.output.fps as f32;

        let mut surface =
            surfaces::raster_n32_premul((conf.output.width as i32, conf.output.height as i32))
                .ok_or(ChronuxError::SkiaError(
                    "failed to create Skia surface".into(),
                ))?;

        surface.canvas().clear(conf.output.background_color);

        for o in objs.iter() {
            let tl = &timeline[&o.id];
            let properties = tl.interpolate(t);
            render::draw(&surface.canvas(), &o.shape, &properties)?;
        }

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
    }

    Ok(())
}
