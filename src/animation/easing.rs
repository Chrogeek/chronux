use roots::Roots;
use serde::Deserialize;

/// 三阶贝塞尔曲线插值
fn cubic_bezier(x: f32, p1: f32, p2: f32, p3: f32, p4: f32) -> f32 {
    let t = roots::find_roots_cubic(
        3.0 * p1 - 3.0 * p3 + 1.0,
        -6.0 * p1 + 3.0 * p3,
        3.0 * p1,
        -x,
    );
    let t = match t {
        Roots::One([t]) => t,
        Roots::Two([t1, t2]) => {
            if t1 >= 0.0 && t1 <= 1.0 {
                t1
            } else if t2 >= 0.0 && t2 <= 1.0 {
                t2
            } else {
                unreachable!()
            }
        }
        Roots::Three([t1, t2, t3]) => {
            let mut valid_t = vec![];
            if t1 >= 0.0 && t1 <= 1.0 {
                valid_t.push(t1);
            }
            if t2 >= 0.0 && t2 <= 1.0 {
                valid_t.push(t2);
            }
            if t3 >= 0.0 && t3 <= 1.0 {
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
    Linear,
    CubicBezier(f32, f32, f32, f32),
}

impl<'de> Deserialize<'de> for Easing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        match s.as_str() {
            "linear" => Ok(Easing::Linear),
            "ease-in" => Ok(Easing::CubicBezier(0.42, 0.0, 1.0, 1.0)),
            "ease-out" => Ok(Easing::CubicBezier(0.0, 0.0, 0.58, 1.0)),
            "ease-in-out" => Ok(Easing::CubicBezier(0.42, 0.0, 0.58, 1.0)),
            _ => Err(serde::de::Error::custom(format!(
                "Unknown easing type: {}",
                s
            ))),
        }
    }
}

impl Easing {
    pub fn evaluate(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0); // Ensure t is within [0, 1]
        let result = match self {
            Easing::Linear => t,
            Easing::CubicBezier(p1, p2, p3, p4) => cubic_bezier(t, *p1, *p2, *p3, *p4),
        };

        log::trace!(
            "Evaluated easing at t={:.2} = {}, with easing = {:?}",
            t,
            result,
            self
        );
        result
    }
}
