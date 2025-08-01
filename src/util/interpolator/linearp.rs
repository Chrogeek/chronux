use crate::util::{ChronuxError, ChronuxResult};

#[derive(Debug, Clone)]
pub struct LinearInterpolator {
    t: Vec<f32>,
    p: Vec<f32>,
    max_t: f32,   // maximum time value
    t_start: f32, // start time
    p_start: f32, // start position
    p_end: f32,   // end position
}

impl LinearInterpolator {
    pub fn new(t: Vec<f32>, p: Vec<f32>) -> ChronuxResult<LinearInterpolator> {
        // check input validity
        if t.len() != p.len() || t.is_empty() {
            return Err(ChronuxError::ArgError(
                "Time and position arrays must have the same non-zero length.".into(),
            ));
        }

        let n = t.len();

        // check if t is strictly increasing
        for i in 1..n {
            if t[i] <= t[i - 1] {
                return Err(ChronuxError::ArgError(
                    "Time points must be strictly increasing.".into(),
                ));
            }
        }

        // check if p is non-decreasing
        for i in 1..n {
            if p[i] < p[i - 1] {
                return Err(ChronuxError::ArgError(
                    "Position points must be non-decreasing.".into(),
                ));
            }
        }

        let t_start = t[0];
        let max_t = *t.last().unwrap() - t_start;
        let p_start = p[0];
        let p_end = p[n - 1];

        let t = t.iter().map(|v| v - t_start).collect::<Vec<_>>();
        let p = p
            .iter()
            .map(|v| (v - p_start) / (p_end - p_start))
            .collect::<Vec<_>>();

        Ok(LinearInterpolator {
            t,
            p,
            max_t,
            t_start,
            p_start,
            p_end,
        })
    }

    pub fn interpolate(&self, t: f32) -> Option<f32> {
        if t <= self.t_start || t >= self.max_t + self.t_start {
            log::warn!(
                "Time point out of bounds in linear interpolation: t = {}, t_start = {}, t_length = {}",
                t,
                self.t_start,
                self.max_t
            );
            return None;
        }
        let t = t - self.t_start;
        let i = self.t.iter().position(|&x| x > t).unwrap() - 1;
        let dt = t - self.t[i];
        let p_i = self.p[i];
        let p = p_i + dt / (self.t[i + 1] - self.t[i]) * (self.p[i + 1] - p_i);
        Some(p * (self.p_end - self.p_start) + self.p_start)
    }
}
