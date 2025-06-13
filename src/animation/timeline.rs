use crate::{animation::{properties::Properties, Animation}, config::event::Event};

#[derive(Debug)]
pub struct Timeline {
    /// event-id => absolute time
    pub events: Vec<Event>,
    pub start: Properties,
}

impl Timeline {
    pub fn new(events: Vec<Event>, start: Properties) -> Self {
        let mut events = events;
        events.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));

        let mut start = start;

        for e in events.iter() {
            match e.animation {
                // If the event has a trim animation, we apply it to the start properties
                Animation::TrimPath { .. } => {
                    start.trim = (0.0, 1.0);
                }
                // Add other animation types here as needed
                _ => {}
            }
        }

        Timeline {
            events,
            start,
        }
    }

    pub fn interpolate(&self, time: f64) -> Properties {
        // This method would contain logic to interpolate properties based on the timeline

        let mut result = self.start.clone();

        for event in &self.events {
            if time > event.start + event.duration {
                // Apply the properties of the event at this time
                // This is a placeholder; actual implementation would depend on the event's properties
                result = result.apply(&event.animation, 1.0);
            } else if time > event.start {
                // Interpolate between start and end of the event
                let t = (time - event.start) / event.duration;
                result = result.apply(&event.animation, t);
            }
        }

        result
    }
}
