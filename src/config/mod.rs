use core::f32;
use std::collections::{HashMap, HashSet};

use itertools::multizip;
use serde::Deserialize;
use skia_safe::{Color, Path, PathMeasure};

use crate::{
    animation::{easing::Easing, properties::Properties, timeline::Timeline, track, track_on_path},
    config::{
        event::Event,
        group::Group,
        viewport::{TimeKey, ViewportTrack},
    },
    render::viewport::{Viewport, interpolate_viewport},
    svg::element::{Object, Shape},
    util::{
        COLOR_DICT, ChronuxError, ChronuxResult, interpolator::PchipInterpolator, vector::Vec3,
        version::is_compatible,
    },
};

pub mod event;
pub mod group;
pub mod viewport;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub input: InputConfig,
    pub output: OutputConfig,
    pub drawing: DrawingConfig,
    #[serde(deserialize_with = "deserialize_events")]
    pub events: HashMap<String, Event>,
    pub groups: Vec<Group>,
    pub viewports: Vec<Viewport>,
    #[serde(rename = "vp-tracks")]
    pub viewport_tracks: Vec<ViewportTrack>,
    #[serde(skip)]
    pub vp_pos_inter: Vec<PchipInterpolator>,
    #[serde(skip)]
    pub vp_size_inter: Vec<PchipInterpolator>,
    #[serde(skip)]
    pub vp_tracks: Vec<Path>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InputConfig {
    pub version: String,
    pub svg: String,
    #[serde(default = "HashSet::new")]
    pub pinned: HashSet<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OutputConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    #[serde(rename = "time-start", default = "default_output_time")]
    pub time_start: f32,
    #[serde(rename = "time-end", default = "default_output_time")]
    pub time_end: f32,
    #[serde(deserialize_with = "crate::util::deserialize_hex_color")]
    #[serde(rename = "background-color")]
    pub background_color: Color,
}

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

fn default_output_time() -> f32 {
    f32::NAN
}

fn default_ascii_font() -> String {
    "sans-serif".to_string()
}

impl Config {
    pub fn load(filename: &str) -> ChronuxResult<Config> {
        let config_data =
            std::fs::read_to_string(filename).map_err(|e| ChronuxError::IoError(e))?;
        let config: Config =
            toml::from_str(&config_data).map_err(|e| ChronuxError::ConfigError(e))?;

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
            .expect(&format!("no entry found for event_id = '{}'", event_id))
            .depends
            .iter()
        {
            let t = self.dfs(next, visited, stack, objs, objmap)?;
            after_start_time = after_start_time.max(t + self.events[next].duration);
            with_start_time = with_start_time.max(t);

            if let event::Offset::Track { .. } = self.events[event_id].offset {
                let nextobj = &objs[*objmap
                    .get(&self.events[next].object)
                    .expect(&format!("No entry found for event: {}", next))];
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

    pub fn extract_objects(&self) -> ChronuxResult<HashSet<String>> {
        let mut objects = HashSet::new();

        for event_id in self.events.keys() {
            let object_id = &self.events[event_id].object;

            if !objects.contains(object_id) {
                objects.insert(object_id.clone());
            }
        }

        for vp_tracks in self.viewport_tracks.iter() {
            if !objects.contains(&vp_tracks.tracked_path) {
                objects.insert(vp_tracks.tracked_path.clone());
            }
        }

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

    pub fn calc_viewport_tracks(&mut self, objs: &Vec<Object>) -> ChronuxResult<()> {
        let objmap = objs
            .iter()
            .enumerate()
            .map(|(i, o)| (o.id.clone(), i))
            .collect::<HashMap<_, _>>();

        for viewport in self.viewport_tracks.drain(..) {
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
                .into_iter()
                .enumerate()
                .map(|(id, k)| {
                    let (dep, time, duration_coeff) = match k.time {
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
                            "Viewport keyframe {} has no position defined",
                            id
                        )));
                    };

                    if let Some(ev) = self.events.get(&dep) {
                        let time = ev.start + time + ev.duration * duration_coeff;
                        Ok((position, k.size, time))
                    } else {
                        Err(ChronuxError::BrokenDependency(format!(
                            "a viewport keyframe depends on non-existent event {}",
                            dep
                        )))
                    }
                })
                .flatten()
                .collect::<Vec<_>>();

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
                        "For each viewport track, size must be defined for the first keyframe even if it is not changed since previous track end: {:?}",
                        size_seq
                    )));
                }
            }
            let time_seq = keyframes
                .iter()
                .map(|(_, _, time)| *time)
                .collect::<Vec<_>>();

            let pos_pchip = PchipInterpolator::new(time_seq.clone(), pos_seq)?;
            let size_pchip = PchipInterpolator::new(time_seq, new_size_seq)?;

            self.vp_pos_inter.push(pos_pchip);
            self.vp_size_inter.push(size_pchip);
            self.vp_tracks.push(d.clone());
        }
        Ok(())
    }

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
                let pos = pm.pos_tan(pos).unwrap().0;
                return Ok(Vec3(pos.x, pos.y, size));
            }
        }

        // If no tracks are available, try to interpolate between tracks
        for w in vp_zip.windows(2) {
            let (pos1, size1, path1) = w[0];
            let (pos2, size2, path2) = w[1];

            let te1 = pos1.t_end;
            let ts2 = pos2.t_start;

            if te1 <= t && t <= ts2 {
                let p = (t - te1) / (ts2 - te1);
                let mut pm1 = PathMeasure::new(path1, false, Some(2.0));
                let mut pm2 = PathMeasure::new(path2, false, Some(2.0));
                let path1_len = pm1.length();
                let p1 = pm1.pos_tan(path1_len).unwrap().0;
                let p2 = pm2.pos_tan(0.0).unwrap().0;
                let pos = p1 + (p2 - p1) * p;
                let size = size1.p_end + (size2.p_start - size1.p_end) * p;
                return Ok(Vec3(pos.x, pos.y, size));
            }
        }

        // If no tracks are available, use the legacy viewport keyframes
        // Return an error if still failed
        interpolate_viewport(&self.viewports, t).ok_or(ChronuxError::ArgError(format!(
            "Viewport interpolation failed for time {}",
            t
        )))
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
                        "cannot parse color literal '{}' for color key '{}': {}",
                        v,
                        k,
                        e.to_string()
                    ))
                })?
                .to_rgba8();
            Ok((k, Color::from_argb(a, r, g, b)))
        })
        .flatten()
        .collect();

    COLOR_DICT
        .set(colors)
        .map_err(|_| serde::de::Error::custom("Failed to set COLOR_DICT"))?;

    Ok(())
}
