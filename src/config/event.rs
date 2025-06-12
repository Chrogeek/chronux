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
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl<'de> Deserialize<'de> for EventEasing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        match s.as_str() {
            "linear" => Ok(EventEasing::Linear),
            "ease-in" => Ok(EventEasing::EaseIn),
            "ease-out" => Ok(EventEasing::EaseOut),
            "ease-in-out" => Ok(EventEasing::EaseInOut),
            _ => Err(serde::de::Error::custom(format!("Unknown easing type: {}", s))),
        }
    }
}
