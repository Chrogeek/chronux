use std::collections::HashMap;

use serde::Deserialize;

use crate::{
    animation::easing::cubic_bezier,
    config::event::Event,
    util::{
        ChronuxError, ChronuxResult,
        vector::{Vec3, Vec6},
    },
};

#[derive(Debug)]
pub struct Viewport {
    pub depends: String,
    pub offset: f32,
    pub smooth: ViewportSmooth,
    pub cx: f32,
    pub cy: f32,
    pub width: f32,
    // 以下属性不从 TOML 中提取，而是计算得出，反序列化时被忽略
    pub t: f32,
    ctl: Vec6,
    ctr: Vec6,
}

impl<'de> Deserialize<'de> for Viewport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map = toml::Table::deserialize(deserializer)?;

        let viewport = Viewport {
            depends: map
                .get("depends")
                .and_then(|v| v.as_str())
                .ok_or(serde::de::Error::custom(
                    "Missing or invalid 'depends' field in viewport",
                ))?
                .to_string(),
            offset: map
                .get("offset")
                .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
                .ok_or(serde::de::Error::custom(
                    "Missing or invalid 'offset' field in viewport",
                ))? as f32,
            smooth: map
                .get("smooth")
                .and_then(|v| v.as_str())
                .and_then(|s| ViewportSmooth::from_str(s))
                .ok_or(serde::de::Error::custom(
                    "Missing or invalid 'smooth' field in viewport",
                ))?,
            cx: map
                .get("cx")
                .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
                .ok_or(serde::de::Error::custom(
                    "Missing or invalid 'cx' field in viewport",
                ))? as f32,
            cy: map
                .get("cy")
                .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
                .ok_or(serde::de::Error::custom(
                    "Missing or invalid 'cy' field in viewport",
                ))? as f32,
            width: map
                .get("width")
                .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
                .ok_or(serde::de::Error::custom(
                    "Missing or invalid 'width' field in viewport",
                ))? as f32,
            // Skipped fields, not extracted from deserialized TOML table
            t: f32::NAN, // Start time will be calculated later
            ctl: Vec6(1.0, 1.0, 1.0, 1.0, 1.0, 1.0), // Default control points
            ctr: Vec6(0.0, 0.0, 0.0, 0.0, 0.0, 0.0), // Default control points
        };
        Ok(viewport)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub enum ViewportSmooth {
    /// 线性插值
    #[serde(rename = "linear")]
    Linear,
    /// 缓入插值
    #[serde(rename = "ease-in")]
    EaseIn,
    /// 缓出插值
    #[serde(rename = "ease-out")]
    EaseOut,
    /// 缓入缓出插值
    #[serde(rename = "ease-in-out")]
    EaseInOut,
    /// 自动贝塞尔曲线插值
    #[serde(rename = "auto")]
    AutoBezier,
    /// 在下一个关键帧到来之前保持不变
    #[serde(rename = "freeze")]
    Freeze,
}

impl ViewportSmooth {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "linear" => Some(ViewportSmooth::Linear),
            "ease-in" => Some(ViewportSmooth::EaseIn),
            "ease-out" => Some(ViewportSmooth::EaseOut),
            "ease-in-out" => Some(ViewportSmooth::EaseInOut),
            "auto" => Some(ViewportSmooth::AutoBezier),
            "freeze" => Some(ViewportSmooth::Freeze),
            _ => None,
        }
    }
}

pub fn sort_viewports(viewports: &mut Vec<Viewport>) {
    viewports.sort_by(|a, b| a.t.partial_cmp(&b.t).expect("Failed to compare viewports"));
}

pub fn calc_viewport_times(
    viewports: &mut Vec<Viewport>,
    events: &HashMap<String, Event>,
) -> ChronuxResult<()> {
    for vp in viewports.iter_mut() {
        if let Some(ev) = events.get(&vp.depends) {
            vp.t = ev.start + vp.offset;
        } else {
            return Err(ChronuxError::BrokenDependency(format!(
                "a viewport keyframe depends on non-existent event {}",
                vp.depends
            )));
        }
    }
    Ok(())
}

