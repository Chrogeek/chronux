use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use skia_safe::{
    Font, FontHinting, FontStyle,
    font::Edging,
    font_style::{Slant, Width},
};

use crate::util::{ChronuxError, ChronuxResult};

#[derive(Hash, Debug, Clone, PartialEq, Eq)]
pub struct TypefaceMatcher {
    pub font_family: String,
    pub font_size: i32,
    pub font_weight: i32,
}

static FONT_CACHE: LazyLock<RwLock<HashMap<TypefaceMatcher, Font>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn get_font(font_family: &str, font_size: f32, font_weight: i32) -> ChronuxResult<Font> {
    let matcher = TypefaceMatcher {
        font_family: font_family.to_string(),
        font_size: font_size as i32,
        font_weight,
    };

    if let Some(font) = FONT_CACHE
        .read()
        .map_err(|_| ChronuxError::SkiaError("Failed to load font cache".to_string()))?
        .get(&matcher)
    {
        return Ok(font.clone());
    }

    let mgr = skia_safe::FontMgr::new();
    let tf = mgr
        .match_family_style(
            &font_family,
            FontStyle::new(font_weight.into(), Width::NORMAL, Slant::Upright),
        )
        .ok_or(ChronuxError::SkiaError(format!(
            "failed to create typeface '{}'",
            font_family
        )))?;
    let mut font = Font::from_typeface(tf, font_size as f32);
    font.set_subpixel(true);
    font.set_hinting(FontHinting::Slight);
    font.set_edging(Edging::SubpixelAntiAlias);

    FONT_CACHE
        .write()
        .map_err(|_| ChronuxError::SkiaError("Failed to write to font cache".to_string()))?
        .insert(matcher.clone(), font.clone());

    Ok(font)
}
