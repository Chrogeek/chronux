use itertools::multizip;
use serde::Deserialize;
use skia_safe::{Path, PathMeasure};

use crate::util::{ChronuxError, ChronuxResult, interpolator::Interpolator, vector::Vec3};

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum TimeKey {
    /// 与被依赖动画同时开始，延迟 delay 秒
    #[serde(rename = "delay")]
    WithPrevious { depends: String, delay: f32 },
    /// 被依赖动画完全结束后，延迟 offset 秒开始
    #[serde(rename = "offset")]
    AfterPrevious { depends: String, offset: f32 },
}

#[derive(Debug, Deserialize, Clone)]
pub struct ViewportKeyframe {
    #[serde(flatten)]
    pub time: TimeKey,
    pub x: Option<f32>,
    pub y: Option<f32>,
    #[serde(rename = "width")]
    pub size: Option<f32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ViewportTrack {
    #[serde(rename = "tracked-path")]
    pub tracked_path: String,
    pub keyframes: Vec<ViewportKeyframe>,
}

#[derive(Debug, Clone, Default)]
pub struct ViewportSequence {
    pub vp_pos_inter: Vec<Interpolator>,
    pub vp_size_inter: Vec<Interpolator>,
    pub vp_tracks: Vec<Path>,
    pub vp_track_paths: Vec<String>,
}

impl ViewportSequence {
    pub fn interpolate_vp(&self, t: f32) -> ChronuxResult<Vec3> {
        // Try to interpolate with tracks first
        let vp_zip = multizip((
            self.vp_pos_inter.iter(),
            self.vp_size_inter.iter(),
            self.vp_tracks.iter(),
        ))
        .collect::<Vec<_>>();
        for (pos, size, path) in vp_zip.iter() {
            let pos = pos.interpolate(t);
            let size = size.interpolate(t);
            if let Some(pos) = pos
                && let Some(size) = size
            {
                let mut pm = PathMeasure::new(path, false, Some(2.0));
                let pos = pm.pos_tan(pos).map(|(p, _)| p).unwrap_or_else(|| {
                    self.vp_tracks[0]
                        .get_point(0)
                        .expect("path has at least one point")
                });
                return Ok(Vec3(pos.x, pos.y, size));
            }
        }

        // If no tracks are available, try to interpolate between tracks
        for w in vp_zip.windows(2) {
            let (pos1, size1, path1) = w[0];
            let (pos2, size2, path2) = w[1];

            let te1 = pos1.t_end();
            let ts2 = pos2.t_start();

            if te1 <= t && t <= ts2 {
                let p = (t - te1) / (ts2 - te1);
                let mut pm1 = PathMeasure::new(path1, false, Some(2.0));
                let mut pm2 = PathMeasure::new(path2, false, Some(2.0));
                let path1_len = pm1.length();
                let p1 = pm1
                    .pos_tan(path1_len)
                    .map(|(p, _)| p)
                    .unwrap_or_else(|| path1.get_point(0).expect("path has at least one point"));
                let p2 = pm2
                    .pos_tan(0.0)
                    .map(|(p, _)| p)
                    .unwrap_or_else(|| path2.get_point(0).expect("path has at least one point"));
                let pos = p1 + (p2 - p1) * p;
                let size = size1.p_end() + (size2.p_start() - size1.p_end()) * p;
                return Ok(Vec3(pos.x, pos.y, size));
            }
        }

        if t <= self.vp_pos_inter[0].t_start() {
            // If t is before the first track, use the first track's position and size
            let size = self.vp_size_inter[0].p_start();
            let mut pm = PathMeasure::new(&self.vp_tracks[0], false, Some(2.0));
            let pos = pm.pos_tan(0.0).map(|(p, _)| p).unwrap_or_else(|| {
                self.vp_tracks[0]
                    .get_point(0)
                    .expect("path has at least one point")
            });
            return Ok(Vec3(pos.x, pos.y, size));
        } else if let Some(vp_pos_last) = self.vp_pos_inter.last()
            && t >= vp_pos_last.t_end()
        {
            // If t is after the last track, use the last track's position and size
            let size = self.vp_size_inter.last().unwrap().p_end();
            let mut pm = PathMeasure::new(self.vp_tracks.last().unwrap(), false, Some(2.0));
            let len = pm.length();
            let pos = pm.pos_tan(len).unwrap().0;
            return Ok(Vec3(pos.x, pos.y, size));
        }

        Err(ChronuxError::ArgError(format!(
            "Viewport interpolation failed for time {t}"
        )))
    }
}
