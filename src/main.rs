mod config;
mod animation;
mod render;
mod util;
// mod encoder;

fn main() {
    // 1. 加载配置和SVG
    let input = config::load("D:\\Animations\\03-哈尔滨\\harbin.toml");
    match input {
        Ok(conf) => {
            println!("Loaded config: {:?}", conf);
            // 2. 解析SVG
            // let svg = svg::parse(&config.input.svg).expect("Failed to parse SVG");
            // 3. 处理动画
            // let animations = animation::process_animations(&svg, &config);
            // 4. 渲染动画
            // render::render_animations(&animations, &config.output);
        }
        Err(e) => {
            eprintln!("Error loading config: {:?}", e);
        }
    }
}
