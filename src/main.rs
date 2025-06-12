use crate::util::ChronuxError;

mod animation;
mod config;
mod render;
mod util;
// mod encoder;

fn main() -> Result<(), ChronuxError> {
    env_logger::init();

    run().or_else(|e| {
        log::error!("{:?}", e);
        Err(e)
    })
}

fn run() -> Result<(), ChronuxError> {
    let input = "D:\\Animations\\03-哈尔滨\\harbin.toml";

    // 1. 加载动画配置
    let conf = config::load(input)?;
    log::info!("Loaded config from file `{}`", input);

    // 2. 解析SVG
    // let svg = svg::parse(&conf.input.svg).expect("Failed to parse SVG");

    // 3. 处理动画
    let total_frames = (conf.output.fps as f64 * conf.output.duration) as u32;
    log::info!("Total frames: {}", total_frames);

    // 4. 渲染动画
    for f in 0..total_frames {
        // 这里可以添加对每一帧的处理逻辑
        let t = f as f64 / conf.output.fps as f64;
    }

    Ok(())
}
