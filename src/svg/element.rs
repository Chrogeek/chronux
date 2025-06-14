use core::sync;

use roxmltree::Node;
use svgtypes::{PathParser, PathSegment};

use crate::{
    animation::properties::Properties,
    svg::{self, path::Path},
    util::{ChronuxError, ChronuxResult},
};

#[derive(Debug)]
pub struct Object {
    pub id: String,
    pub style: Properties,
    pub shape: Shape,
}

#[derive(Debug)]
pub enum Shape {
    Path { d: Vec<Path> },
    Circle { cx: f64, cy: f64, r: f64 },
    Text { x: f64, y: f64, content: String },
}

impl Object {
    pub fn from_node(node: &Node, id: String) -> ChronuxResult<Object> {
        let style = Properties::from_style(&svg::attribute_str(node, "style")?)?;
        Ok(Object {
            id,
            style,
            shape: match node.tag_name().name() {
                "path" => Shape::Path {
                    d: parse_path(&svg::attribute_str(node, "d")?)?,
                },
                "circle" => Shape::Circle {
                    cx: svg::attribute_f64(node, "cx")?,
                    cy: svg::attribute_f64(node, "cy")?,
                    r: svg::attribute_f64(node, "cr")?,
                },
                "text" => Shape::Text {
                    x: svg::attribute_f64(node, "x")?,
                    y: svg::attribute_f64(node, "y")?,
                    content: node.text().unwrap_or_default().into(),
                },
                _ => unreachable!("unexpected SVG element: {}", node.tag_name().name()),
            },
        })
    }
}

