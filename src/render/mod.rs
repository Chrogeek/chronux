// use resvg::usvg;

pub mod camera;

// pub struct Renderer {
//     svg_tree: usvg::Tree,
//     current_viewport: [f64; 4],
//     current_scale: f64,
// }

// impl Renderer {
//     pub fn new(svg_data: &[u8]) -> Self {
//         let opt = usvg::Options::default();
//         let tree = usvg::Tree::from_data(svg_data, &opt).unwrap();
//         Self {
//             svg_tree: tree,
//             current_viewport: [0.0, 0.0, 1920.0, 1080.0],
//             current_scale: 1.0,
//         }
//     }

//     pub fn apply_animation(&mut self, event: &ActiveEvent, t: f64) {
//         let progress = (t - event.start_time) / event.duration;
//         let value = animation::curve::evaluate_curve(&event.curve, progress);

//         match event.event_type {
//             EventType::Camera => {
//                 let params = event.params.as_camera().unwrap();
//                 self.current_viewport = lerp_array(params.viewport_start, params.viewport_end, value);
//                 self.current_scale = lerp(params.scale_start, params.scale_end, value);
//             },
//             // 其他动画类型处理...
//         }
//     }

//     pub fn render_frame(&self) -> tiny_skia::Pixmap {
//         let mut pixmap = tiny_skia::Pixmap::new(self.output_width, self.output_height).unwrap();
//         let transform = self.build_transform();

//         let opt = resvg::Options {
//             resources_dir: None,
//             transform,
//             ..Default::default()
//         };

//         resvg::render(&self.svg_tree, opt, &mut pixmap.as_mut());
//         pixmap
//     }
// }
