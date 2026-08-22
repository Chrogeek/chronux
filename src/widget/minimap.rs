use std::collections::{HashMap, HashSet};

use skia_safe::{
    Canvas, ClipOp, Color, Paint, PaintStyle, RRect, Rect, TileMode,
    canvas::{SaveLayerFlags, SaveLayerRec},
    image_filters::{blur, drop_shadow_only},
};

use crate::{
    animation::timeline::Timeline,
    config::{MinimapAnchor, MinimapConfig, counter::Counters},
    render,
    svg::{defs::SvgDef, element::Object},
    util::{dict::RenderDict, error::ChronuxResult},
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
    dict: &RenderDict,
    phantom: &HashSet<String>,
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

    let minimap_rect = Rect::from_xywh(rx1, ry1, rx2 - rx1, ry2 - ry1);

    if minimap.shadow > 0.0 {
        let sh = minimap.shadow;
        let shadow_color = Color::BLACK;
        let shadow_filter =
            drop_shadow_only((0.0, 0.0), (sh, sh), shadow_color, None, None, None).unwrap();
        let mut shadow_paint = Paint::default();
        shadow_paint
            .set_image_filter(shadow_filter)
            .set_style(PaintStyle::Stroke)
            .set_anti_alias(true);
        canvas.draw_round_rect(
            minimap_rect,
            minimap.corner_radius,
            minimap.corner_radius,
            &shadow_paint,
        );
    }

    canvas.clip_rrect(
        RRect::new_rect_xy(minimap_rect, minimap.corner_radius, minimap.corner_radius),
        ClipOp::Intersect,
        true,
    );

    if minimap.backdrop_blur > 0.0 {
        let blur_bounds = minimap_rect;
        blur(
            (minimap.backdrop_blur, minimap.backdrop_blur),
            TileMode::Clamp,
            None,
            blur_bounds,
        )
        .inspect(|filter| {
            let save_layer_rec = SaveLayerRec::default()
                .bounds(&minimap_rect)
                .backdrop(&filter)
                .flags(SaveLayerFlags::INIT_WITH_PREVIOUS);
            canvas.save_layer(&save_layer_rec);
        });
    }

    let mut fill = skia_safe::Paint::default();
    fill.set_anti_alias(true);
    fill.set_color4f(minimap.fill_color, None);
    fill.set_style(skia_safe::PaintStyle::Fill);
    canvas.draw_round_rect(
        minimap_rect,
        minimap.corner_radius,
        minimap.corner_radius,
        &fill,
    );

    if minimap.stroke_width > 0.0 {
        let mut stroke = skia_safe::Paint::default();
        stroke.set_anti_alias(true);
        stroke.set_color4f(minimap.stroke_color, None);
        stroke.set_stroke_width(minimap.stroke_width);
        stroke.set_style(skia_safe::PaintStyle::Stroke);
        canvas.draw_round_rect(
            minimap_rect,
            minimap.corner_radius,
            minimap.corner_radius,
            &stroke,
        );
    }

    canvas
        .translate((x1, y1))
        .scale((scale.recip(), scale.recip()))
        .translate((-bb.0, -bb.1));

    for o in objs.iter() {
        if minimap.paths.contains(&o.id) && !phantom.contains(&o.id) {
            let properties = match timeline.get(&o.id) {
                Some(tl) => tl.interpolate(t, dict),
                None => o.style.clone(),
            };
            render::draw(
                canvas,
                &properties,
                minimap.min_stroke_width,
                counters,
                &Rect::new(bb.0, bb.1, bb.2, bb.3),
                defs,
                o.transform,
            )?;
        }
    }

    // 恢复坐标变换
    canvas.restore_to_count(canvas_restore_count);

    Ok(())
}
