use serde::Deserialize;

/// 三阶贝塞尔曲线插值
fn cubic_bezier(t: f64, p1: f64, p2: f64, p3: f64, p4: f64) -> f64 {
    let t1 = 1.0 - t;
    t1.powi(3) * p1 + 3.0 * t1.powi(2) * t * p2 + 3.0 * t1 * t.powi(2) * p3 + t.powi(3) * p4
}

#[derive(Debug)]
pub enum Easing {
    Linear,
    CubicBezier(f64, f64, f64, f64),
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
    pub fn evaluate(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0); // Ensure t is within [0, 1]
        match self {
            Easing::Linear => t,
            Easing::CubicBezier(p1, p2, p3, p4) => cubic_bezier(t, *p1, *p2, *p3, *p4),
        }
    }
}
