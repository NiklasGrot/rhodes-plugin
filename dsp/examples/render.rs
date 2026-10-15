//! Render a short phrase offline, as a listening test and CPU check.
//!
//!     cargo run -p rhodes-dsp --release --example render -- [out.wav]

use rhodes_dsp::Engine;
use std::time::Instant;

const FS: u32 = 48000;
const GAIN: f32 = 2.0; // +6 dB, the plugin's default

/// (time s, MIDI note, velocity or None for note off)
fn phrase() -> Vec<(f64, u8, Option<f32>)> {
    let mut ev = Vec::new();
    // Velocity ladder on one key: the fitted p, mp, mf, f
    for (i, v) in [0.37, 0.64, 0.82, 1.0].into_iter().enumerate() {
        let t = i as f64 * 0.6;
        ev.push((t, 57, Some(v)));
        ev.push((t + 0.45, 57, None));
    }
    // Chords: Fmaj9, Em7, Dm9, Cmaj7 (bass + voicing)
    let chords: [&[u8]; 4] = [
        &[41, 57, 60, 64, 67],
        &[40, 55, 59, 62, 67],
        &[38, 53, 57, 60, 64],
        &[36, 52, 55, 59, 64],
    ];
    for (i, c) in chords.iter().enumerate() {
        let t = 3.0 + i as f64 * 1.5;
        for (j, &k) in c.iter().enumerate() {
            ev.push((t + 0.012 * j as f64, k, Some(0.55 + 0.1 * (j % 2) as f32)));
            ev.push((t + 1.4, k, None));
        }
    }
    // Run up the keyboard, full range
    for (i, k) in (28..=100).step_by(3).enumerate() {
        let t = 9.5 + i as f64 * 0.12;
        ev.push((t, k, Some(0.7)));
        ev.push((t + 0.3, k, None));
    }
    ev.sort_by(|a, b| a.0.total_cmp(&b.0));
    ev
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "render.wav".into());
    let mut engine = Engine::bundled(FS as f64);
    let events = phrase();
    let n = (13.5 * FS as f64) as usize;
    let mut out = vec![0.0f32; n];

    let start = Instant::now();
    let mut pos = 0;
    for &(t, key, vel) in events
        .iter()
        .chain(std::iter::once(&(n as f64 / FS as f64, 0, None)))
    {
        let at = ((t * FS as f64) as usize).min(n);
        engine.render(&mut out[pos..at]);
        pos = at;
        match vel {
            Some(v) => engine.note_on(key, v),
            None => engine.note_off(key),
        }
    }
    let secs = start.elapsed().as_secs_f64();

    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs())) * GAIN;
    for s in &mut out {
        *s *= GAIN;
    }
    write_wav(&path, &out);
    println!(
        "{path}: {:.1} s of audio in {:.3} s ({:.0}x real time), peak {peak:.3}",
        n as f64 / FS as f64,
        secs,
        n as f64 / FS as f64 / secs
    );
}

/// Mono 32-bit float WAV.
fn write_wav(path: &str, x: &[f32]) {
    let data = (x.len() * 4) as u32;
    let mut b = Vec::with_capacity(44 + data as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&FS.to_le_bytes());
    b.extend_from_slice(&(FS * 4).to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for s in x {
        b.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, b).expect("write wav");
}
