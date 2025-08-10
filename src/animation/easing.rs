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
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    // CubicBezier(f32, f32, f32, f32),
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
            "ease-out" => Ok(Easing::EaseOut),
            "ease-in-out" => Ok(Easing::EaseInOut),
            _ => Err(serde::de::Error::custom(format!(
                "Unknown easing type: {s}"
            ))),
        }
    }
}

impl Easing {
    pub fn evaluate(&self, t_in: f32, total_time: f32) -> f32 {
        let t = t_in.clamp(0.0, 1.0); // Ensure t is within [0, 1]
        let bez = if total_time < 4.0 { 0.42 } else { 1.68 / total_time };
        match self {
            Easing::Linear => t,
            Easing::EaseIn => cubic_bezier(t, bez, 0.0, 1.0, 1.0),
            Easing::EaseOut => cubic_bezier(t, 0.0, 0.0, 1.0 - bez, 1.0),
            Easing::EaseInOut => cubic_bezier(t, bez, 0.0, 1.0 - bez, 1.0),
            // Easing::CubicBezier(p1, p2, p3, p4) => cubic_bezier(t, *p1, *p2, *p3, *p4),
        }
    }
}
