use skia_safe::Color4f;

use crate::util::TRANSPARENT;

#[derive(Debug, Clone)]
pub enum ColorPaint {
    Color(Color4f),
    Href(String),
}

impl Default for ColorPaint {
    fn default() -> Self {
        ColorPaint::Color(TRANSPARENT)
    }
}
