use skia_safe::{Matrix, Path, PathDirection, Point, Vector, path::ArcSize};
use svgtypes::{PathParser, PathSegment};

use crate::util::{ChronuxError, ChronuxResult};

pub fn svg_to_path(d: &str, mat: Matrix) -> ChronuxResult<Path> {
    let mut p = Path::new();
    let mut cur = None;
    let mut first = None;
    for seg in PathParser::from(d) {
        if seg.is_err() {
            return Err(ChronuxError::SvgError(format!(
                "failed to parse path segment for d = {}",
                d
            )));
        }
        let seg = seg.unwrap();

        match seg {
            PathSegment::MoveTo { abs, x, y } => {
                let (x, y) = (x as f32, y as f32);
                if abs || cur.is_none() {
                    cur = Some((x, y));

                    let Point { x, y } = mat.map_point((x, y));
                    p.move_to((x, y));
                } else {
                    cur = cur.map(|(cx, cy)| (cx + x, cy + y));

                    let Vector { x, y } = mat.map_vector((x, y));
                    p.r_move_to((x, y));
                }
            }
            PathSegment::LineTo { abs, x, y } => {
                let (x, y) = (x as f32, y as f32);
                if abs {
                    cur = Some((x, y));

                    let Point { x, y } = mat.map_point((x, y));
                    p.line_to((x, y));
                } else {
                    cur = cur.map(|(cx, cy)| (cx + x, cy + y));

                    let Vector { x, y } = mat.map_vector((x, y));
                    p.r_line_to((x, y));
                }
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                let x = x as f32;
                if abs {
                    cur = cur.map(|(_cx, cy)| (x, cy));
                    let Point { x, y } = mat.map_point(cur.unwrap());
                    p.line_to((x, y));
                } else {
                    cur = cur.map(|(cx, cy)| (cx + x, cy));
                    let Vector { x, y } = mat.map_vector((x, 0.0));
                    p.r_line_to((x, y));
                }
            }
            PathSegment::VerticalLineTo { abs, y } => {
                let y = y as f32;
                if abs {
                    cur = cur.map(|(cx, _cy)| (cx, y));
                    let Point { x, y } = mat.map_point(cur.unwrap());
                    p.line_to((x, y));
                } else {
                    cur = cur.map(|(cx, cy)| (cx, cy + y));
                    let Vector { x, y } = mat.map_vector((0.0, y));
                    p.r_line_to((x, y));
                }
            }
            PathSegment::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let (x1, y1, x2, y2, x, y) = (
                    x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32,
                );
                if abs {
                    cur = Some((x, y));
                    let Point { x: x1, y: y1 } = mat.map_point((x1, y1));
                    let Point { x: x2, y: y2 } = mat.map_point((x2, y2));
                    let Point { x, y } = mat.map_point((x, y));
                    p.cubic_to((x1, y1), (x2, y2), (x, y));
                } else {
                    cur = cur.map(|(cx, cy)| (cx + x, cy + y));
                    let Vector { x: x1, y: y1 } = mat.map_vector((x1, y1));
                    let Vector { x: x2, y: y2 } = mat.map_vector((x2, y2));
                    let Vector { x, y } = mat.map_vector((x, y));
                    p.r_cubic_to((x1, y1), (x2, y2), (x, y));
                }
            }
            PathSegment::Quadratic { abs, x1, y1, x, y } => {
                let (x1, y1, x, y) = (x1 as f32, y1 as f32, x as f32, y as f32);
                if abs {
                    cur = Some((x, y));
                    let Point { x: x1, y: y1 } = mat.map_point((x1, y1));
                    let Point { x, y } = mat.map_point((x, y));
                    p.quad_to((x1, y1), (x, y));
                } else {
                    cur = cur.map(|(cx, cy)| (cx + x, cy + y));
                    let Vector { x: x1, y: y1 } = mat.map_vector((x1, y1));
                    let Vector { x, y } = mat.map_vector((x, y));
                    p.r_quad_to((x1, y1), (x, y));
                }
            }
            PathSegment::EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => {
                let (rx, ry, x_axis_rotation, x, y) = (
                    rx as f32,
                    ry as f32,
                    x_axis_rotation as f32,
                    x as f32,
                    y as f32,
                );
                if abs {
                    cur = Some((x, y));
                    let Point { x, y } = mat.map_point((x, y));
                    let Vector { x: rx, y: ry } = mat.map_vector((rx, ry));

                    let rot = mat.map_vector((x_axis_rotation.cos(), x_axis_rotation.sin()));
                    let x_axis_rotation = rot.x.atan2(rot.y).to_degrees();
                    p.arc_to_rotated(
                        (rx, ry),
                        x_axis_rotation,
                        if large_arc {
                            ArcSize::Large
                        } else {
                            ArcSize::Small
                        },
                        if sweep {
                            PathDirection::CW
                        } else {
                            PathDirection::CCW
                        },
                        (x, y),
                    );
                } else {
                    cur = cur.map(|(cx, cy)| (cx + x, cy + y));
                    let Vector { x, y } = mat.map_vector((x, y));
                    let Vector { x: rx, y: ry } = mat.map_vector((rx, ry));

                    let rot = mat.map_vector((x_axis_rotation.cos(), x_axis_rotation.sin()));
                    let x_axis_rotation = rot.x.atan2(rot.y).to_degrees();
                    p.r_arc_to_rotated(
                        (rx, ry),
                        x_axis_rotation,
                        if large_arc {
                            ArcSize::Large
                        } else {
                            ArcSize::Small
                        },
                        if sweep {
                            PathDirection::CW
                        } else {
                            PathDirection::CCW
                        },
                        (x, y),
                    );
                }
            }
            PathSegment::ClosePath { abs: _ } => {
                p.close();
                cur = first;
                first = None;
            }
            _ => {
                return Err(ChronuxError::SvgError(format!(
                    "unsupported path segment {:?} in d = {}",
                    seg, d
                )));
            }
        }
        if first.is_none() {
            first = cur;
        }
    }
    Ok(p)
}
