//! Magnetic pickup: the flux Psi(x, z) seen by the coil for a tine tip at (x, 0, z).
//!
//! Psi is tabulated offline on a uniform grid and read back with Catmull-Rom in both
//! axes. The coil voltage is eps = -kappa dPsi/dt = -kappa (dPsi/dx x' + dPsi/dz z'), so
//! the lookup returns dPsi/dx together with Psi: the tine velocity x' is known
//! analytically from the modes, and multiplying it with the spline slope gives the voltage
//! without differencing samples.

use crate::model::TableLayout;

#[derive(Debug, Clone)]
pub struct PsiTable {
    x0: f64,
    dx: f64,
    z0: f64,
    dz: f64,
    n_x: usize,
    n_z: usize,
    /// Row-major (n_z, n_x).
    v: Vec<f64>,
}

impl PsiTable {
    pub fn from_le_bytes(layout: &TableLayout, bytes: &[u8]) -> Self {
        assert_eq!(
            bytes.len(),
            8 * layout.n_x * layout.n_z,
            "psi table size mismatch"
        );
        assert!(
            layout.n_x >= 4 && layout.n_z >= 4,
            "Catmull-Rom needs 4 points per axis"
        );
        let v = bytes
            .chunks_exact(8)
            .map(|b| f64::from_le_bytes(b.try_into().unwrap()))
            .collect();
        Self {
            x0: layout.x0,
            dx: layout.dx,
            z0: layout.z0,
            dz: layout.dz,
            n_x: layout.n_x,
            n_z: layout.n_z,
            v,
        }
    }

    /// (Psi, dPsi/dx) at tip position (x, z), both in metres.
    #[inline]
    pub fn eval(&self, x: f64, z: f64) -> (f64, f64) {
        let (ix, ux) = cell((x - self.x0) / self.dx, self.n_x);
        let (iz, uz) = cell((z - self.z0) / self.dz, self.n_z);

        let mut val = [0.0; 4];
        let mut slope = [0.0; 4];
        for (k, (val, slope)) in val.iter_mut().zip(&mut slope).enumerate() {
            let base = (iz + k - 1) * self.n_x + ix - 1;
            let r = &self.v[base..base + 4];
            (*val, *slope) = catmull([r[0], r[1], r[2], r[3]], ux);
        }
        let (psi, _) = catmull(val, uz);
        let (dpsi_du, _) = catmull(slope, uz);
        (psi, dpsi_du / self.dx)
    }
}

/// Grid coordinate -> (base index with a valid 4-point stencil, fraction). Like
/// model._cell: outside the grid the end cubic is extrapolated, clamped at the last point.
#[inline]
fn cell(s: f64, n: usize) -> (usize, f64) {
    let s = s.clamp(0.0, (n - 1) as f64);
    let i = (s.floor() as usize).clamp(1, n - 3);
    (i, s - i as f64)
}

/// Cubic through p[1] (u = 0) and p[2] (u = 1), tangents from p[0] and p[3]:
/// (value, d/du).
#[inline]
fn catmull(p: [f64; 4], u: f64) -> (f64, f64) {
    let a = 2.0 * p[1];
    let b = p[2] - p[0];
    let c = 2.0 * p[0] - 5.0 * p[1] + 4.0 * p[2] - p[3];
    let d = -p[0] + 3.0 * p[1] - 3.0 * p[2] + p[3];
    (
        0.5 * (a + u * (b + u * (c + u * d))),
        0.5 * (b + u * (2.0 * c + u * 3.0 * d)),
    )
}
