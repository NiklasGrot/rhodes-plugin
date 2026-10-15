# rhodes-plugin

Real-time Rhodes electric piano as a VST3/CLAP instrument, built with
[nice-plug](https://codeberg.org/RustAudio/nice-plug). It runs a modal model with
parameters fitted to recordings: a hammer excites the tine modes, and their motion goes
through the magnetic pickup nonlinearity, eps = -kappa dPsi(x, p_d)/dt.

## Download

The latest release (VST3 and CLAP, unzip into your plugin folder):
[macOS](https://github.com/NiklasGrot/rhodes-plugin/releases/latest/download/Rhodes-macos.zip) ·
[Windows](https://github.com/NiklasGrot/rhodes-plugin/releases/latest/download/Rhodes-windows.zip) ·
[Linux](https://github.com/NiklasGrot/rhodes-plugin/releases/latest/download/Rhodes-linux.zip)

> **macOS:** the plugin is not notarized, so macOS quarantines it after download and
> DAWs fail to load it. Clear the flag once after installing:
>
> ```bash
> sudo xattr -dr com.apple.quarantine /Library/Audio/Plug-Ins/VST3/Rhodes.vst3 /Library/Audio/Plug-Ins/CLAP/Rhodes.clap
> ```

## Layout

| Path      | Contents                                                              |
| --------- | --------------------------------------------------------------------- |
| `dsp/`    | `rhodes-dsp`: the model with no plugin dependencies, testable offline |
| `plugin/` | `rhodes`: the nice-plug shell (params, MIDI, voices)                  |
| `xtask/`  | `cargo xtask bundle`                                                  |
| `assets/` | Fitted parameters and the Psi lookup table, compiled into the binary  |

## Assets

The fitted data is not edited by hand.

- `model.json` holds the parameters for all 73 keys (MIDI 28-100): the fundamental's
  frequency, decay and gain, the inharmonic modes (near 7.1, 20.4 and 39.7 f0) with one
  gain per dynamic, the hammer velocity of each dynamic, the hammer contact times, p_d,
  p_o, kappa and the detuning flags. `source` records the commit the fit came from.
- `psi.f64` is the Psi(x, z) flux table (96 x 9755), stored as little-endian f64. The voltage depends on dPsi/dx, and in the flat regions Psi changes by about 1e-5
  per cell, which is too little for f32 to resolve.

## Model

The DSP lives in `rhodes-dsp` (`dsp/src`).

- **Keys** (`voice.rs`). There is one voice per physical key, 73 in total. The pickup is
  nonlinear, so a repeated strike adds to the key's moving modes (displacement) rather
  than starting a second voice whose audio would be summed.
- **Hammer** (`hammer.rs`). A raised-cosine (sin^2) force pulse of length
  tau = tau0 vel^-beta, using the closed-form forced response. At t = tau it hands the
  amplitude and phase to the free modes.
- **Modes**. The fundamental and the inharmonic modes run as damped rotating phasors,
  which give the displacement x and the velocity x' together. The gain of each
  inharmonic mode is interpolated linearly over the fitted velocities 0.37 to 1 (p to f).
  Modes above 0.45 fs are dropped.
- **Pickup**. eps = -kappa dPsi/dx(x + p_o, p_d) x'. The slope comes from the Catmull-Rom
  interpolation of the table.
- **Dampers**. These are not part of the fit. On release, and unless the sustain pedal
  (CC64) is down, every mode of the key gets an extra decay rate of ln(1000) / T60, ramped
  in over 5 ms. T60 is fixed at 0.15 s.

## Build

```bash
cargo test -p rhodes-dsp
```

```bash
cargo xtask bundle rhodes --release
```

The bundles are written to `target/bundled/Rhodes.{vst3,clap}`.
