use svgtypes::{PathParser, PathSegment};

use crate::util::{ChronuxError, ChronuxResult};

#[derive(Debug, Clone)]
pub enum Curve {
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    CubicBezier {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
        x4: f64,
        y4: f64,
    },
    QuadraticBezier {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
    },
    Arc {
        x1: f64,
        y1: f64,
        rx: f64,
        ry: f64,
        x_axis_rotation: f64,
        large_arc_flag: bool,
        sweep_flag: bool,
        x2: f64,
        y2: f64,
    },
}

impl Curve {
    pub fn from_path(d: &str) -> ChronuxResult<Vec<Curve>> {
        let mut segments = Vec::new();
        let (mut cx, mut cy) = (0.0, 0.0);
        let (mut sx, mut sy) = (0.0, 0.0);
        let (mut lx, mut ly) = (0.0, 0.0);
        let mut is_curve = false;
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
                    is_curve = false;
                }
                PathSegment::LineTo { abs, x, y } => {
                    if abs {
                        segments.push(Curve::Line {
                            x1: cx,
                            y1: cy,
                            x2: x,
                            y2: y,
                        });
                        (cx, cy) = (x, y);
                    } else {
                        segments.push(Curve::Line {
                            x1: cx,
                            y1: cy,
                            x2: cx + x,
                            y2: cy + y,
                        });
                        (cx, cy) = (cx + x, cy + y);
                    }
                    (lx, ly) = (cx, cy);
                    is_curve = false;
                }
                PathSegment::HorizontalLineTo { abs, x } => {
                    if abs {
                        segments.push(Curve::Line {
                            x1: cx,
                            y1: cy,
                            x2: x,
                            y2: cy,
                        });
                        cx = x;
                    } else {
                        segments.push(Curve::Line {
                            x1: cx,
                            y1: cy,
                            x2: cx + x,
                            y2: cy,
                        });
                        cx += x;
                    }
                    (lx, ly) = (cx, cy);
                    is_curve = false;
                }
                PathSegment::VerticalLineTo { abs, y } => {
                    if abs {
                        segments.push(Curve::Line {
                            x1: cx,
                            y1: cy,
                            x2: cx,
                            y2: y,
                        });
                        cy = y;
                    } else {
                        segments.push(Curve::Line {
                            x1: cx,
                            y1: cy,
                            x2: cx,
                            y2: cy + y,
                        });
                        cy += y;
                    }
                    (lx, ly) = (cx, cy);
                    is_curve = false;
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
                        segments.push(Curve::CubicBezier {
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
                        segments.push(Curve::CubicBezier {
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
                    is_curve = true;
                }
                PathSegment::SmoothCurveTo { abs, x2, y2, x, y } => {
                    if !is_curve {
                        (lx, ly) = (cx, cy);
                    }
                    if abs {
                        segments.push(Curve::CubicBezier {
                            x1: cx,
                            y1: cy,
                            x2: cx * 2.0 - lx,
                            y2: cy * 2.0 - ly,
                            x3: x2,
                            y3: y2,
                            x4: x,
                            y4: y,
                        });
                        (lx, ly) = (x2, y2);
                        (cx, cy) = (x, y);
                    } else {
                        segments.push(Curve::CubicBezier {
                            x1: cx,
                            y1: cy,
                            x2: cx * 2.0 - lx,
                            y2: cy * 2.0 - ly,
                            x3: cx + x2,
                            y3: cy + y2,
                            x4: cx + x,
                            y4: cy + y,
                        });
                        (lx, ly) = (cx + x2, cy + y2);
                        (cx, cy) = (cx + x, cy + y);
                    }
                    is_curve = true;
                }
                PathSegment::Quadratic { abs, x1, y1, x, y } => {
                    if abs {
                        segments.push(Curve::QuadraticBezier {
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
                        segments.push(Curve::QuadraticBezier {
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
                    is_curve = false;
                }
                PathSegment::SmoothQuadratic { abs, x, y } => {
                    if is_curve && abs {
                        (lx, ly) = (x, y);
                    } else if is_curve && !abs {
                        (lx, ly) = (cx + x, cy + y);
                    } else {
                        (lx, ly) = (cx * 2.0 - lx, cy * 2.0 - ly);
                    }
                    if abs {
                        segments.push(Curve::QuadraticBezier {
                            x1: cx,
                            y1: cy,
                            x2: lx,
                            y2: ly,
                            x3: x,
                            y3: y,
                        });
                        (cx, cy) = (x, y);
                    } else {
                        segments.push(Curve::QuadraticBezier {
                            x1: cx,
                            y1: cy,
                            x2: lx,
                            y2: ly,
                            x3: cx + x,
                            y3: cy + y,
                        });
                        (cx, cy) = (cx + x, cy + y);
                    }
                    is_curve = false;
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
                    if abs {
                        segments.push(Curve::Arc {
                            x1: cx,
                            y1: cy,
                            rx: rx,
                            ry: ry,
                            x_axis_rotation: x_axis_rotation,
                            large_arc_flag: large_arc,
                            sweep_flag: sweep,
                            x2: x,
                            y2: y,
                        });
                        cx = x;
                        cy = y;
                    } else {
                        segments.push(Curve::Arc {
                            x1: cx,
                            y1: cy,
                            rx: rx,
                            ry: ry,
                            x_axis_rotation: x_axis_rotation,
                            large_arc_flag: large_arc,
                            sweep_flag: sweep,
                            x2: cx + x,
                            y2: cy + y,
                        });
                        cx += x;
                        cy += y;
                    }
                    is_curve = false;
                }
                PathSegment::ClosePath { abs: _ } => {
                    segments.push(Curve::Line {
                        x1: cx,
                        y1: cy,
                        x2: sx,
                        y2: sy,
                    });
                    cx = sx;
                    cy = sy;
                    is_curve = false;
                }
            }
        }
        Ok(segments)
    }

    pub fn length(&self) -> f64 {
        match self {
            Curve::Line { x1, y1, x2, y2 } => {
                ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
            }
            Curve::CubicBezier { x1, y1, x2, y2, x3, y3, x4, y4 } => {
                // Approximate length of cubic bezier using numerical integration
                let steps = 100;
                let mut length = 0.0;
                for i in 0..steps {
                    let t1 = i as f64 / steps as f64;
                    let t2 = (i + 1) as f64 / steps as f64;
                    let p1 = bezier_point(t1, *x1, *y1, *x2, *y2, *x3, *y3, *x4, *y4);
                    let p2 = bezier_point(t2, *x1, *y1, *x2, *y2, *x3, *y3, *x4, *y4);
                    length += ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt();
                }
                length
            }
            Curve::QuadraticBezier { x1, y1, x2, y2, x3, y3 } => {
                // Approximate length of quadratic bezier using numerical integration
                let steps = 100;
                let mut length = 0.0;
                for i in 0..steps {
                    let t1 = i as f64 / steps as f64;
                    let t2 = (i + 1) as f64 / steps as f64;
                    let p1 = quadratic_point(t1, *x1, *y1, *x2, *y2, *x3, *y3);
                    let p2 = quadratic_point(t2, *x1, *y1, *x2, *y2, *x3, *y3);
                    length += ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt();
                }
                length
            }
            Curve::Arc { x1, y1, rx, ry, x_axis_rotation, large_arc_flag, sweep_flag, x2, y2 } => {
                // Approximate length of arc using numerical integration
                let steps = 100;
                let mut length = 0.0;
                for i in 0..steps {
                    let t1 = i as f64 / steps as f64;
                    let t2 = (i + 1) as f64 / steps as f64;
                    let p1 = arc_point(t1, *x1, *y1, *rx, *ry, *x_axis_rotation, *large_arc_flag, *sweep_flag, *x2, *y2);
                    let p2 = arc_point(t2, *x1, *y1, *rx, *ry, *x_axis_rotation, *large_arc_flag, *sweep_flag, *x2, *y2);
                    length += ((p2.0 - p1.0).powi(2) + (p2.1 - p1.1).powi(2)).sqrt();
                }
                length
            }
        }
    }
}
