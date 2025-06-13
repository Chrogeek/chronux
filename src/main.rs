use std::collections::HashSet;

use crate::{config::Config, svg::Svg, util::ChronuxError};

mod animation;
mod config;
mod render;
mod svg;
mod util;

fn main() -> Result<(), ChronuxError> {
    env_logger::init();
    log::info!("Welcome to Chronux!");

    run().or_else(|e| {
        log::error!("{:?}", e);
        Err(e)
    })
}

fn run() -> Result<(), ChronuxError> {
    let input = "D:\\Animations\\03-哈尔滨\\harbin.toml";

    // 1. 加载动画配置
    let conf = Config::load(input)?;
    log::info!("Loaded config from file `{}`", input);

    // 提取所需的SVG元素及时间线
    let timeline = conf.calc_timeline()?;

    let ids = timeline.keys().cloned().collect::<HashSet<_>>();
    log::trace!("Timeline IDs: {:?}", ids);

    // 2. 解析SVG
    let svg = Svg::from_file(&conf.input.svg)?;
    log::info!("Loaded SVG from file `{}`", conf.input.svg);

    // 读取SVG中的路径、圆形和文本
    // let svg_size = svg.size();
    // log::info!("SVG size: {}x{}", svg_size.width(), svg_size.height());

    let objs = svg.search_labels(&ids)?;
    log::info!("Found {} objects in SVG", objs.len());

    // 3. 处理动画
    let total_frames = (conf.output.fps as f64 * conf.output.duration) as u32;
    log::info!("Total frames: {}", total_frames);

    // 4. 渲染动画
    // for f in 0..total_frames {
    //     // 这里可以添加对每一帧的处理逻辑
    //     let _t = f as f64 / conf.output.fps as f64;
    // }

    Ok(())
}
