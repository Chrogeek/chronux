use crate::util::ChronuxError;

mod animation;
mod config;
mod render;
mod util;
// mod encoder;

fn main() -> Result<(), ChronuxError> {
    // 1. 加载动画配置
    let conf = config::load("D:\\Animations\\03-哈尔滨\\harbin.toml")?;
    println!("Loaded config: {:?}", conf);

    // 2. 解析SVG
    // let svg = svg::parse(&conf.input.svg).expect("Failed to parse SVG");

    // 3. 处理动画
    // let animations = animation::process_animations(&svg, &conf);

    // 4. 渲染动画
    // render::render_animations(&animations, &conf.output);

    Ok(())
}
