use crate::util::{
    ChronuxResult,
    interpolator::{pchip::PchipInterpolator, zero::ZeroInterpolator},
};

mod pchip;
mod zero;

#[derive(Debug, Clone)]
pub enum Interpolator {
    Pchip(PchipInterpolator),
    Zero(ZeroInterpolator),
}

impl Interpolator {
    pub fn new_pchip(t: Vec<f32>, p: Vec<f32>) -> ChronuxResult<Interpolator> {
        Ok(Interpolator::Pchip(PchipInterpolator::new(t, p)?))
    }

    pub fn new_zero(t_start: f32, t_end: f32, p: f32) -> Interpolator {
        Interpolator::Zero(ZeroInterpolator::new(t_start, t_end, p))
    }

    pub fn interpolate(&self, t: f32) -> Option<f32> {
        match self {
            Interpolator::Pchip(inter) => inter.interpolate(t),
            Interpolator::Zero(inter) => inter.interpolate(t),
        }
    }

    pub fn t_start(&self) -> f32 {
        match self {
            Interpolator::Pchip(inter) => inter.t_start,
            Interpolator::Zero(inter) => inter.t_start,
        }
    }

    pub fn t_end(&self) -> f32 {
        match self {
            Interpolator::Pchip(inter) => inter.t_end,
            Interpolator::Zero(inter) => inter.t_end,
        }
    }

    pub fn p_start(&self) -> f32 {
        match self {
            Interpolator::Pchip(inter) => inter.p_start,
            Interpolator::Zero(inter) => inter.p,
        }
    }

    pub fn p_end(&self) -> f32 {
        match self {
            Interpolator::Pchip(inter) => inter.p_end,
            Interpolator::Zero(inter) => inter.p,
        }
    }
}
