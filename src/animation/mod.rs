use std::collections::HashMap;

use serde::Deserialize;
use skia_safe::{Path, PathMeasure};

use crate::{
    animation::{easing::Easing, tween::Tween},
    config::{counter::Counters, event::Event},
    parser::chain::ChainedPath,
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
    #[serde(rename = "none")]
    None,
    #[serde(rename = "trim-path")]
    TrimPath {
        #[serde(default)]
        reverse: bool,
    },
    #[serde(rename = "trim-path-out")]
    TrimPathOut {
        #[serde(default)]
        reverse: bool,
    },
    #[serde(rename = "typewriter")]
    Typewriter {
        #[serde(default)]
        reverse: bool,
    },
    #[serde(rename = "typewriter-out")]
    TypewriterOut {
        #[serde(default)]
        reverse: bool,
    },
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
    #[serde(rename = "move")]
    Move { dx: f32, dy: f32 },
    #[serde(rename = "stretch-in")]
    StretchIn,
    #[serde(rename = "stretch-out")]
    StretchOut,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TaggedAnimation {
    #[serde(rename = "object")]
    pub obj_id: ChainedPath,
    #[serde(flatten)]
    pub animation: Animation,
    #[serde(default)]
    pub counters: Counters,
    #[serde(default)]
    pub easing: Option<Easing>,
}

/// 跟踪路径 `chained_path` 上距离 (x,y) 最近的位置
/// 为保证求解正确，建议 event 的缓动函数为单调函数
/// 返回 (t, dis2)，其中 t 为跟踪位置，dis2 为最近距离的平方
pub fn track(
    chained_path: &ChainedPath,
    objs: &[Object],
    objmap: &HashMap<String, usize>,
    event: &Event,
    x: f32,
    y: f32,
    dict: &HashMap<String, f32>,
) -> ChronuxResult<(f32, f32)> {
    // 先求出不加缓动函数前提下，最近的位置t
    let (t, dis2) = match chained_path {
        ChainedPath::Atom(o) => {
            let obj = &objs[objmap[o]];
            if let Shape::Path { d } = &obj.shape {
                let (t, _, dis2) = track_on_path(d, x, y, &Easing::Linear);
                Ok((t, dis2))
            } else {
                Err(ChronuxError::ArgError(
                    "non-path shapes cannot be tracked".into(),
                ))
            }
        }
        ChainedPath::Seq(chained_paths) => {
            let lengths = chained_paths
                .iter()
                .map(|c| c.length(dict))
                .collect::<Option<Vec<_>>>()
                .expect(&format!(
                    "Cannot get length of chained path '{:?}'",
                    chained_path
                ));
            let total_length = lengths.iter().sum::<f32>();

            let tracks = chained_paths
                .iter()
                .map(|c| track(c, objs, objmap, event, x, y, dict))
                .collect::<ChronuxResult<Vec<_>>>()?;

            // 获取距离最近的点
            let nearest = tracks
                .into_iter()
                .enumerate()
                .reduce(|(acc_id, (acc_t, acc_dis2)), (cur_id, (cur_t, cur_dis2))| {
                    if cur_dis2 <= acc_dis2 {
                        (cur_id, (cur_t, cur_dis2))
                    } else {
                        (acc_id, (acc_t, acc_dis2))
                    }
                })
                .ok_or_else(|| ChronuxError::ArgError("Empty #seq chained path".to_string()))?;

            let pre_length = lengths[..nearest.0].iter().sum::<f32>();
            let t = (pre_length + nearest.1.0 * lengths[nearest.0]) / total_length;
            Ok((t, nearest.1.1))
        }
        ChainedPath::Par(chained_paths) => {
            let tracks = chained_paths
                .iter()
                .map(|c| track(c, objs, objmap, event, x, y, dict))
                .collect::<ChronuxResult<Vec<_>>>()?;

            // 获取距离最近的点
            tracks
                .into_iter()
                .reduce(|(acc_t, acc_dis2), (cur_t, cur_dis2)| {
                    if cur_dis2 <= acc_dis2 {
                        (cur_t, cur_dis2)
                    } else {
                        (acc_t, acc_dis2)
                    }
                })
                .ok_or_else(|| ChronuxError::ArgError("Empty #par chained path".to_string()))
        }
    }?;

    let t = match event.animation {
        Animation::TrimPath { reverse } => Ok(if reverse { 1.0 - t } else { t }),
        Animation::TrimPathOut { reverse } => Ok(if reverse { t } else { 1.0 - t }),
        _ => Err(ChronuxError::ArgError(format!(
            "animation '{:?}' cannot be tracked",
            event.animation
        ))),
    }?;

    // 根据event的缓动函数设置，二分法求出正确的t
    let (mut tl, mut tr, mut tm) = (0.0, 1.0, 0.5);
    while tr - tl > 1e-4 {
        if event.easing.evaluate(tm) < t {
            tl = tm;
        } else {
            tr = tm;
        }
        tm = (tl + tr) / 2.0;
    }

    // 返回跟踪结果
    Ok((tm, dis2))
}

/// Calculate the nearest point on path d to given point (x, y).
/// Returns (t, d, p) where d is the path length and p is the coordinate of nearest point, l is the length from path origin to the nearest point, and t = l / d.
pub fn track_on_path(d: &Path, x: f32, y: f32, easing: &Easing) -> (f32, f32, f32) {
    const SEGMENTS: usize = 20;

    let mut pm = PathMeasure::new(d, false, Some(2.0));
    let length = pm.length();

    let closest = (0..SEGMENTS + 1)
        .map(|i| {
            let p = i as f32 / SEGMENTS as f32;
            let t = easing.evaluate(p);
            let dis = t * length;
            (p, pm.pos_tan(dis).unwrap().0)
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
        let dl = pm.pos_tan(easing.evaluate(ml) * length).unwrap().0;
        let dr = pm.pos_tan(easing.evaluate(mr) * length).unwrap().0;
        let d1 = (dl.x - x).powi(2) + (dl.y - y).powi(2);
        let d2 = (dr.x - x).powi(2) + (dr.y - y).powi(2);
        if d1 < d2 {
            tr = mr;
        } else {
            tl = ml;
        }
    }

    let t = (tl + tr) / 2.0;
    let pt = pm.pos_tan(t * length).unwrap().0;
    let dis2 = (pt.x - x).powi(2) + (pt.y - y).powi(2);

    (t, length, dis2)
}
