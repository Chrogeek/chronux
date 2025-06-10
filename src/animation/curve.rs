// use crate::config::AnimationCurve;

// pub fn cubic_bezier(t: f64, p1: f64, p2: f64, p3: f64, p4: f64) -> f64 {
//     // 实现三阶贝塞尔曲线计算
//     let t1 = 1.0 - t;
//     t1.powi(3) * p1
//         + 3.0 * t1.powi(2) * t * p2
//         + 3.0 * t1 * t.powi(2) * p3
//         + t.powi(3) * p4
// }

// pub fn evaluate_curve(curve: &AnimationCurve, t: f64) -> f64 {
//     match curve {
//         AnimationCurve::Bezier { bezier } => cubic_bezier(t, bezier[0], bezier[1], bezier[2], bezier[3]),
//         AnimationCurve::Preset { preset } => match preset.as_str() {
//             "ease_in" => t * t,
//             "ease_out" => 1.0 - (1.0 - t) * (1.0 - t),
//             _ => t, // linear
//         },
//     }
// }
