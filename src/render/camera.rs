// use tiny_skia::Transform;
// use super::Renderer;

// #[derive(Debug, Clone)]
// pub struct CameraState {
//     pub viewport: [f64; 4], // x, y, width, height
//     pub scale: f64,
// }

// impl CameraState {
//     pub fn to_transform(&self, output_width: u32, output_height: u32) -> Transform {
//         let scale = self.scale * (output_width as f64 / self.viewport[2]).min(
//             output_height as f64 / self.viewport[3],
//         );

//         Transform::from_scale(scale as f32, scale as f32).post_translate(
//             (-self.viewport[0] * scale) as f32,
//             (-self.viewport[1] * scale) as f32,
//         )
//     }
// }

// impl Renderer {
//     pub fn update_camera(&mut self, params: &CameraParams, progress: f64) {
//         self.current_viewport = lerp_array(params.viewport_start, params.viewport_end, progress);
//         self.current_scale = lerp(params.scale_start, params.scale_end, progress);
//     }
// }

// fn lerp_array(a: [f64; 4], b: [f64; 4], t: f64) -> [f64; 4] {
//     [
//         a[0] + (b[0] - a[0]) * t,
//         a[1] + (b[1] - a[1]) * t,
//         a[2] + (b[2] - a[2]) * t,
//         a[3] + (b[3] - a[3]) * t,
//     ]
// }
