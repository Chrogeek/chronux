use std::collections::HashMap;

use skia_safe::{Canvas, ClipOp, RRect, Rect};

use crate::{
    animation::timeline::Timeline,
    config::{MinimapAnchor, MinimapConfig, counter::Counters},
    render,
    svg::{defs::SvgDef, element::Object},
    util::ChronuxResult,
};

pub fn draw(
    canvas: &Canvas,
    minimap: &MinimapConfig,
    timeline: &HashMap<String, Timeline>,
    objs: &[Object],
    t: f32,
    counters: &Counters,
    defs: &HashMap<String, SvgDef>,
    x: f32,
    y: f32,
    bounds: Option<(f32, f32, f32, f32)>,
    dict: &HashMap<String, f32>,
) -> ChronuxResult<()> {
    if bounds.is_none() {
        // 小地图为空，什么都不画
        return Ok(());
    }

    let canvas_restore_count = canvas.save();

    let bb = bounds.unwrap();
    let (bw, bh) = (bb.2 - bb.0, bb.3 - bb.1);

    let padding = minimap.padding;

    // 计算小地图绘制时每个像素应代表底图的多少像素
    // 要求不得少于 min_scale，且整个地图的尺寸不得大于 max_size
    let scale = bw / minimap.max_size.min(bw / minimap.min_scale);
    let scale = scale.max(bh / minimap.max_size.min(bh / minimap.min_scale));

    let mw = bw / scale; // minimap width
    let mh = bh / scale; // minimap height

    // 计算小地图实际内容区的位置（不算 padding）
    let (x1, y1, x2, y2) = match minimap.anchor {
        MinimapAnchor::TopLeft => (x + padding, y + padding, x + mw + padding, y + mh + padding),
        MinimapAnchor::TopRight => (x - mw - padding, y + padding, x - padding, y + mh + padding),
        MinimapAnchor::BottomLeft => (x + padding, y - mh - padding, x + mw + padding, y - padding),
        MinimapAnchor::BottomRight => {
            (x - mw - padding, y - mh - padding, x - padding, y - padding)
        }
    };

    // 计算小地图的实际尺寸（含 padding）
    let (rx1, ry1, rx2, ry2) = match minimap.anchor {
        MinimapAnchor::TopLeft => (
            x1 - 2.0 * padding,
            y1 - 2.0 * padding,
            x2 + padding,
            y2 + padding,
        ),
        MinimapAnchor::TopRight => (
            x1 - padding,
            y1 - 2.0 * padding,
            x2 + 2.0 * padding,
            y2 + padding,
        ),
        MinimapAnchor::BottomLeft => (
            x1 - 2.0 * padding,
            y1 - padding,
            x2 + padding,
            y2 + 2.0 * padding,
        ),
        MinimapAnchor::BottomRight => (
            x1 - padding,
            y1 - padding,
            x2 + 2.0 * padding,
            y2 + 2.0 * padding,
        ),
    };

    let mut fill = skia_safe::Paint::default();
    fill.set_anti_alias(true);
    fill.set_color(minimap.fill_color);
    fill.set_style(skia_safe::PaintStyle::Fill);

    let minimap_rect = Rect::from_xywh(rx1, ry1, rx2 - rx1, ry2 - ry1);

    canvas.draw_round_rect(
        minimap_rect,
        minimap.corner_radius,
        minimap.corner_radius,
        &fill,
    );

    canvas.clip_rrect(
        RRect::new_rect_xy(minimap_rect, minimap.corner_radius, minimap.corner_radius),
        ClipOp::Intersect,
        true,
    );

    canvas
        .translate((x1, y1))
        .scale((scale.recip(), scale.recip()))
        .translate((-bb.0, -bb.1));

    for o in objs.iter() {
        if minimap.paths.contains(&o.id) {
            let properties = match timeline.get(&o.id) {
                Some(tl) => tl.interpolate(t, dict),
                None => o.style.clone(),
            };
            render::draw(
                canvas,
                &o.shape,
                &properties,
                minimap.min_stroke_width,
                counters,
                &Rect::new(bb.0, bb.1, bb.2, bb.3),
                &None,
                defs,
                o.transform,
            )?;
        }
    }

    // 恢复坐标变换
    canvas.restore_to_count(canvas_restore_count);

    Ok(())
}
