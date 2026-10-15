//! Just enough complex arithmetic for the mode phasors.

use std::ops::{Add, AddAssign, Mul};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct C64 {
    pub re: f64,
    pub im: f64,
}

impl C64 {
    pub const ZERO: C64 = C64 { re: 0.0, im: 0.0 };

    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn from_polar(r: f64, phi: f64) -> Self {
        let (s, c) = phi.sin_cos();
        Self::new(r * c, r * s)
    }

    /// e^{re + i im}
    pub fn exp(re: f64, im: f64) -> Self {
        Self::from_polar(re.exp(), im)
    }

    pub fn norm(self) -> f64 {
        self.re.hypot(self.im)
    }
}

impl Add for C64 {
    type Output = C64;
    #[inline]
    fn add(self, o: C64) -> C64 {
        C64::new(self.re + o.re, self.im + o.im)
    }
}

impl AddAssign for C64 {
    #[inline]
    fn add_assign(&mut self, o: C64) {
        *self = *self + o;
    }
}

impl Mul for C64 {
    type Output = C64;
    #[inline]
    fn mul(self, o: C64) -> C64 {
        C64::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
}

impl Mul<f64> for C64 {
    type Output = C64;
    #[inline]
    fn mul(self, s: f64) -> C64 {
        C64::new(self.re * s, self.im * s)
    }
}
