//! Modal Rhodes model: hammer, tine modes and magnetic pickup.
//!
//! Signal chain per voice:
//!
//! 1. **Hammer** -- raised-cosine force pulse of length tau = tau0 vel^-beta. While in contact
//!    each mode follows the closed-form forced response; at t = tau it hands over amplitude
//!    and phase to the free oscillation.
//! 2. **Tine modes** -- fundamental plus the fitted inharmonic modes (near 7.1, 20.4,
//!    39.7 f0), each a decaying sinusoid. Their sum is the tip displacement
//!    x(t) = p_o + sum_q q_q(t).
//! 3. **Pickup** -- eps = -kappa dPsi(x, p_d)/dt through the tabulated flux [`PsiTable`].
//!
//! Everything fitted lives in `assets/` and is compiled in.

mod cplx;
pub mod engine;
pub mod hammer;
pub mod model;
pub mod pickup;
pub mod voice;

pub use engine::Engine;
pub use model::Model;
pub use pickup::PsiTable;
