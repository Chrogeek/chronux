#[derive(Debug, Clone)]
pub enum Path {
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
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        x_axis_rotation: f64,
        large_arc_flag: bool,
        sweep_flag: bool,
        x: f64,
        y: f64,
    },
    ClosePath,
}