pub fn parse_path(d: &str) -> ChronuxResult<Vec<Path>> {
    let mut segments = Vec::new();
    let (mut cx, mut cy) = (0.0, 0.0);
    let (mut sx, mut sy) = (0.0, 0.0);
    let (mut lx, mut ly) = (0.0, 0.0);
    for (num, seg) in PathParser::from(d).enumerate() {
        if seg.is_err() {
            return Err(ChronuxError::SvgError(format!(
                "failed to parse path segment for d = {}",
                d
            )));
        }
        let seg = seg.unwrap();
        if num == 0 {
            if let PathSegment::MoveTo { abs, x, y } = seg {
                if !abs {
                    return Err(ChronuxError::SvgError(format!(
                        "first path segment must be absolute MoveTo, found {:?} for d = {}",
                        seg, d
                    )));
                }
                (cx, cy) = (x, y);
                (sx, sy) = (x, y);
            } else {
                return Err(ChronuxError::SvgError(format!(
                    "first path segment must be MoveTo, found {:?} for d = {}",
                    seg, d
                )));
            }
            continue;
        }
        match seg {
            PathSegment::MoveTo { abs, x, y } => {
                if abs {
                    (cx, cy) = (x, y);
                } else {
                    (cx, cy) = (cx + x, cy + y);
                }
                (lx, ly) = (cx, cy);
            }
            PathSegment::LineTo { abs, x, y } => {
                if abs {
                    segments.push(Path::Line {
                        x1: cx,
                        y1: cy,
                        x2: x,
                        y2: y,
                    });
                    (cx, cy) = (x, y);
                } else {
                    segments.push(Path::Line {
                        x1: cx,
                        y1: cy,
                        x2: cx + x,
                        y2: cy + y,
                    });
                    (cx, cy) = (cx + x, cy + y);
                }
                (lx, ly) = (cx, cy);
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                if abs {
                    segments.push(Path::Line {
                        x1: cx,
                        y1: cy,
                        x2: x,
                        y2: cy,
                    });
                    cx = x;
                } else {
                    segments.push(Path::Line {
                        x1: cx,
                        y1: cy,
                        x2: cx + x,
                        y2: cy,
                    });
                    cx += x;
                }
                (lx, ly) = (cx, cy);
            }
            PathSegment::VerticalLineTo { abs, y } => {
                if abs {
                    segments.push(Path::Line {
                        x1: cx,
                        y1: cy,
                        x2: cx,
                        y2: y,
                    });
                    cy = y;
                } else {
                    segments.push(Path::Line {
                        x1: cx,
                        y1: cy,
                        x2: cx,
                        y2: cy + y,
                    });
                    cy += y;
                }
                (lx, ly) = (cx, cy);
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
                    segments.push(Path::CubicBezier {
                        x1: cx,
                        y1: cy,
                        x2: x1,
                        y2: y1,
                        x3: x2,
                        y3: y2,
                        x4: x,
                        y4: y,
                    });
                    (lx, ly) = (x2, y2);
                    (cx, cy) = (x, y);
                } else {
                    segments.push(Path::CubicBezier {
                        x1: cx,
                        y1: cy,
                        x2: cx + x1,
                        y2: cy + y1,
                        x3: cx + x2,
                        y3: cy + y2,
                        x4: cx + x,
                        y4: cy + y,
                    });
                    (lx, ly) = (cx + x2, cy + y2);
                    (cx, cy) = (cx + x, cy + y);
                }
            }
            PathSegment::SmoothCurveTo { abs, x2, y2, x, y } => {
                if abs {
                    segments.push(Path::CubicBezier {
                        x1: cx,
                        y1: cy,
                        x2: lx,
                        y2: ly,
                        x3: x2,
                        y3: y2,
                        x4: x,
                        y4: y,
                    });
                    (lx, ly) = (x2, y2);
                    (cx, cy) = (x, y);
                } else {
                    segments.push(Path::CubicBezier {
                        x1: cx,
                        y1: cy,
                        x2: lx,
                        y2: ly,
                        x3: cx + x2,
                        y3: cy + y2,
                        x4: cx + x,
                        y4: cy + y,
                    });
                    (lx, ly) = (cx + x2, cy + y2);
                    (cx, cy) = (cx + x, cy + y);
                }
            },
            PathSegment::Quadratic { abs, x1, y1, x, y } => {
                if abs {
                    segments.push(Path::QuadraticBezier {
                        x1: cx,
                        y1: cy,
                        x2: x1,
                        y2: y1,
                        x3: x,
                        y3: y,
                    });
                    (lx, ly) = (x1, y1);
                    (cx, cy) = (x, y);
                } else {
                    segments.push(Path::QuadraticBezier {
                        x1: cx,
                        y1: cy,
                        x2: cx + x1,
                        y2: cy + y1,
                        x3: cx + x,
                        y3: cy + y,
                    });
                    (lx, ly) = (cx + x1, cy + y1);
                    (cx, cy) = (cx + x, cy + y);
                }
            }
            PathSegment::SmoothQuadratic { abs, x, y } => {
                if abs {
                    segments.push(Path::QuadraticBezier {
                        x1: cx,
                        y1: cy,
                        x2: lx,
                        y2: ly,
                        x3: x,
                        y3: y,
                    });
                    (lx, ly) = (cx, cy);
                    (cx, cy) = (x, y);
                } else {
                    segments.push(Path::QuadraticBezier {
                        x1: cx,
                        y1: cy,
                        x2: lx,
                        y2: ly,
                        x3: cx + x,
                        y3: cy + y,
                    });
                    (lx, ly) = (cx + x, cy + y);
                    (cx, cy) = (cx + x, cy + y);
                }
            },
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
                if abs {
                    segments.push(Path::Arc {
                        cx: cx,
                        cy: cy,
                        rx: rx,
                        ry: ry,
                        x_axis_rotation: x_axis_rotation,
                        large_arc_flag: large_arc,
                        sweep_flag: sweep,
                        x: x,
                        y: y,
                    });
                    cx = x;
                    cy = y;
                } else {
                    segments.push(Path::Arc {
                        cx: cx,
                        cy: cy,
                        rx: rx,
                        ry: ry,
                        x_axis_rotation: x_axis_rotation,
                        large_arc_flag: large_arc,
                        sweep_flag: sweep,
                        x: cx + x,
                        y: cy + y,
                    });
                    cx += x;
                    cy += y;
                }
            }
            PathSegment::ClosePath { abs: _ } => {
                segments.push(Path::Line {
                    x1: cx,
                    y1: cy,
                    x2: sx,
                    y2: sy,
                });
                cx = sx;
                cy = sy;
            }
        }
    }
    Ok(segments)
}
