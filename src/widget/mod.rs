pub enum Widget {
    LineIcon {
        name: String,
        length: f32,
    },
    Date {
        year: u32,
        day: String,
    },
    SystemStats {
        stations: u32,
        total_length: f32,
    },
    Text {
        text: String,
    }
}

pub struct WidgetMeta {
    pub x: f32,
    pub y: f32,
}

pub struct Panel {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub const PANEL_BACKGROUND_COLOR: &str = "#F2F2F2";
pub const PANEL_BORDER_COLOR: &str = "#EDEDED";
pub const PANEL_BORDER_WIDTH: f32 = 5.0;
