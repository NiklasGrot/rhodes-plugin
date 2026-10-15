use nice_plug::midi::Key;
use nice_plug::prelude::*;
use rhodes_dsp::Engine;
use std::sync::Arc;

pub struct Rhodes {
    params: Arc<RhodesParams>,
    engine: Engine,
}

#[derive(Params)]
struct RhodesParams {
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for RhodesParams {
    fn default() -> Self {
        Self {
            // One key at full velocity peaks around 0.1 and a five-note chord around 0.3,
            // hence the default above 0 dB.
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(6.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-24.0),
                    max: util::db_to_gain(24.0),
                    factor: FloatRange::gain_skew_factor(-24.0, 24.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Default for Rhodes {
    fn default() -> Self {
        Self {
            params: Arc::new(RhodesParams::default()),
            engine: Engine::bundled(48000.0),
        }
    }
}

impl Rhodes {
    fn handle_event(&mut self, event: NoteEvent<()>) {
        // A wildcard key addresses every key.
        let keys = |key: Key| match key {
            Key::Number(k) => k..=k,
            Key::Wildcard => 0..=127,
        };
        match event {
            NoteEvent::NoteOn {
                key: Key::Number(k),
                velocity,
                ..
            } => self.engine.note_on(k, velocity),
            NoteEvent::NoteOff { key, .. } => keys(key).for_each(|k| self.engine.note_off(k)),
            NoteEvent::Choke { key, .. } => keys(key).for_each(|k| self.engine.choke(k)),
            NoteEvent::MidiCC {
                cc: control_change::DAMPER_PEDAL,
                value,
                ..
            } => self.engine.set_sustain(value >= 0.5),
            _ => {}
        }
    }
}

impl Plugin for Rhodes {
    const NAME: &'static str = "Rhodes";
    const VENDOR: &'static str = "Niklas Grotzeck";
    const URL: &'static str = "";
    const EMAIL: &'static str = "n.grotzeck@im-en.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    // MidiCCs rather than Basic for the sustain pedal.
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type Editor = ();
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn activate(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl ActivateContext<Self>,
    ) -> bool {
        self.engine
            .set_sample_rate(buffer_config.sample_rate as f64);
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let n = buffer.samples();
        let out = buffer.as_slice();
        let (mono, rest) = out.split_first_mut().expect("stereo output");

        // Render up to each event, then apply it: note timing is sample accurate.
        let mut pos = 0;
        let mut next = context.next_event();
        while pos < n {
            while let Some(event) = next.filter(|e| e.timing() as usize <= pos) {
                self.handle_event(event);
                next = context.next_event();
            }
            let end = next.map_or(n, |e| (e.timing() as usize).min(n));
            self.engine.render(&mut mono[pos..end]);
            pos = end;
        }

        for s in mono.iter_mut() {
            *s *= self.params.gain.smoothed.next();
        }
        for channel in rest {
            channel.copy_from_slice(mono);
        }

        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for Rhodes {
    const CLAP_ID: &'static str = "com.niklasgrotzeck.rhodes";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Physically modelled Rhodes electric piano");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for Rhodes {
    const VST3_CLASS_ID: [u8; 16] = *b"NGrotzeckRhodes1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Piano,
        Vst3SubCategory::Stereo,
    ];
}

nice_export_clap!(Rhodes);
nice_export_vst3!(Rhodes);
