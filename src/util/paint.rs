use skia_safe::Color;

#[derive(Debug, Clone)]
pub enum ColorPaint {
    Color(Color),
    Href(String),
}

impl Default for ColorPaint {
    fn default() -> Self {
        ColorPaint::Color(Color::TRANSPARENT)
    }
}
