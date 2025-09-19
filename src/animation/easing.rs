use std::f32::consts::PI;

use roots::Roots;
use serde::Deserialize;

/// 三阶贝塞尔曲线插值
pub fn cubic_bezier(x: f32, p1: f32, p2: f32, p3: f32, p4: f32) -> f32 {
    if x < 1e-4 {
        return 0.0;
    } else if x > 1.0 - 1e-4 {
        return 1.0;
    }
    let t = roots::find_roots_cubic(
        3.0 * p1 - 3.0 * p3 + 1.0,
        -6.0 * p1 + 3.0 * p3,
        3.0 * p1,
        -x,
    );
    let t = match t {
        Roots::One([t]) => t,
        Roots::Two([t1, t2]) => {
            if (0.0..=1.0).contains(&t1) {
                t1
            } else if (0.0..=1.0).contains(&t2) {
                t2
            } else {
                unreachable!()
            }
        }
        Roots::Three([t1, t2, t3]) => {
            let mut valid_t = vec![];
            if (0.0..=1.0).contains(&t1) {
                valid_t.push(t1);
            }
            if (0.0..=1.0).contains(&t2) {
                valid_t.push(t2);
            }
            if (0.0..=1.0).contains(&t3) {
                valid_t.push(t3);
            }
            if valid_t.len() == 1 {
                valid_t[0]
            } else {
                unreachable!()
            }
        }
        _ => todo!(),
    };

    (3.0 * t - 6.0 * t.powi(2) + 3.0 * t.powi(3)) * p2
        + (3.0 * t.powi(2) - 3.0 * t.powi(3)) * p4
        + t.powi(3)
}

#[derive(Debug, Clone)]
pub enum Easing {
    // 线性动画，无缓动
    Linear,

    // 三阶贝塞尔曲线，四个参数分别为两个控制点的 x 和 y 坐标，x 坐标范围为 [0, 1]，y 坐标无限制
    CubicBezier(f32, f32, f32, f32), // p1x, p1y, p2x, p2y

    // 预定义的缓动函数
    EaseIn,
    EaseOut,
    EaseInOut,

    // 基于正弦函数的缓动
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,

    // 基于轻微贝塞尔曲线的缓动（平滑淡入淡出，但基本上没有速度变化）
    EaseInSlight,
    EaseOutSlight,
    EaseInOutSlight,

    // 基于回弹效果的缓动
    EaseOutBack,
    EaseInOutBack,

    // 基于二次方函数的缓动
    EaseInOutQuad,

    // 基于三次方函数的缓动
    EaseInOutCubic,
}

impl<'de> Deserialize<'de> for Easing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        match s.as_str() {
            "linear" => Ok(Easing::Linear),
            "ease-in" => Ok(Easing::EaseIn),
            "ease-in-sine" => Ok(Easing::EaseInSine),
            "ease-in-slight" => Ok(Easing::EaseInSlight),
            "ease-out" => Ok(Easing::EaseOut),
            "ease-out-sine" => Ok(Easing::EaseOutSine),
            "ease-out-slight" => Ok(Easing::EaseOutSlight),
            "ease-out-back" => Ok(Easing::EaseOutBack),
            "ease-in-out" => Ok(Easing::EaseInOut),
            "ease-in-out-quad" => Ok(Easing::EaseInOutQuad),
            "ease-in-out-slight" => Ok(Easing::EaseInOutSlight),
            "ease-in-out-cubic" => Ok(Easing::EaseInOutCubic),
            "ease-in-out-sine" => Ok(Easing::EaseInOutSine),
            "ease-in-out-back" => Ok(Easing::EaseInOutBack),
            s if s.starts_with("cubic-bezier") => {
                let params: Vec<f32> = s["cubic-bezier(".len()..s.len() - 1]
                    .split(',')
                    .map(|p| p.trim().parse::<f32>())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| {
                        serde::de::Error::custom(format!("Invalid cubic-bezier params: {e}"))
                    })?;
                if params.len() != 4 {
                    return Err(serde::de::Error::custom(
                        "Cubic-bezier requires 4 parameters",
                    ));
                }
                Ok(Easing::CubicBezier(
                    params[0], params[1], params[2], params[3],
                ))
            }
            _ => Err(serde::de::Error::custom(format!(
                "Unknown easing type: {s}"
            ))),
        }
    }
}

impl Easing {
    pub fn evaluate(&self, t_in: f32) -> f32 {
        let t = t_in.clamp(0.0, 1.0); // Ensure t is within [0, 1]
        match self {
            Easing::Linear => t,
            Easing::CubicBezier(p1x, p1y, p2x, p2y) => cubic_bezier(t, *p1x, *p1y, *p2x, *p2y),
            Easing::EaseIn | Easing::EaseInSine => 1.0 - ((t * PI) / 2.0).cos(),
            Easing::EaseInSlight => cubic_bezier(t, 0.2, 0.0, 1.0, 1.0),
            Easing::EaseOut | Easing::EaseOutSine => ((t * PI) / 2.0).sin(),
            Easing::EaseOutSlight => cubic_bezier(t, 0.0, 0.0, 0.8, 1.0),
            Easing::EaseInOut | Easing::EaseInOutSine => -((PI * t).cos() - 1.0) / 2.0,
            Easing::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Easing::EaseInOutSlight => cubic_bezier(t, 0.2, 0.0, 0.8, 1.0),
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t.powi(3)
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Easing::EaseOutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
            Easing::EaseInOutBack => {
                let c1 = 1.70158;
                let c2 = c1 * 1.525;
                if t < 0.5 {
                    ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
                }
            }
        }
    }
}
