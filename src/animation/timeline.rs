use std::collections::HashMap;

use crate::{animation::properties::Properties, config::event::Event, svg::element::Shape};

#[derive(Debug, Clone)]
pub struct Timeline {
    /// event-id => absolute time
    pub events: Vec<Event>,
    pub start: Properties,
    pub object: String,
}

impl Timeline {
    pub fn new(events: Vec<Event>, start: Properties, object: String) -> Option<Self> {
        if events.is_empty() {
            return None;
        }

        let mut events = events;
        events.sort_by(|a, b| {
            a.start
                .partial_cmp(&b.start)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Some(Timeline {
            events,
            start,
            object,
        })
    }

    pub fn interpolate(
        &self,
        time: f32,
        dict: &HashMap<String, f32>,
        shape_dict: &HashMap<String, Shape>,
    ) -> Properties {
        let mut result = self.start.clone();

        for event in &self.events {
            if time > event.start + event.duration {
                // Apply the properties of the event at this time
                result = result.apply(&event, 1.0, &self.object, dict, shape_dict);
            } else if time > event.start {
                // Interpolate between start and end of the event
                let t = (time - event.start) / event.duration;
                result = result.apply(
                    &event,
                    event.easing.evaluate(t),
                    &self.object,
                    dict,
                    shape_dict,
                );
            }
        }

        result
    }
}
