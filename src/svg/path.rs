use skia_safe::Path;
use svgtypes::{PathParser, PathSegment};

use crate::util::{ChronuxError, ChronuxResult};

pub fn svg_to_path(d: &str) -> ChronuxResult<Path> {
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
                    p.move_to((x, y));
                    (cx, cy) = (x, y);
                } else {
                    p.r_move_to((x, y));
                    (cx, cy) = (cx + x, cy + y);
                }
            }
            PathSegment::LineTo { abs, x, y } => {
                let (x, y) = (x as f32, y as f32);
                if abs {
                    p.line_to((x, y));
                    (cx, cy) = (x, y);
                } else {
                    p.r_line_to((x, y));
                    (cx, cy) = (cx + x, cy + y);
                }
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                let x = x as f32;
                if abs {
                    p.line_to((x, cy));
                    cx = x;
                } else {
                    p.r_line_to((x, 0.0));
                    cx += x;
                }
            }
            PathSegment::VerticalLineTo { abs, y } => {
                let y = y as f32;
                if abs {
                    p.line_to((cx, y));
                    cy = y;
                } else {
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
                let (x1, y1, x2, y2, x, y) = (
                    x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32,
                );
                if abs {
                    p.cubic_to((x1, y1), (x2, y2), (x, y));
                    (cx, cy) = (x, y);
                } else {
                    p.r_cubic_to((x1, y1), (x2, y2), (x, y));
                    (cx, cy) = (cx + x, cy + y);
                }
            }
            PathSegment::Quadratic { abs, x1, y1, x, y } => {
                let (x1, y1, x, y) = (x1 as f32, y1 as f32, x as f32, y as f32);
                if abs {
                    p.quad_to((x1, y1), (x, y));
                    (cx, cy) = (x, y);
                } else {
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
