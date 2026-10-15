//! Fitted model parameters: per key the fundamental, pickup geometry, hammer contact time
//! and the inharmonic modes, plus the shared kappa, beta and the Psi table.

use serde::Deserialize;

use crate::pickup::PsiTable;

static MODEL_JSON: &str = include_str!("../../assets/model.json");
static PSI_F64: &[u8] = include_bytes!("../../assets/psi.f64");

#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    /// Names of the recorded dynamics, softest first (p, mp, mf, f).
    pub dyns: Vec<String>,
    /// Hammer velocity (0-1] of each dynamic.
    pub vels: Vec<f64>,
    /// Contact time exponent: tau = tau0 * vel^-beta.
    pub beta: f64,
    /// Output scale of the pickup voltage, eps = -kappa dPsi/dt.
    pub kappa: f64,
    pub pickup: Pickup,
    pub table: TableLayout,
    /// One entry per key, ascending MIDI note.
    pub keys: Vec<Key>,
}

/// Pickup geometry the table was computed for (fixed from photos, not fitted).
#[derive(Debug, Clone, Deserialize)]
pub struct Pickup {
    /// Pole piece radius (m).
    pub r: f64,
    /// Half-width of the flat plateau (m).
    pub a: f64,
    /// Chamfer slope.
    pub m: f64,
    pub surf_n: usize,
    /// |displacement| the table is guaranteed to cover (m).
    pub disp_max: f64,
    /// Largest offset p_o the table covers (m).
    pub po_max: f64,
    /// Tabulated range of the tine-pickup distance p_d (m).
    pub pd_range: [f64; 2],
}

/// Uniform (x, z) grid of the Psi table: `V[iz * n_x + ix] = Psi(x0 + ix dx, z0 + iz dz)`.
#[derive(Debug, Clone, Deserialize)]
pub struct TableLayout {
    pub x0: f64,
    pub dx: f64,
    pub z0: f64,
    pub dz: f64,
    pub n_x: usize,
    pub n_z: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Key {
    pub midi: u8,
    /// Sample library name (one octave below the MIDI name).
    pub name: String,
    /// Sample detuned by more than 8 cents (F0, G0, D6); its c0 is interpolated.
    #[serde(default)]
    pub flag: bool,
    /// Fundamental frequency (Hz) and decay rate (1/s).
    pub f0: f64,
    pub sigma0: f64,
    /// Fundamental gain in the hammer model (displacement per unit impulse).
    pub c0: f64,
    /// Hammer contact time at vel = 1 (s).
    pub tau0: f64,
    /// Tine-pickup distance and lateral offset (m).
    pub p_d: f64,
    pub p_o: f64,
    /// Fitted inharmonic modes, lowest first.
    pub modes: Vec<Mode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Mode {
    /// Mode label (m2 .. m6).
    #[serde(default)]
    pub name: String,
    /// Ratio to f0 (Gabrielli 2020: 7.1, 20.4, 39.7, 62.9, 93.1).
    pub ratio: f64,
    /// Fitted frequency (Hz) and decay rate (1/s).
    pub f: f64,
    pub sigma: f64,
    /// Gain per dynamic, same order as [`Model::dyns`].
    pub c: Vec<f64>,
}

impl Model {
    /// The model compiled into the binary, together with its pickup table.
    pub fn bundled() -> (Model, PsiTable) {
        let model: Model = serde_json::from_str(MODEL_JSON).expect("assets/model.json is valid");
        let table = PsiTable::from_le_bytes(&model.table, PSI_F64);
        (model, table)
    }

    /// Key for a MIDI note, if the instrument has one (73 keys, MIDI 28-100).
    pub fn key(&self, midi: u8) -> Option<&Key> {
        let first = self.keys.first()?.midi;
        self.keys.get(midi.checked_sub(first)? as usize)
    }
}
