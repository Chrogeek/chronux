use core::f32;
use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use serde_with::serde_as;
use skia_safe::{Color, PathMeasure};

use crate::{
    animation::{easing::Easing, properties::Properties, timeline::Timeline, track, track_on_path},
    config::{
        counter::{update_counters, Counters}, dates::DatesConfig, event::Event, extra_stroke::ExtraStrokeConfig, group::Group, input::InputConfig, output::OutputConfig, viewport::{TimeKey, ViewportSequence, ViewportTrack}
    },
    svg::element::{Object, Shape},
    util::{
        interpolator::PchipInterpolator, version::is_compatible, ChronuxError, ChronuxResult, COLOR_DICT
    },
};

pub mod counter;
pub mod dates;
pub mod event;
pub mod group;
pub mod input;
pub mod output;
pub mod viewport;
pub mod extra_stroke;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub input: InputConfig,
    pub output: OutputConfig,
    pub drawing: DrawingConfig,
    #[serde(deserialize_with = "deserialize_events")]
    pub events: HashMap<String, Event>,
    pub groups: Vec<Group>,
    #[serde(rename = "viewports")]
    pub viewport_tracks: Vec<ViewportTrack>,
}

#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub struct DrawingConfig {
    #[serde(rename = "colors")]
    #[serde(deserialize_with = "deserialize_color_dict")]
    _colors: (),
    pub scale: Option<ScaleConfig>,
    pub minimap: Option<MinimapConfig>,
    #[serde(rename = "debug-timecode")]
    pub debug_timecode: bool,
    #[serde(rename = "ascii-font", default = "default_ascii_font")]
    pub ascii_font: String,
    #[serde(rename = "sum-counters", default)]
    pub sum_counters: HashMap<String, Vec<String>>,
    #[serde_with(as = "OneOrMany<_, PreferOne>")]
    pub phantom: HashSet<String>,
    pub dates: Option<DatesConfig>,
    #[serde(rename = "extra-stroke")]
    pub extra_stroke: Option<ExtraStrokeConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MinimapConfig {
    pub paths: HashSet<String>,
    pub padding: f32,
    #[serde(rename = "corner-radius", default = "default_zero")]
    pub corner_radius: f32,
    #[serde(rename = "minimap-size")]
    pub size: f32,
    #[serde(rename = "min-stroke-width", default = "default_stroke_width")]
    pub min_stroke_width: f32,
}

