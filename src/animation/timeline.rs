use std::collections::HashMap;

#[derive(Debug)]
pub struct Timeline {
    /// event-id => absolute time
    pub events: HashMap<String, f64>,
    pub start: Properties,
    pub end: Properties,
}

#[derive(Debug)]
pub struct Properties;

impl Timeline {
    pub fn new(events: HashMap<String, f64>) -> Self {
        Timeline {
            events,
            start: Properties,
            end: Properties,
        }
    }

    pub fn interpolate(&self, time: f64) -> Properties {
        // This method would contain logic to interpolate properties based on the timeline
        // For now, we return an empty Properties instance
        Properties
    }
}
