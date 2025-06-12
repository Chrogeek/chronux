use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Event {
    pub id: String,
    #[serde(default = "Vec::new")]
    pub depends: Vec<String>,
    pub object: String,
    pub easing: EventEasing,
    pub offset: f64,
    pub duration: f64,
}

#[derive(Debug)]
pub enum EventEasing {
    Linear,
    CubicBezier(f64, f64, f64, f64),
}

impl<'de> Deserialize<'de> for EventEasing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        match s.as_str() {
            "linear" => Ok(EventEasing::Linear),
            "ease-in" => Ok(EventEasing::CubicBezier(0.42, 0.0, 1.0, 1.0)),
            "ease-out" => Ok(EventEasing::CubicBezier(0.0, 0.0, 0.58, 1.0)),
            "ease-in-out" => Ok(EventEasing::CubicBezier(0.42, 0.0, 0.58, 1.0)),
            _ => Err(serde::de::Error::custom(format!("Unknown easing type: {}", s))),
        }
    }
}
