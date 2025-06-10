mod config;
mod animation;
mod render;
// mod encoder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 加载配置和SVG
    // let config = config::load("config.toml");
    // let svg_data = std::fs::read("input.svg")?;

    // 2. 初始化系统
    // let mut renderer = render::Renderer::new(&svg_data);
    // // let mut encoder = encoder::VideoEncoder::new(&config.output, "output.mp4")?;
    // let event_scheduler = animation::EventScheduler::new(config.events)?;

    // // 3. 逐帧渲染
    // for frame in 0..total_frames {
    //     let t = frame as f64 / config.output.fps as f64;

    //     // 处理当前时刻的活跃事件
    //     for event in event_scheduler.get_active_events(t) {
    //         renderer.apply_animation(&event, t);
    //     }

    //     // 渲染并编码
    //     let pixmap = renderer.render_frame();
    //     // encoder.encode_frame(&pixmap)?;
    // }

    // encoder.finish()?;
    Ok(())
}