fn default_stroke_width() -> f32 {
    5.0
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScaleConfig {
    /// OSM zoom level
    /// If the scale is drawn, both `zoom` and `lat` must be set.
    pub zoom: f32,
    /// Latitude in degrees to calculate scale
    /// If the scale is drawn, both `zoom` and `lat` must be set.
    pub lat: f32,
    pub padding: f32,
}

fn default_zero() -> f32 {
    0.0
}

fn default_ascii_font() -> String {
    "sans-serif".to_string()
}

impl Config {
    pub fn load(filename: &str) -> ChronuxResult<Config> {
        let config_data = std::fs::read_to_string(filename).map_err(ChronuxError::IoError)?;
        let config: Config = toml::from_str(&config_data).map_err(ChronuxError::ConfigError)?;

        if !is_compatible(&config.input.version) {
            return Err(ChronuxError::VersionError(config.input.version.clone()));
        }
        Ok(config)
    }

    fn dfs(
        &self,
        event_id: &String,
        visited: &mut HashMap<String, f32>,
        stack: &mut HashSet<String>,
        objs: &Vec<Object>,
        objmap: &HashMap<String, usize>,
    ) -> ChronuxResult<f32> {
        let mut after_start_time = 0.0f32;
        let mut with_start_time = 0.0f32;
        let mut track_start_time = 0.0f32;

        if stack.contains(event_id) {
            return Err(ChronuxError::CircularDependency(event_id.clone()));
        }

        if visited.contains_key(event_id) {
            return Ok(visited[event_id]);
        }

        stack.insert(event_id.clone());

        for next in self
            .events
            .get(event_id)
            .unwrap_or_else(|| panic!("no entry found for event_id = '{event_id}'"))
            .depends
            .iter()
        {
            let t = self.dfs(next, visited, stack, objs, objmap)?;
            after_start_time = after_start_time.max(t + self.events[next].duration);
            with_start_time = with_start_time.max(t);

            if let event::Offset::Track { .. } = self.events[event_id].offset {
                let nextobj = &objs[*objmap.get(&self.events[next].object).unwrap_or_else(|| {
                    panic!(
                        "Object for event {} not found ({}, failed for event tracking)",
                        next, self.events[next].object
                    )
                })];
                let obj = &objs[objmap[&self.events[event_id].object]];
                let (x, y) = obj.get_position();
                track_start_time = t + track(nextobj, &self.events[next], x, y)?;
            }
        }

        let start_time = match self.events[event_id].offset {
            event::Offset::WithPrevious { delay } => with_start_time + delay,
            event::Offset::AfterPrevious { offset } => after_start_time + offset,
            event::Offset::Track { track: offset } => track_start_time + offset,
        };

        stack.remove(event_id);
        visited.insert(event_id.clone(), start_time);
        Ok(start_time)
    }

    pub fn calc_start_times(&mut self, objs: &Vec<Object>) -> ChronuxResult<()> {
        let mut visited = HashMap::new();
        let mut stack = HashSet::new();

        let objmap = objs
            .iter()
            .enumerate()
            .map(|(i, o)| (o.id.clone(), i))
            .collect::<HashMap<_, _>>();

        for event_id in self.events.keys() {
            if !visited.contains_key(event_id) {
                self.dfs(event_id, &mut visited, &mut stack, objs, &objmap)?;
            }
        }

        for (event_id, start_time) in visited.iter() {
            let event = self.events.get_mut(event_id).unwrap();
            event.start = *start_time;
            log::debug!(
                "Event {}: start time = {:.2}, duration = {:.2}",
                event_id,
                event.start,
                event.duration
            );
        }

        Ok(())
    }

    pub fn calc_timeline(&self) -> ChronuxResult<HashMap<String, Timeline>> {
        let mut timeline = HashMap::new();

        for event_id in self.events.keys() {
            let object_id = &self.events[event_id].object;

            if !timeline.contains_key(object_id) {
                timeline.insert(object_id.clone(), Vec::new());
            }

            let event = &self.events[event_id];
            timeline.get_mut(object_id).unwrap().push(event.clone());
        }

        let timeline = timeline
            .into_iter()
            .filter_map(|(object_id, events)| {
                Timeline::new(events, Properties::default()).map(|tl| (object_id, tl))
            })
            .collect();

        Ok(timeline)
    }

    /// 提取所有出现在 events、viewport_tracks 和 dates 中的对象 ID
    pub fn extract_objects(&self) -> ChronuxResult<HashSet<String>> {
        let mut objects = HashSet::new();

        for event_id in self.events.keys() {
            objects.insert(self.events[event_id].object.clone());
        }

        for vp_tracks in self.viewport_tracks.iter() {
            objects.insert(vp_tracks.tracked_path.clone());
        }

        if let Some(dates) = &self.drawing.dates {
            objects.insert(dates.position_date.clone());
            objects.insert(dates.position_year.clone());
            objects.insert(dates.area_date.clone());
            objects.insert(dates.area_year.clone());
        }

        objects.extend(self.input.pinned.iter().cloned());

        Ok(objects)
    }

    pub fn flatten_groups(&mut self) -> ChronuxResult<()> {
        for group in self.groups.drain(..) {
            let events = group.to_events();
            for event in events {
                if self.events.contains_key(&event.id) {
                    return Err(ChronuxError::ConfigError(serde::de::Error::custom(
                        format!("Duplicate event ID: {}", event.id),
                    )));
                }
                self.events.insert(event.id.clone(), event);
            }
        }

        Ok(())
    }

    pub fn calc_viewport_tracks(&self, objs: &[Object]) -> ChronuxResult<ViewportSequence> {
        let objmap = objs
            .iter()
            .enumerate()
            .map(|(i, o)| (o.id.clone(), i))
            .collect::<HashMap<_, _>>();

        let mut seq = ViewportSequence::default();

        for viewport in &self.viewport_tracks[..] {
            let tracked_path =
                &objs[*objmap
                    .get(&viewport.tracked_path)
                    .ok_or(ChronuxError::ArgError(format!(
                        "Viewport track reference object does not exist: {}",
                        viewport.tracked_path
                    )))?];
            let d = match &tracked_path.shape {
                Shape::Path { d } => d,
                _ => {
                    return Err(ChronuxError::ArgError(format!(
                        "Tracked object for viewport_track is not a Path: {}",
                        viewport.tracked_path
                    )));
                }
            };
            let num_keyframes = viewport.keyframes.len();
            let keyframes = viewport
                .keyframes
                .iter()
                .enumerate()
                .map(|(id, k)| {
                    let (dep, time, duration_coeff) = match &k.time {
                        TimeKey::WithPrevious { depends, delay } => (depends, delay, 0.0),
                        TimeKey::AfterPrevious { depends, offset } => (depends, offset, 1.0),
                    };

                    let position = if let Some(kx) = k.x
                        && let Some(ky) = k.y
                    {
                        let (pt, len) = track_on_path(d, kx, ky, &Easing::Linear, 0.0);
                        // total_time=0.0 since in this case (not animating) we don't care it
                        pt * len
                    } else if id == 0 {
                        0.0
                    } else if id == num_keyframes - 1 {
                        let mut pm = PathMeasure::new(d, false, Some(2.0));
                        pm.length()
                    } else {
                        return Err(ChronuxError::ArgError(format!(
                            "Viewport keyframe {id} has no position defined"
                        )));
                    };

                    if let Some(ev) = self.events.get(dep) {
                        let time = ev.start + time + ev.duration * duration_coeff;
                        Ok((position, k.size, time))
                    } else {
                        Err(ChronuxError::BrokenDependency(format!(
                            "a viewport keyframe depends on non-existent event {dep}"
                        )))
                    }
                })
                .collect::<ChronuxResult<Vec<_>>>()?;

            let pos_seq = keyframes.iter().map(|(pos, _, _)| *pos).collect::<Vec<_>>();
            let size_seq = keyframes
                .iter()
                .map(|(_, size, _)| *size)
                .collect::<Vec<_>>();
            let mut new_size_seq = Vec::new();
            for s in size_seq.iter() {
                if let Some(s) = s {
                    new_size_seq.push(*s);
                } else if let Some(last) = new_size_seq.last() {
                    new_size_seq.push(*last);
                } else {
                    return Err(ChronuxError::ArgError(format!(
                        "For each viewport track, size must be defined for the first keyframe even if it is not changed since previous track end: {size_seq:?}"
                    )));
                }
            }
            let time_seq = keyframes
                .iter()
                .map(|(_, _, time)| *time)
                .collect::<Vec<_>>();

            let pos_pchip =
                PchipInterpolator::new(time_seq.clone(), pos_seq).inspect_err(|_e| {
                    log::error!(
                        "Failed to create position PchipInterpolator for viewport track on {})",
                        tracked_path.id,
                    );
                })?;
            let size_pchip = PchipInterpolator::new(time_seq, new_size_seq).inspect_err(|_e| {
                log::error!(
                    "Failed to create size PchipInterpolator for viewport track on {}",
                    tracked_path.id,
                );
            })?;

            seq.vp_pos_inter.push(pos_pchip);
            seq.vp_size_inter.push(size_pchip);
            seq.vp_tracks.push(d.clone());
            seq.vp_track_paths.push(viewport.tracked_path.clone());
        }

        for i in 1..seq.vp_pos_inter.len() {
            // 检查相邻两个插值的时间区间是否交叉
            let (p0, p1) = (&seq.vp_pos_inter[i - 1], &seq.vp_pos_inter[i]);
            if p1.t_start < p0.t_end {
                log::warn!(
                    "Viewport tracks time interval intersection detected between track '{} (t_end = {})' and '{} (t_start = {})'",
                    seq.vp_track_paths[i - 1],
                    p0.t_end,
                    seq.vp_track_paths[i],
                    p1.t_start,
                );
                log::warn!("This may cause unexpected behavior for viewport move.");
                log::warn!(
                    "You may want to adjust the keyframes or time intervals to avoid this issue."
                );
            }
        }
        Ok(seq)
    }

    pub fn get_counters(&self, time: f32) -> Counters {
        let mut cnt = Counters::default();
        for event in self.events.values() {
            if time > event.start + event.duration {
                for (k, v) in &event.counters {
                    update_counters(&mut cnt, k, *v);
                }
            } else if time > event.start {
                let t = (time - event.start) / event.duration;
                for (k, v) in &event.counters {
                    update_counters(&mut cnt, k, v * event.easing.evaluate(t, event.duration));
                }
            }
        }
        cnt
    }
}

fn deserialize_events<'de, D>(deserializer: D) -> Result<HashMap<String, Event>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mut events = HashMap::new();
    for item in Vec::<Event>::deserialize(deserializer)? {
        if events.contains_key(&item.id) {
            return Err(serde::de::Error::custom(format!(
                "Duplicate event ID: {}",
                item.id
            )));
        }

        events.insert(item.id.clone(), item);
    }
    Ok(events)
}

fn deserialize_color_dict<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    let colors = HashMap::<String, String>::deserialize(deserializer)?
        .into_iter()
        .map(|(k, v)| -> ChronuxResult<(String, Color)> {
            let [r, g, b, a] = csscolorparser::parse(&v)
                .map_err(|e| {
                    ChronuxError::ColorParseError(format!(
                        "cannot parse color literal '{v}' for color key '{k}': {e}"
                    ))
                })?
                .to_rgba8();
            Ok((k, Color::from_argb(a, r, g, b)))
        })
        .collect::<Result<HashMap<_, _>, _>>()
        .map_err(|e| serde::de::Error::custom(format!("Failed to parse color dictionary: {e}")))?;

    COLOR_DICT
        .set(colors)
        .map_err(|_| serde::de::Error::custom("Failed to set COLOR_DICT"))?;

    Ok(())
}
