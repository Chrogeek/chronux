use std::collections::HashMap;

use serde::Deserialize;
use serde_with::serde_as;

use crate::{
    animation::easing::Easing,
    config::{event::Event, viewport::TimeKey},
    util::{ChronuxError, ChronuxResult},
};

#[derive(Debug, Deserialize, Clone)]
pub enum DatesAnimation {
    #[serde(rename = "rolling")]
    Rolling,
}

#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub struct DatesConfig {
    #[serde(rename = "font-family")]
    pub font_family: Option<String>,
    #[serde(rename = "font-weight-year")]
    pub font_weight_year: f32,
    #[serde(rename = "font-size-year")]
    pub font_size_year: f32,
    #[serde(rename = "font-weight-date")]
    pub font_weight_date: f32,
    #[serde(rename = "font-size-date")]
    pub font_size_date: f32,
    #[serde(rename = "position-year")]
    pub position_year: String,
    #[serde(rename = "position-date")]
    pub position_date: String,
    #[serde(rename = "area-year")]
    pub area_year: String,
    #[serde(rename = "area-date")]
    pub area_date: String,
    #[serde_with(as = "OneOrMany<_, PreferOne>")]
    pub keyframes: Vec<DatesKeyframe>,
    pub easing: Easing,
    pub animation: DatesAnimation,
    #[serde(rename = "fill-color")]
    pub fill_color: String,
    #[serde(rename = "line-height")]
    pub line_height: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatesKeyframe {
    #[serde(flatten)]
    pub time: TimeKey,
    pub year: Option<String>,
    pub date: Option<String>,
}

impl DatesConfig {
    pub fn parse_timekeys(
        &self,
        events: &HashMap<String, Event>,
    ) -> ChronuxResult<Vec<(f32, String, String)>> {
        self.keyframes
            .iter()
            .map(|kf| {
                let time = match &kf.time {
                    TimeKey::WithPrevious { depends, delay } => {
                        let dep = events.get(depends).map_or(
                            Err(ChronuxError::ArgError(format!(
                                "Timeline keyframe dependency {} is broken",
                                depends
                            ))),
                            |o| Ok(o),
                        )?;
                        dep.start + delay
                    }
                    TimeKey::AfterPrevious { depends, offset } => {
                        let dep = events.get(depends).map_or(
                            Err(ChronuxError::ArgError(format!(
                                "Timeline keyframe dependency {} is broken",
                                depends
                            ))),
                            |o| Ok(o),
                        )?;
                        dep.start + dep.duration + offset
                    }
                };
                Ok((
                    time,
                    kf.year.clone().unwrap_or(String::new()),
                    kf.date.clone().unwrap_or(String::new()),
                ))
            })
            .collect()
    }
}
