use skia_safe::{Matrix, Path, Point, Vector};
use svgtypes::{PathParser, PathSegment};

use crate::util::{ChronuxError, ChronuxResult};

pub fn svg_to_path(d: &str, mat: Matrix) -> ChronuxResult<Path> {
    let mut p = Path::new();
    let (mut cx, mut cy) = (0.0, 0.0);
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
                if abs {
                    let (x, y) = (x as f32, y as f32);
                    let Point { x, y } = mat.map_point((x, y));
                    p.move_to((x, y));
                    (cx, cy) = (x, y);
                } else {
                    let (x, y) = (x as f32, y as f32);
                    let Vector { x, y } = mat.map_vector((x, y));
                    p.r_move_to((x, y));
                    (cx, cy) = (cx + x, cy + y);
                }
            }
            PathSegment::LineTo { abs, x, y } => {
                if abs {
                    let (x, y) = (x as f32, y as f32);
                    let Point { x, y } = mat.map_point((x, y));
                    p.line_to((x, y));
                    (cx, cy) = (x, y);
                } else {
                    let (x, y) = (x as f32, y as f32);
                    let Vector { x, y } = mat.map_vector((x, y));
                    p.r_line_to((x, y));
                    (cx, cy) = (cx + x, cy + y);
                }
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                if abs {
                    let Point { x, y: _ } = mat.map_point((x as f32, 0.0));
                    p.line_to((x, cy));
                    cx = x;
                } else {
                    let Vector { x, y: _ } = mat.map_vector((x as f32, 0.0));
                    p.r_line_to((x, 0.0));
                    cx += x;
                }
            }
            PathSegment::VerticalLineTo { abs, y } => {
                if abs {
                    let Point { x: _, y } = mat.map_point((0.0, y as f32));
                    p.line_to((cx, y));
                    cy = y;
                } else {
                    let Vector { x: _, y } = mat.map_vector((0.0, y as f32));
                    p.r_line_to((0.0, y));
                    cy += y;
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
                if abs {
                    let (x1, y1, x2, y2, x, y) = (
                        x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32,
                    );
                    let Point { x: x1, y: y1 } = mat.map_point((x1, y1));
                    let Point { x: x2, y: y2 } = mat.map_point((x2, y2));
                    let Point { x, y } = mat.map_point((x, y));
                    p.cubic_to((x1, y1), (x2, y2), (x, y));
                    (cx, cy) = (x, y);
                } else {
                    let (x1, y1, x2, y2, x, y) = (
                        x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32,
                    );
                    let Vector { x: x1, y: y1 } = mat.map_vector((x1, y1));
                    let Vector { x: x2, y: y2 } = mat.map_vector((x2, y2));
                    let Vector { x, y } = mat.map_vector((x, y));
                    p.r_cubic_to((x1, y1), (x2, y2), (x, y));
                    (cx, cy) = (cx + x, cy + y);
                }
            }
            PathSegment::Quadratic { abs, x1, y1, x, y } => {
                if abs {
                    let (x1, y1, x, y) = (x1 as f32, y1 as f32, x as f32, y as f32);
                    let Point { x: x1, y: y1 } = mat.map_point((x1, y1));
                    let Point { x, y } = mat.map_point((x, y));
                    p.quad_to((x1, y1), (x, y));
                    (cx, cy) = (x, y);
                } else {
                    let (x1, y1, x, y) = (x1 as f32, y1 as f32, x as f32, y as f32);
                    let Vector { x: x1, y: y1 } = mat.map_vector((x1, y1));
                    let Vector { x, y } = mat.map_vector((x, y));
                    p.r_quad_to((x1, y1), (x, y));
                    (cx, cy) = (cx + x, cy + y);
                }
            }
            PathSegment::ClosePath { abs: _ } => {
                p.close();
            }
            _ => {
                return Err(ChronuxError::SvgError(format!(
                    "unsupported path segment {:?} in d = {}",
                    seg, d
                )));
            }
        }
    }
    Ok(p)
}
