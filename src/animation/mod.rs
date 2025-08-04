use serde::Deserialize;
use skia_safe::{Path, PathMeasure};

use crate::{
    animation::{easing::Easing, tween::Tween},
    config::event::Event,
    svg::element::{Object, Shape},
    util::{ChronuxError, ChronuxResult},
};

pub mod easing;
pub mod properties;
pub mod timeline;
pub mod tween;

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "animation")]
pub enum Animation {
    #[serde(rename = "trim-path")]
    TrimPath { reverse: bool },
    #[serde(rename = "trim-path-out")]
    TrimPathOut { reverse: bool },
    #[serde(rename = "typewriter")]
    Typewriter,
    #[serde(rename = "typewriter-out")]
    TypewriterOut,
    #[serde(rename = "sweep")]
    Sweep,
    #[serde(rename = "sweep-out")]
    SweepOut,
    #[serde(rename = "tween")]
    Tween(Tween),
    #[serde(rename = "fade-in")]
    FadeIn,
    #[serde(rename = "fade-out")]
    FadeOut,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TaggedAnimation {
    #[serde(rename = "object")]
    pub obj_id: String,
    #[serde(flatten)]
    pub animation: Animation,
}

pub fn track(obj: &Object, event: &Event, x: f32, y: f32) -> ChronuxResult<f32> {
    let t = match event.animation {
        Animation::TrimPath { reverse } => {
            if let Shape::Path { d } = &obj.shape {
                let t = track_on_path(d, x, y, &event.easing, event.duration).0;
                let t = if reverse { 1.0 - t } else { t };
                Ok(t)
            } else {
                Err(ChronuxError::ArgError(
                    "non-path shapes cannot be tracked".into(),
                ))
            }
        }
        Animation::TrimPathOut { reverse } => {
            if let Shape::Path { d } = &obj.shape {
                let t = track_on_path(d, x, y, &event.easing, event.duration).0;
                let t = if reverse { t } else { 1.0 - t };
                Ok(t)
            } else {
                Err(ChronuxError::ArgError(
                    "non-path shapes cannot be tracked".into(),
                ))
            }
        }
        _ => Err(ChronuxError::ArgError(format!(
            "animation '{:?}' cannot be tracked",
            event.animation
        ))),
    }?;
    Ok(t * event.duration)
}

/// Calculate the nearest point on path d to given point (x, y).
/// Returns (t, d) where d is the path length, l is the length from path origin to the nearest point, and t = l / d.
pub fn track_on_path(d: &Path, x: f32, y: f32, easing: &Easing, total_time: f32) -> (f32, f32) {
    const SEGMENTS: usize = 20;

    let mut pm = PathMeasure::new(&d, false, Some(2.0));
    let length = pm.length();

    let closest = (0..SEGMENTS + 1)
        .map(|i| {
            let t = easing.evaluate(i as f32 / SEGMENTS as f32, total_time);
            let dis = t * length;
            (t, pm.pos_tan(dis).unwrap().0)
        })
        .min_by(|(_, p1), (_, p2)| {
            let d1 = (p1.x - x).powi(2) + (p1.y - y).powi(2);
            let d2 = (p2.x - x).powi(2) + (p2.y - y).powi(2);
            d1.partial_cmp(&d2).unwrap()
        })
        .unwrap();

    let mut tl = (closest.0 - 1.0 / SEGMENTS as f32).max(0.0);
    let mut tr = (closest.0 + 1.0 / SEGMENTS as f32).min(1.0);

    for _ in 0..20 {
        let ml = tl + (tr - tl) / 3.0;
        let mr = tr - (tr - tl) / 3.0;
        let dl = pm.pos_tan(easing.evaluate(ml, total_time) * length).unwrap().0;
        let dr = pm.pos_tan(easing.evaluate(mr, total_time) * length).unwrap().0;
        let d1 = (dl.x - x).powi(2) + (dl.y - y).powi(2);
        let d2 = (dr.x - x).powi(2) + (dr.y - y).powi(2);
        if d1 < d2 {
            tr = mr;
        } else {
            tl = ml;
        }
    }

    ((tl + tr) / 2.0, length)
}
