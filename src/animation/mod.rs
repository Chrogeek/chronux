use serde::Deserialize;
use skia_safe::PathMeasure;

use crate::{
    config::event::Event,
    svg::element::{Object, Shape},
    util::{ChronuxError, ChronuxResult},
};

pub mod easing;
pub mod properties;
pub mod timeline;

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "animation")]
pub enum Animation {
    #[serde(rename = "trim-path")]
    TrimPath { reverse: bool },
}

pub fn track(obj: &Object, event: &Event, x: f32, y: f32) -> ChronuxResult<f32> {
    match event.animation {
        Animation::TrimPath { reverse } => {
            const SEGMENTS: usize = 100;

            if let Shape::Path { d } = &obj.shape {
                let mut pm = PathMeasure::new(&d, false, Some(2.0));
                let length = pm.length();

                let closest = (0..SEGMENTS + 1)
                    .map(|i| {
                        let t = event.easing.evaluate(i as f32 / SEGMENTS as f32);
                        let dis = t * length;
                        (i, pm.pos_tan(dis).unwrap().0)
                    })
                    .min_by(|(_, p1), (_, p2)| {
                        let d1 = (p1.x - x).powi(2) + (p1.y - y).powi(2);
                        let d2 = (p2.x - x).powi(2) + (p2.y - y).powi(2);
                        d1.partial_cmp(&d2).unwrap()
                    })
                    .unwrap();

                let mut tl = (closest.0 - 1).max(0) as f32 / SEGMENTS as f32;
                let mut tr = (closest.0 + 1).min(SEGMENTS) as f32 / SEGMENTS as f32;

                while tr - tl > 1e-4 {
                    let ml = tl + (tr - tl) / 3.0;
                    let mr = tr - (tr - tl) / 3.0;
                    let ml = event.easing.evaluate(ml);
                    let mr = event.easing.evaluate(mr);
                    let dl = pm.pos_tan(ml * length).unwrap().0;
                    let dr = pm.pos_tan(mr * length).unwrap().0;
                    let d1 = (dl.x - x).powi(2) + (dl.y - y).powi(2);
                    let d2 = (dr.x - x).powi(2) + (dr.y - y).powi(2);
                    if d1 < d2 {
                        tr = mr;
                    } else {
                        tl = ml;
                    }
                }

                let tm = (tl + tr) / 2.0;

                return Ok(if reverse { 1.0 - tm } else { tm });
            } else {
                return Err(ChronuxError::ArgError(
                    "non-path shapes cannot be tracked".into(),
                ));
            };
        }
    }
}
