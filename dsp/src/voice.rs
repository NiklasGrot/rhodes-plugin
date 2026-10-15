//! One physical key: its tine modes, the strike in progress and the damper.
//!
//! The pickup is nonlinear, so everything that moves one tine must be summed as
//! displacement before the pickup, not as audio after it. A key therefore owns a single
//! set of mode phasors; a repeated strike adds to them, as a hammer hitting a still
//! ringing tine does in the (linear) mode model.

use crate::cplx::C64;
use crate::hammer::{ModeStrike, contact_time};
use crate::model::Key;
use crate::pickup::PsiTable;
use std::f64::consts::TAU;

/// Fundamental plus up to five inharmonic modes (m2 .. m6).
pub const MAX_MODES: usize = 6;
/// Number of fitted dynamics a mode gain is given at.
pub const MAX_DYNS: usize = 4;

/// Modes above this fraction of the sample rate are dropped rather than aliased.
const MAX_MODE_FRAC: f64 = 0.45;
/// A key whose summed mode amplitude falls below this (m) goes idle.
const SILENT: f64 = 1e-10;

/// A strike still in hammer contact.
#[derive(Debug, Clone, Copy)]
struct Strike {
    /// Samples since the onset.
    n: u32,
    tau: f64,
    om: f64,
    modes: [ModeStrike; MAX_MODES],
}

#[derive(Debug, Clone)]
pub struct KeyVoice {
    // Fitted, per key
    p_o: f64,
    p_d: f64,
    tau0: f64,
    n_modes: usize,
    w: [f64; MAX_MODES],
    lam: [f64; MAX_MODES],
    /// Gain per mode and dynamic.
    c: [[f64; MAX_DYNS]; MAX_MODES],

    // Per sample rate
    dt: f64,
    /// Modes below MAX_MODE_FRAC fs (a prefix, the modes ascend in frequency).
    n_active: usize,
    /// One-sample free rotation e^{(-lam + i w) dt}.
    r: [C64; MAX_MODES],
    /// One-pole coefficient of the damper ramp.
    damp_smooth: f64,

    // State
    /// Free mode phasors: q_q = Im z_q.
    z: [C64; MAX_MODES],
    strike: Option<Strike>,
    /// Extra decay rate the damper felt applies to every mode (1/s), ramped to its target.
    lam_d: f64,
    pub(crate) damped: bool,
    pub(crate) held: bool,
    active: bool,
}

impl KeyVoice {
    pub fn new(key: &Key, n_dyns: usize) -> Self {
        assert!(n_dyns <= MAX_DYNS && key.modes.len() < MAX_MODES);
        let mut v = Self {
            p_o: key.p_o,
            p_d: key.p_d,
            tau0: key.tau0,
            n_modes: 1 + key.modes.len(),
            w: [0.0; MAX_MODES],
            lam: [0.0; MAX_MODES],
            c: [[0.0; MAX_DYNS]; MAX_MODES],
            dt: 0.0,
            n_active: 0,
            r: [C64::ZERO; MAX_MODES],
            damp_smooth: 0.0,
            z: [C64::ZERO; MAX_MODES],
            strike: None,
            lam_d: 0.0,
            damped: false,
            held: false,
            active: false,
        };
        // The fundamental's gain is the same at every dynamic: the hammer model alone
        // scales it with vel. The inharmonic modes carry one gain per dynamic.
        v.w[0] = TAU * key.f0;
        v.lam[0] = key.sigma0;
        v.c[0] = [key.c0; MAX_DYNS];
        for (q, m) in key.modes.iter().enumerate() {
            v.w[q + 1] = TAU * m.f;
            v.lam[q + 1] = m.sigma;
            v.c[q + 1][..n_dyns].copy_from_slice(&m.c[..n_dyns]);
        }
        v
    }

