use core::f32;
use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use skia_safe::Color;

use crate::{
    animation::{properties::Properties, timeline::Timeline, track},
    config::{event::Event, group::Group},
    render::viewport::Viewport,
    svg::element::Object,
    util::{version::is_compatible, ChronuxError, ChronuxResult, COLOR_DICT},
};

pub mod event;
pub mod group;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub input: InputConfig,
    pub output: OutputConfig,
    pub drawing: DrawingConfig,
    #[serde(deserialize_with = "deserialize_events")]
    pub events: HashMap<String, Event>,
    pub groups: Vec<Group>,
    pub viewports: Vec<Viewport>,
}

#[derive(Debug, Deserialize)]
pub struct InputConfig {
    pub version: String,
    pub svg: String,
    #[serde(default = "Vec::new")]
    pub pinned: Vec<String>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct DrawingConfig {
    #[serde(rename = "colors")]
    #[serde(deserialize_with = "deserialize_color_dict")]
    _colors: (),
    /// OSM zoom level
    /// If the scale is drawn, both `zoom` and `lat` must be set.
    pub zoom: Option<f32>,
    /// Latitude in degrees to calculate scale
    /// If the scale is drawn, both `zoom` and `lat` must be set.
    pub lat: Option<f32>,
    pub minimap: Option<MinimapConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MinimapConfig {
    pub paths: HashSet<String>,
    #[serde(rename = "min-scale", default = "default_min_scale")]
    pub min_scale: f32,
    #[serde(rename = "max-scale", default = "default_max_scale")]
    pub max_scale: f32,
    pub padding: f32,
    #[serde(rename = "corner-radius", default = "default_zero")]
    pub corner_radius: f32,
    #[serde(rename = "minimap-size")]
    pub size: f32,
}

fn default_zero() -> f32 {
    0.0
}

fn default_output_time() -> f32 {
    f32::NAN
}

fn default_min_scale() -> f32 {
    1.0
}

fn default_max_scale() -> f32 {
    f32::INFINITY
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

        for next in self.events[event_id].depends.iter() {
            let t = self.dfs(next, visited, stack, objs, objmap)?;
            after_start_time = after_start_time.max(t + self.events[next].duration);
            with_start_time = with_start_time.max(t);

            if let event::Offset::Track { .. } = self.events[event_id].offset {
                let nextobj = &objs[objmap[&self.events[next].object]];
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
            .map(|(object_id, events)| (object_id, Timeline::new(events, Properties::default())))
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

    COLOR_DICT.set(colors)
        .map_err(|_| serde::de::Error::custom("Failed to set COLOR_DICT"))?;

    Ok(())
}
