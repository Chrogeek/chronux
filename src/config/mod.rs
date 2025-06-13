use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::{
    animation::timeline::Timeline,
    config::event::Event,
    util::{ChronuxError, ChronuxResult},
};

pub mod event;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub input: InputConfig,
    pub output: OutputConfig,
    #[serde(deserialize_with = "deserialize_events")]
    pub events: HashMap<String, Event>,
}

#[derive(Debug, Deserialize)]
pub struct InputConfig {
    pub svg: String,
    pub map: String,
}

#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration: f64,
}

impl Config {
    pub fn load(filename: &str) -> ChronuxResult<Config> {
        let config_data =
            std::fs::read_to_string(filename).map_err(|e| ChronuxError::IoError(e))?;
        let mut config: Config =
            toml::from_str(&config_data).map_err(|e| ChronuxError::ConfigError(e))?;
        config.calc_start_times()?;
        Ok(config)
    }

    fn dfs(
        &self,
        event_id: &String,
        visited: &mut HashMap<String, f64>,
        stack: &mut HashSet<String>,
    ) -> ChronuxResult<f64> {
        let mut start_time = 0.0f64;

        if visited.contains_key(event_id) {
            return Ok(visited[event_id]);
        }

        if stack.contains(event_id) {
            return Err(ChronuxError::CircularDependency(event_id.clone()));
        }

        stack.insert(event_id.clone());

        for next in self.events[event_id].depends.iter() {
            let t = self.dfs(next, visited, stack)?;
            start_time = start_time.max(t + self.events[next].duration);
        }

        start_time += self.events[event_id].offset;

        stack.remove(event_id);

        visited.insert(event_id.clone(), start_time);

        Ok(start_time)
    }

    fn calc_start_times(&mut self) -> ChronuxResult<()> {
        let mut visited = HashMap::new();
        let mut stack = HashSet::new();

        for event_id in self.events.keys() {
            if !visited.contains_key(event_id) {
                self.dfs(event_id, &mut visited, &mut stack)?;
            }
        }

        for (event_id, start_time) in visited.iter() {
            let event = self.events.get_mut(event_id).unwrap();
            event.start = *start_time;
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
            .map(|(object_id, events)| (object_id, Timeline::new(events)))
            .collect();

        Ok(timeline)
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
