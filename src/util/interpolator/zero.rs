#[derive(Debug, Clone)]
pub struct ZeroInterpolator {
    pub t_start: f32,
    pub t_end: f32,
    pub p: f32,
}

impl ZeroInterpolator {
    pub fn new(t_start: f32, t_end: f32, p: f32) -> ZeroInterpolator {
        ZeroInterpolator { t_start, t_end, p }
    }

    pub fn interpolate(&self, t: f32) -> Option<f32> {
        if t < self.t_start || t > self.t_end {
            None
        } else {
            Some(self.p)
        }
    }
}
