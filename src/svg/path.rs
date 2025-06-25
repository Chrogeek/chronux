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
                let (x, y) = (x as f32, y as f32);
                if abs {
                    (cx, cy) = (x, y);

                    let Point { x, y } = mat.map_point((x, y));
                    p.move_to((x, y));
                } else {
                    (cx, cy) = (cx + x, cy + y);

                    let Vector { x, y } = mat.map_vector((x, y));
                    p.r_move_to((x, y));
                }
            }
            PathSegment::LineTo { abs, x, y } => {
                let (x, y) = (x as f32, y as f32);
                if abs {
                    (cx, cy) = (x, y);

                    let Point { x, y } = mat.map_point((x, y));
                    p.line_to((x, y));
                } else {
                    (cx, cy) = (cx + x, cy + y);

                    let Vector { x, y } = mat.map_vector((x, y));
                    p.r_line_to((x, y));
                }
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                let x = x as f32;
                if abs {
                    cx = x;
                } else {
                    cx += x;
                }
                let Point { x, y } = mat.map_point((cx, cy));
                p.line_to((x, y));
            }
            PathSegment::VerticalLineTo { abs, y } => {
                let y = y as f32;
                if abs {
                    cy = y;
                } else {
                    cy += y;
                }
                let Point { x, y } = mat.map_point((cx, cy));
                p.line_to((x, y));
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
                    (cx, cy) = (x, y);
                    let Point { x: x1, y: y1 } = mat.map_point((x1, y1));
                    let Point { x: x2, y: y2 } = mat.map_point((x2, y2));
                    let Point { x, y } = mat.map_point((x, y));
                    p.cubic_to((x1, y1), (x2, y2), (x, y));
                } else {
                    (cx, cy) = (cx + x, cy + y);
                    let Vector { x: x1, y: y1 } = mat.map_vector((x1, y1));
                    let Vector { x: x2, y: y2 } = mat.map_vector((x2, y2));
                    let Vector { x, y } = mat.map_vector((x, y));
                    p.r_cubic_to((x1, y1), (x2, y2), (x, y));
                }
            }
            PathSegment::Quadratic { abs, x1, y1, x, y } => {
                let (x1, y1, x, y) = (x1 as f32, y1 as f32, x as f32, y as f32);
                if abs {
                    (cx, cy) = (x, y);
                    let Point { x: x1, y: y1 } = mat.map_point((x1, y1));
                    let Point { x, y } = mat.map_point((x, y));
                    p.quad_to((x1, y1), (x, y));
                } else {
                    (cx, cy) = (cx + x, cy + y);
                    let Vector { x: x1, y: y1 } = mat.map_vector((x1, y1));
                    let Vector { x, y } = mat.map_vector((x, y));
                    p.r_quad_to((x1, y1), (x, y));
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