    pub fn set_sample_rate(&mut self, fs: f64) {
        self.dt = 1.0 / fs;
        self.n_active = (0..self.n_modes)
            .take_while(|&q| self.w[q] / TAU < MAX_MODE_FRAC * fs)
            .count();
        for q in 0..MAX_MODES {
            self.r[q] = C64::exp(-self.lam[q] * self.dt, self.w[q] * self.dt);
        }
        self.damp_smooth = 1.0 - (-self.dt / 5e-3).exp(); // 5 ms
    }

    pub fn reset(&mut self) {
        self.z = [C64::ZERO; MAX_MODES];
        self.strike = None;
        self.lam_d = 0.0;
        self.damped = false;
        self.held = false;
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Hammer strike. vel in (0, 1]; `vels` are the velocities the dynamics were fitted
    /// at, and a mode's gain is interpolated linearly between them (held beyond the ends).
    pub fn strike(&mut self, vel: f64, beta: f64, vels: &[f64]) {
        // A strike still in contact is handed over early. It only ever overlaps by the
        // ~1 ms contact time, and doing so keeps a single strike slot per key.
        if let Some(s) = self.strike.take() {
            self.hand_over(&s);
        }
        let tau = contact_time(self.tau0, beta, vel);
        let mut modes = [ModeStrike::default(); MAX_MODES];
        for (q, m) in modes.iter_mut().enumerate().take(self.n_active) {
            *m = ModeStrike::new(self.w[q], interp(vels, &self.c[q], vel), vel, tau);
        }
        self.strike = Some(Strike {
            n: 0,
            tau,
            om: TAU / tau,
            modes,
        });
        self.active = true;
    }

    /// Adds the free continuation of `s` at its current time to the mode phasors.
    fn hand_over(&mut self, s: &Strike) {
        let st = s.n as f64 * self.dt - s.tau;
        for q in 0..self.n_active {
            self.z[q] += s.modes[q].h * C64::exp(-self.lam[q] * st, self.w[q] * st);
        }
    }

    /// Adds the pickup voltage of this key to `out`. `lam_damper` is the damper decay
    /// rate the key moves to while `damped`.
    pub fn render(&mut self, table: &PsiTable, kappa: f64, lam_damper: f64, out: &mut [f64]) {
        if !self.active {
            return;
        }
        let lam_target = if self.damped { lam_damper } else { 0.0 };
        for o in out.iter_mut() {
            let (mut x, mut v) = (0.0, 0.0);

            if let Some(s) = &mut self.strike {
                let t = s.n as f64 * self.dt;
                if t < s.tau {
                    for q in 0..self.n_active {
                        let (xq, vq) = s.modes[q].contact(self.w[q], s.om, t);
                        x += xq;
                        v += vq;
                    }
                    s.n += 1;
                } else {
                    let s = *s;
                    self.hand_over(&s);
                    self.strike = None;
                }
            }

            self.lam_d += (lam_target - self.lam_d) * self.damp_smooth;
            let lam_d = self.lam_d;
            let damp = if lam_d > 0.0 {
                (-lam_d * self.dt).exp()
            } else {
                1.0
            };
            for q in 0..self.n_active {
                let z = self.z[q];
                // q = Im z, dq/dt = Im((-(lam + lam_d) + i w) z)
                x += z.im;
                v += self.w[q] * z.re - (self.lam[q] + lam_d) * z.im;
                self.z[q] = z * self.r[q] * damp;
            }

            let (_, dpsi_dx) = table.eval(x + self.p_o, self.p_d);
            *o += -kappa * dpsi_dx * v;
        }

        if self.strike.is_none() {
            let amp: f64 = self.z[..self.n_active].iter().map(|z| z.norm()).sum();
            if amp < SILENT {
                self.z = [C64::ZERO; MAX_MODES];
                self.active = false;
            }
        }
    }
}

/// Piecewise-linear y(x) through (xs, ys), held constant outside [xs[0], xs[last]].
fn interp(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let n = xs.len();
    if x <= xs[0] {
        return ys[0];
    }
    for i in 1..n {
        if x <= xs[i] {
            let u = (x - xs[i - 1]) / (xs[i] - xs[i - 1]);
            return ys[i - 1] + u * (ys[i] - ys[i - 1]);
        }
    }
    ys[n - 1]
}
