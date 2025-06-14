use skia_safe::Path;
use svgtypes::{PathParser, PathSegment};

use crate::{
    svg::Dimension,
    util::{ChronuxError, ChronuxResult},
};

pub fn svg_to_path(d: &str, dim: &Dimension) -> ChronuxResult<Path> {
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
                    let (x, y) = dim.to_real_coords(x, y);
                    let (x, y) = (x as f32, y as f32);
                    p.move_to((x, y));
                    (cx, cy) = (x, y);
                } else {
                    let (x, y) = (dim.to_real_length(x), dim.to_real_length(y));
                    let (x, y) = (x as f32, y as f32);
                    p.r_move_to((x, y));
                    (cx, cy) = (cx + x, cy + y);
                }
            }
            PathSegment::LineTo { abs, x, y } => {
                if abs {
                    let (x, y) = dim.to_real_coords(x, y);
                    let (x, y) = (x as f32, y as f32);
                    p.line_to((x, y));
                    (cx, cy) = (x, y);
                } else {
                    let (x, y) = (dim.to_real_length(x), dim.to_real_length(y));
                    let (x, y) = (x as f32, y as f32);
                    p.r_line_to((x, y));
                    (cx, cy) = (cx + x, cy + y);
                }
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                if abs {
                    let x = dim.to_real_coords(x, 0.0).0 as f32;
                    p.line_to((x, cy));
                    cx = x;
                } else {
                    let x = dim.to_real_length(x) as f32;
                    p.r_line_to((x, 0.0));
                    cx += x;
                }
            }
            PathSegment::VerticalLineTo { abs, y } => {
                if abs {
                    let y = dim.to_real_coords(0.0, y).1 as f32;
                    p.line_to((cx, y));
                    cy = y;
                } else {
                    let y = dim.to_real_length(y) as f32;
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
                    let ((x1, y1), (x2, y2), (x, y)) = (
                        dim.to_real_coords(x1, y1),
                        dim.to_real_coords(x2, y2),
                        dim.to_real_coords(x, y),
                    );
                    let (x1, y1, x2, y2, x, y) = (
                        x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32,
                    );
                    p.cubic_to((x1, y1), (x2, y2), (x, y));
                    (cx, cy) = (x, y);
                } else {
                    let (x1, y1, x2, y2, x, y) = (
                        dim.to_real_length(x1),
                        dim.to_real_length(y1),
                        dim.to_real_length(x2),
                        dim.to_real_length(y2),
                        dim.to_real_length(x),
                        dim.to_real_length(y),
                    );
                    let (x1, y1, x2, y2, x, y) = (
                        x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32,
                    );
                    p.r_cubic_to((x1, y1), (x2, y2), (x, y));
                    (cx, cy) = (cx + x, cy + y);
                }
            }
            PathSegment::Quadratic { abs, x1, y1, x, y } => {
                if abs {
                    let (x1, y1) = dim.to_real_coords(x1, y1);
                    let (x, y) = dim.to_real_coords(x, y);
                    let (x1, y1, x, y) = (x1 as f32, y1 as f32, x as f32, y as f32);
                    p.quad_to((x1, y1), (x, y));
                    (cx, cy) = (x, y);
                } else {
                    let (x1, y1) = (dim.to_real_length(x1), dim.to_real_length(y1));
                    let (x, y) = (dim.to_real_length(x), dim.to_real_length(y));
                    let (x1, y1, x, y) = (x1 as f32, y1 as f32, x as f32, y as f32);
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
