use std::collections::HashMap;

use skia_safe::{Canvas, Rect};

use crate::{
    animation::timeline::Timeline,
    config::MinimapConfig,
    render::{self, get_bounding_box},
    svg::element::Object,
    util::{ChronuxResult, parse_css_color},
    widget::{PANEL_BACKGROUND_COLOR, PANEL_BORDER_COLOR, PANEL_BORDER_WIDTH, WIDGET_CANVAS_WIDTH},
};

pub fn draw(
    canvas: &Canvas,
    minimap: &MinimapConfig,
    timeline: &HashMap<String, Timeline>,
    objs: &Vec<Object>,
    t: f32,
) -> ChronuxResult<()> {
    let mut bounding_box: Option<(f32, f32, f32, f32)> = None;
    for o in objs.iter() {
        if minimap.paths.contains(&o.id) {
            let properties = match timeline.get(&o.id) {
                Some(tl) => tl.interpolate(t),
                None => o.style.clone(),
            };
            let ret_bb = get_bounding_box(&o.shape, &properties);
            if let Some((bl, bt, br, bb)) = bounding_box && let Some(ret_bb) = ret_bb {
                bounding_box = Some((
                    bl.min(ret_bb.0),
                    bt.min(ret_bb.1),
                    br.max(ret_bb.2),
                    bb.max(ret_bb.3),
                ));
            } else if bounding_box.is_none() {
                bounding_box = ret_bb;
            }
        }
    }

    if bounding_box.is_none() {
        // 小地图为空
        return Ok(());
    }

    let canvas_restore_count = canvas.save();

    let bb = bounding_box.unwrap();
    let (bw, bh) = (bb.2 - bb.0, bb.3 - bb.1);

    let b_size = bw.max(bh);

    let minimap_size = minimap.size - minimap.padding * 2.0; // 小地图的大小
    let scale = minimap_size / b_size;

    let (bl, bt) = if bw > bh {
        (bb.0, bb.1 - (bw - bh) / 2.0)
    } else {
        (bb.0 - (bh - bw) / 2.0, bb.1)
    };

    let mut fill = skia_safe::Paint::default();
    fill.set_anti_alias(true);
    fill.set_color(parse_css_color(PANEL_BACKGROUND_COLOR, 1.0)?);
    fill.set_style(skia_safe::PaintStyle::Fill);

    let mut stroke = skia_safe::Paint::default();
    stroke.set_anti_alias(true);
    stroke.set_color(parse_css_color(PANEL_BORDER_COLOR, 1.0)?);
    stroke.set_stroke_width(PANEL_BORDER_WIDTH);
    stroke.set_style(skia_safe::PaintStyle::Stroke);

    let minimap_rect = Rect::from_xywh(
        WIDGET_CANVAS_WIDTH - minimap_size - minimap.padding * 2.0,
        -minimap.corner_radius * 2.0, // 稍微多一点，避免圆角边缘在画面可见部分中露出
        minimap_size + minimap.padding * 2.0 + minimap.corner_radius * 2.0,
        minimap_size + minimap.padding * 2.0 + minimap.corner_radius * 2.0,
    );

    canvas.draw_round_rect(
        minimap_rect,
        minimap.corner_radius,
        minimap.corner_radius,
        &fill,
    );
    canvas.draw_round_rect(
        minimap_rect,
        minimap.corner_radius,
        minimap.corner_radius,
        &stroke,
    );

    canvas
        .translate((WIDGET_CANVAS_WIDTH - minimap_size - minimap.padding, minimap.padding))
        .scale((scale, scale))
        .translate((-bl, -bt));

    for o in objs.iter() {
        if minimap.paths.contains(&o.id) {
            let properties = match timeline.get(&o.id) {
                Some(tl) => tl.interpolate(t),
                None => o.style.clone(),
            };
            render::draw(canvas, &o.shape, &properties)?;
        }
    }

    // 恢复坐标变换
    canvas.restore_to_count(canvas_restore_count);

    Ok(())
}
