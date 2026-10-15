//! Hammer strike: a raised-cosine force pulse F(t) = F0 sin^2(pi t / tau) of length tau
//! and impulse vel.
//!
//! Under the hammer (0 <= t <= tau) each undamped mode follows the closed-form forced
//! response. From t = tau on it swings freely as q(s) = A e^{-lam s} sin(w s + phi),
//! s = t - tau, with A and phi handed over at the end of contact. The free part is kept as
//! the phasor h = A e^{i phi}, so that q(s) = Im(h e^{(-lam + i w) s}).

use crate::cplx::C64;
use std::f64::consts::TAU;

/// Below this |w^2 - Omega^2| (rad^2/s^2) a mode is resonant with the pulse and the
/// closed form divides by ~0, so such modes are zeroed.
const EPS_RES: f64 = 1e-9;

/// Contact time tau = tau0 vel^-beta: softer strikes stay on the tine longer.
pub fn contact_time(tau0: f64, beta: f64, vel: f64) -> f64 {
    tau0 * vel.powf(-beta)
}

/// One mode's response to one strike.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModeStrike {
    /// Forced response x(t) = k1 (1 - cos wt) - k2 (cos Om t - cos wt).
    k1: f64,
    k2: f64,
    /// Free-oscillation phasor A e^{i phi} at t = tau.
    pub h: C64,
}

impl ModeStrike {
    /// Mode of angular frequency w and gain c, struck with velocity vel for tau seconds.
    pub fn new(w: f64, c: f64, vel: f64, tau: f64) -> Self {
        let f0 = 2.0 * vel / tau; // peak force, so that the impulse F0 tau / 2 is vel
        let om = TAU / tau;
        let den = w * w - om * om;
        if den.abs() < EPS_RES {
            return Self::default();
        }
        let k = -f0 * om * om / (2.0 * w * w * den);
        let a = 2.0 * k * (w * tau / 2.0).sin() * c;
        Self {
            k1: f0 * c / (2.0 * w * w),
            k2: f0 * c / (2.0 * den),
            h: C64::from_polar(a, w * tau / 2.0),
        }
    }

    /// (x, dx/dt) under the hammer, 0 <= t <= tau, with om = 2 pi / tau.
    #[inline]
    pub fn contact(&self, w: f64, om: f64, t: f64) -> (f64, f64) {
        let (sw, cw) = (w * t).sin_cos();
        let (so, co) = (om * t).sin_cos();
        (
            self.k1 * (1.0 - cw) - self.k2 * (co - cw),
            self.k1 * w * sw - self.k2 * (w * sw - om * so),
        )
    }
}
