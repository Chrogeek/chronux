use crate::{animation::properties::Properties, config::event::Event};

#[derive(Debug)]
pub struct Timeline {
    /// event-id => absolute time
    pub events: Vec<Event>,
    pub start: Properties,
}

impl Timeline {
    pub fn new(events: Vec<Event>, start: Properties) -> Self {
        let mut events = events;
        events.sort_by(|a, b| {
            a.start
                .partial_cmp(&b.start)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Timeline { events, start }
    }

    pub fn interpolate(&self, time: f32) -> Properties {
        let mut result = self.start.clone();

        for event in &self.events {
            if time > event.start + event.duration {
                // Apply the properties of the event at this time
                result = result.apply(&event.animation, 1.0);
            } else if time > event.start {
                // Interpolate between start and end of the event
                let t = (time - event.start) / event.duration;
                result = result.apply(&event.animation, event.easing.evaluate(t));
            }
        }

        result
    }
}