pub fn calc_viewport_keyframe_bezier(viewports: &mut Vec<Viewport>) {
    let len = viewports.len();
    for i in 0..len {
        if i > 0 {
            assert!(
                viewports[i].t > viewports[i - 1].t,
                "viewport keyframes must be in ascending order (failed between index {} and {})",
                i - 1,
                i
            );
        }
        match viewports[i].smooth {
            ViewportSmooth::Linear | ViewportSmooth::Freeze => {
                viewports[i].ctl = Vec6(1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
                viewports[i].ctr = Vec6(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            }
            ViewportSmooth::EaseIn => {
                viewports[i].ctl = Vec6(0.58, 1.0, 0.58, 1.0, 0.58, 1.0);
                viewports[i].ctr = Vec6(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            }
            ViewportSmooth::EaseOut => {
                viewports[i].ctl = Vec6(1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
                viewports[i].ctr = Vec6(0.42, 0.0, 0.42, 0.0, 0.42, 0.0);
            }
            ViewportSmooth::EaseInOut => {
                viewports[i].ctl = Vec6(0.58, 1.0, 0.58, 1.0, 0.58, 1.0);
                viewports[i].ctr = Vec6(0.42, 0.0, 0.42, 0.0, 0.42, 0.0);
            }
            ViewportSmooth::AutoBezier => {
                if len == 1 {
                    viewports[i].ctl = Vec6(1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
                    viewports[i].ctr = Vec6(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
                } else if i == 0 {
                    viewports[i].ctl = Vec6(1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
                    viewports[i].ctr = Vec6(0.42, 0.0, 0.42, 0.0, 0.42, 0.0);
                } else if i == len - 1 {
                    viewports[i].ctl = Vec6(0.58, 1.0, 0.58, 1.0, 0.58, 1.0);
                    viewports[i].ctr = Vec6(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
                } else {
                    let v1 = Vec3(
                        viewports[i - 1].cx,
                        viewports[i - 1].cy,
                        viewports[i - 1].width,
                    );
                    let v2 = Vec3(viewports[i].cx, viewports[i].cy, viewports[i].width);
                    let v3 = Vec3(
                        viewports[i + 1].cx,
                        viewports[i + 1].cy,
                        viewports[i + 1].width,
                    );
                    let t1 = viewports[i - 1].t;
                    let t2 = viewports[i].t;
                    let t3 = viewports[i + 1].t;

                    let deriv =
                        (v2 - v1) / (t2 - t1) + (v2 - v3) / (t2 - t3) - (v1 - v3) / (t1 - t3);

                    let mut l = 0.42 * deriv / (t2 - t1);
                    let mut r = 0.42 * deriv / (t3 - t2);

                    if (v2.0 - v1.0).abs() < 1e-4 {
                        l.0 = 1.0;
                    } else {
                        l.0 /= v2.0 - v1.0;
                    }
                    if (v2.1 - v1.1).abs() < 1e-4 {
                        l.1 = 1.0;
                    } else {
                        l.1 /= v2.1 - v1.1;
                    }
                    if (v2.2 - v1.2).abs() < 1e-4 {
                        l.2 = 1.0;
                    } else {
                        l.2 /= v2.2 - v1.2;
                    }
                    if (v3.0 - v2.0).abs() < 1e-4 {
                        r.0 = 0.0;
                    } else {
                        r.0 /= v3.0 - v2.0;
                    }
                    if (v3.1 - v2.1).abs() < 1e-4 {
                        r.1 = 0.0;
                    } else {
                        r.1 /= v3.1 - v2.1;
                    }
                    if (v3.2 - v2.2).abs() < 1e-4 {
                        r.2 = 0.0;
                    } else {
                        r.2 /= v3.2 - v2.2;
                    }

                    let l = Vec3(1.0, 1.0, 1.0) - l;

                    viewports[i].ctl = Vec6(0.58, l.0, 0.58, l.1, 0.58, l.2);
                    viewports[i].ctr = Vec6(0.42, r.0, 0.42, r.1, 0.42, r.2);
                }
            }
        }
    }
}

pub fn interpolate_viewport(viewports: &Vec<Viewport>, t: f32) -> Option<Vec3> {
    if viewports.is_empty() {
        return None;
    }

    let len = viewports.len();

    if len == 1 || t < viewports[0].t {
        let v = &viewports[0];
        return Some(Vec3(v.cx, v.cy, v.width));
    }

    if t > viewports[len - 1].t {
        let v = &viewports[len - 1];
        return Some(Vec3(v.cx, v.cy, v.width));
    }

    for i in 0..len - 1 {
        let v1 = &viewports[i];
        let v2 = &viewports[i + 1];
        let dt = v2.t - v1.t;

        if t >= v1.t && t <= v2.t {
            if let ViewportSmooth::Freeze = v1.smooth {
                return Some(Vec3(v1.cx, v1.cy, v1.width));
            }

            if dt < 1e-4 {
                return Some(Vec3(v2.cx, v2.cy, v2.width));
            }

            let ratio = (t - v1.t) / dt;

            let tx = cubic_bezier(ratio, v1.ctr.0, v1.ctr.1, v2.ctl.0, v2.ctl.1);
            let ty = cubic_bezier(ratio, v1.ctr.2, v1.ctr.3, v2.ctl.2, v2.ctl.3);
            let tw = cubic_bezier(ratio, v1.ctr.4, v1.ctr.5, v2.ctl.4, v2.ctl.5);

            return Some(
                Vec3(tx, ty, tw) * (Vec3(v2.cx, v2.cy, v2.width) - Vec3(v1.cx, v1.cy, v1.width))
                    + Vec3(v1.cx, v1.cy, v1.width),
            );
        }
    }

    None
}
