//! The whole instrument: one [`KeyVoice`] per key, sustain pedal and dampers.

use crate::model::Model;
use crate::pickup::PsiTable;
use crate::voice::{KeyVoice, MAX_DYNS};

/// Samples rendered per inner block (f64 scratch on the stack).
const CHUNK: usize = 64;
/// Time for a released key to fall by 60 dB under its damper (s).
const DAMPER_T60: f64 = 0.15;

pub struct Engine {
    table: PsiTable,
    kappa: f64,
    beta: f64,
    vels: [f64; MAX_DYNS],
    n_dyns: usize,
    first_midi: u8,
    keys: Vec<KeyVoice>,
    sustain: bool,
    /// Decay rate a released key's damper adds (1/s).
    lam_damper: f64,
}

impl Engine {
    pub fn new(model: &Model, table: PsiTable, sample_rate: f64) -> Self {
        let n_dyns = model.vels.len();
        assert!(n_dyns <= MAX_DYNS);
        let mut vels = [0.0; MAX_DYNS];
        vels[..n_dyns].copy_from_slice(&model.vels);
        let mut engine = Self {
            table,
            kappa: model.kappa,
            beta: model.beta,
            vels,
            n_dyns,
            first_midi: model.keys[0].midi,
            keys: model
                .keys
                .iter()
                .map(|k| KeyVoice::new(k, n_dyns))
                .collect(),
            sustain: false,
            lam_damper: 1000f64.ln() / DAMPER_T60,
        };
        engine.set_sample_rate(sample_rate);
        engine
    }

    /// The model compiled into the binary.
    pub fn bundled(sample_rate: f64) -> Self {
        let (model, table) = Model::bundled();
        Self::new(&model, table, sample_rate)
    }

    pub fn set_sample_rate(&mut self, fs: f64) {
        for k in &mut self.keys {
            k.set_sample_rate(fs);
        }
    }

    pub fn reset(&mut self) {
        self.sustain = false;
        for k in &mut self.keys {
            k.reset();
        }
    }

    fn key(&mut self, midi: u8) -> Option<&mut KeyVoice> {
        self.keys
            .get_mut(midi.checked_sub(self.first_midi)? as usize)
    }

    /// Strike a key; velocity in [0, 1] is the hammer velocity of the fit (p .. f are
    /// 0.37 .. 1). Notes outside the 73 keys are ignored.
    pub fn note_on(&mut self, midi: u8, velocity: f32) {
        let (beta, vels, n) = (self.beta, self.vels, self.n_dyns);
        if let Some(k) = self.key(midi) {
            k.held = true;
            k.damped = false;
            k.strike((velocity as f64).clamp(0.01, 1.0), beta, &vels[..n]);
        }
    }

    pub fn note_off(&mut self, midi: u8) {
        let sustain = self.sustain;
        if let Some(k) = self.key(midi) {
            k.held = false;
            k.damped = !sustain;
        }
    }

    /// Silence a key at once (host choke).
    pub fn choke(&mut self, midi: u8) {
        if let Some(k) = self.key(midi) {
            k.reset();
        }
    }

    /// Sustain pedal: lifts every damper while down.
    pub fn set_sustain(&mut self, down: bool) {
        self.sustain = down;
        for k in &mut self.keys {
            k.damped = !down && !k.held;
        }
    }

    pub fn active_keys(&self) -> usize {
        self.keys.iter().filter(|k| k.is_active()).count()
    }

    /// Render into `out`, overwriting it.
    pub fn render(&mut self, out: &mut [f32]) {
        for block in out.chunks_mut(CHUNK) {
            let buf = &mut [0.0f64; CHUNK][..block.len()];
            self.render_f64(buf);
            for (o, b) in block.iter_mut().zip(buf.iter()) {
                *o = *b as f32;
            }
        }
    }

    /// Render, adding to `out`.
    pub fn render_f64(&mut self, out: &mut [f64]) {
        for k in &mut self.keys {
            k.render(&self.table, self.kappa, self.lam_damper, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A released key dies away under its damper and goes idle.
    #[test]
    fn damper_silences_released_key() {
        let mut engine = Engine::bundled(48000.0);
        engine.note_on(60, 0.8);
        let mut buf = vec![0.0; 4800];
        engine.render_f64(&mut buf);
        engine.note_off(60);
        for _ in 0..40 {
            buf.fill(0.0);
            engine.render_f64(&mut buf);
        }
        assert_eq!(engine.active_keys(), 0);
    }
}
