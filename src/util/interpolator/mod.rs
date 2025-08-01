use crate::util::{interpolator::{linearp::LinearInterpolator, pchip::PchipInterpolator}, ChronuxResult};

mod linearp;
mod pchip;

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Interpolator {
    Pchip(PchipInterpolator),
    Linear(LinearInterpolator),
}

#[allow(unused)]
impl Interpolator {
    pub fn new_pchip(t: Vec<f32>, p: Vec<f32>) -> ChronuxResult<Interpolator> {
        Ok(Interpolator::Pchip(PchipInterpolator::new(t, p)?))
    }

    pub fn new_linear(t: Vec<f32>, p: Vec<f32>) -> ChronuxResult<Interpolator> {
        Ok(Interpolator::Linear(LinearInterpolator::new(t, p)?))
    }

    pub fn interpolate(&self, t: f32) -> Option<f32> {
        match self {
            Interpolator::Pchip(pchip) => pchip.interpolate(t),
            Interpolator::Linear(linear) => linear.interpolate(t),
        }
    }
}
