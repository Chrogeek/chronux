use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Clone, Copy)]
pub struct Vec6(pub f32, pub f32, pub f32, pub f32, pub f32, pub f32);

impl Add for Vec6 {
    type Output = Vec6;

    fn add(self, other: Vec6) -> Self::Output {
        Vec6(
            self.0 + other.0,
            self.1 + other.1,
            self.2 + other.2,
            self.3 + other.3,
            self.4 + other.4,
            self.5 + other.5,
        )
    }
}

impl Sub for Vec6 {
    type Output = Vec6;

    fn sub(self, other: Vec6) -> Self::Output {
        Vec6(
            self.0 - other.0,
            self.1 - other.1,
            self.2 - other.2,
            self.3 - other.3,
            self.4 - other.4,
            self.5 - other.5,
        )
    }
}

impl Mul<f32> for Vec6 {
    type Output = Vec6;

    fn mul(self, scalar: f32) -> Self::Output {
        Vec6(
            self.0 * scalar,
            self.1 * scalar,
            self.2 * scalar,
            self.3 * scalar,
            self.4 * scalar,
            self.5 * scalar,
        )
    }
}

impl Mul<Vec6> for f32 {
    type Output = Vec6;

    fn mul(self, vp: Vec6) -> Self::Output {
        Vec6(
            vp.0 * self,
            vp.1 * self,
            vp.2 * self,
            vp.3 * self,
            vp.4 * self,
            vp.5 * self,
        )
    }
}

impl Div<f32> for Vec6 {
    type Output = Vec6;

    fn div(self, scalar: f32) -> Self::Output {
        Vec6(
            self.0 / scalar,
            self.1 / scalar,
            self.2 / scalar,
            self.3 / scalar,
            self.4 / scalar,
            self.5 / scalar,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Add for Vec3 {
    type Output = Vec3;

    fn add(self, other: Vec3) -> Self::Output {
        Vec3(self.0 + other.0, self.1 + other.1, self.2 + other.2)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, other: Vec3) -> Self::Output {
        Vec3(self.0 - other.0, self.1 - other.1, self.2 - other.2)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;

    fn mul(self, scalar: f32) -> Self::Output {
        Vec3(self.0 * scalar, self.1 * scalar, self.2 * scalar)
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;

    fn mul(self, vp: Vec3) -> Self::Output {
        Vec3(vp.0 * self, vp.1 * self, vp.2 * self)
    }
}

impl Mul<Vec3> for Vec3 {
    type Output = Vec3;

    fn mul(self, other: Vec3) -> Self::Output {
        Vec3(self.0 * other.0, self.1 * other.1, self.2 * other.2)
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;

    fn div(self, scalar: f32) -> Self::Output {
        Vec3(self.0 / scalar, self.1 / scalar, self.2 / scalar)
    }
}
