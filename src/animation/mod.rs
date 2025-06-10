pub mod curve;
// pub use curve::{evaluate_curve, cubic_bezier};

// use crate::config::{event::EventScheduler, AnimationEvent, EventParams};
// use super::render::Renderer;

// pub struct AnimationSystem {
//     scheduler: EventScheduler,
// }

// impl AnimationSystem {
//     pub fn new(events: Vec<AnimationEvent>) -> Result<Self, String> {
//         Ok(Self {
//             scheduler: EventScheduler::new(events)?,
//         })
//     }

//     pub fn apply_animations(&self, renderer: &mut Renderer, t: f64) {
//         for event in self.scheduler.get_active_events(t) {
//             let progress = (t - event.start_time) / event.duration;
//             let value = evaluate_curve(&event.curve, progress.clamp(0.0, 1.0));

//             match &event.event_type {
//                 EventType::Camera => {
//                     let params = event.params.as_camera().unwrap();
//                     renderer.update_camera(params, value);
//                 }
//                 EventType::TrimPath => {
//                     let params = event.params.as_trim_path().unwrap();
//                     renderer.set_path_trim(params.target, value);
//                 }
//                 // 其他动画类型...
//             }
//         }
//     }
// }
