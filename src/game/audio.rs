use super::*;
use macroquad::audio::{
    PlaySoundParams, Sound, load_sound_from_bytes, play_sound, play_sound_once,
};

pub(super) struct AudioBank {
    music: Option<Sound>,
    fire: Option<Sound>,
    explosion: Option<Sound>,
    smart_bomb: Option<Sound>,
    ui_move: Option<Sound>,
    start: Option<Sound>,
    game_over: Option<Sound>,
    music_started: bool,
}

impl AudioBank {
    pub(super) async fn load() -> Self {
        Self {
            music: load_sound_from_bytes(&make_music_wav()).await.ok(),
            fire: load_sound_from_bytes(&make_tone_wav(430.0, 0.08, 0.25, Waveform::Pulse(0.35)))
                .await
                .ok(),
            explosion: load_sound_from_bytes(&make_noise_wav(0.22, 0.40))
                .await
                .ok(),
            smart_bomb: load_sound_from_bytes(&make_smart_bomb_wav()).await.ok(),
            ui_move: load_sound_from_bytes(&make_tone_wav(780.0, 0.05, 0.12, Waveform::Sine))
                .await
                .ok(),
            start: load_sound_from_bytes(&make_start_wav()).await.ok(),
            game_over: load_sound_from_bytes(&make_game_over_wav()).await.ok(),
            music_started: false,
        }
    }

    pub(super) fn ensure_music(&mut self) {
        if self.music_started {
            return;
        }
        if let Some(sound) = &self.music {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: true,
                    volume: 0.28,
                },
            );
            self.music_started = true;
        }
    }

    pub(super) fn play_fire(&self) {
        if let Some(sound) = &self.fire {
            play_sound_once(sound);
        }
    }

    pub(super) fn play_explosion(&self) {
        if let Some(sound) = &self.explosion {
            play_sound_once(sound);
        }
    }

    pub(super) fn play_smart_bomb(&self) {
        if let Some(sound) = &self.smart_bomb {
            play_sound_once(sound);
        }
    }

    pub(super) fn play_ui_move(&self) {
        if let Some(sound) = &self.ui_move {
            play_sound_once(sound);
        }
    }

    pub(super) fn play_start(&self) {
        if let Some(sound) = &self.start {
            play_sound_once(sound);
        }
    }

    pub(super) fn play_game_over(&self) {
        if let Some(sound) = &self.game_over {
            play_sound_once(sound);
        }
    }
}

enum Waveform {
    Sine,
    Pulse(f32),
}

fn make_tone_wav(freq: f32, seconds: f32, volume: f32, waveform: Waveform) -> Vec<u8> {
    let sample_rate = 22_050;
    let total = (sample_rate as f32 * seconds) as usize;
    let mut samples = Vec::with_capacity(total);

    for index in 0..total {
        let t = index as f32 / sample_rate as f32;
        let env = (1.0 - index as f32 / total as f32).powf(1.4);
        let signal = match waveform {
            Waveform::Sine => (2.0 * PI * freq * t).sin(),
            Waveform::Pulse(duty) => {
                let phase = (freq * t).fract();
                if phase < duty { 1.0 } else { -1.0 }
            }
        };
        samples.push(signal * env * volume);
    }

    make_pcm_wav(sample_rate, &samples)
}

fn make_noise_wav(seconds: f32, volume: f32) -> Vec<u8> {
    let sample_rate = 22_050;
    let total = (sample_rate as f32 * seconds) as usize;
    let mut seed = 0x1234_5678u32;
    let mut samples = Vec::with_capacity(total);

    for index in 0..total {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = ((seed >> 8) as f32 / (u32::MAX >> 8) as f32) * 2.0 - 1.0;
        let env = (1.0 - index as f32 / total as f32).powf(2.2);
        samples.push(noise * env * volume);
    }

    make_pcm_wav(sample_rate, &samples)
}

fn make_start_wav() -> Vec<u8> {
    let sample_rate = 22_050;
    let notes = [(220.0, 0.07), (330.0, 0.07), (440.0, 0.10), (660.0, 0.14)];
    let mut samples = Vec::new();
    for (freq, seconds) in notes {
        samples.extend(make_tone_samples(
            freq,
            seconds,
            0.18,
            Waveform::Pulse(0.4),
            sample_rate,
        ));
    }
    make_pcm_wav(sample_rate, &samples)
}

fn make_game_over_wav() -> Vec<u8> {
    let sample_rate = 22_050;
    let notes = [(330.0, 0.08), (248.0, 0.10), (196.0, 0.14), (147.0, 0.18)];
    let mut samples = Vec::new();
    for (freq, seconds) in notes {
        samples.extend(make_tone_samples(
            freq,
            seconds,
            0.17,
            Waveform::Pulse(0.55),
            sample_rate,
        ));
    }
    make_pcm_wav(sample_rate, &samples)
}

fn make_smart_bomb_wav() -> Vec<u8> {
    let sample_rate = 22_050;
    let total = (sample_rate as f32 * 0.22) as usize;
    let mut samples = Vec::with_capacity(total);
    for index in 0..total {
        let t = index as f32 / sample_rate as f32;
        let freq = 300.0 + 900.0 * (index as f32 / total as f32);
        let env = (1.0 - index as f32 / total as f32).powf(1.1);
        let value = (2.0 * PI * freq * t).sin();
        samples.push(value * env * 0.18);
    }
    make_pcm_wav(sample_rate, &samples)
}

fn make_music_wav() -> Vec<u8> {
    let sample_rate = 22_050;
    let step_seconds = 0.18;
    let lead = [220.0, 330.0, 440.0, 330.0, 246.0, 369.0, 493.0, 369.0];
    let bass = [110.0, 110.0, 123.0, 123.0, 98.0, 98.0, 82.0, 82.0];
    let total = (sample_rate as f32 * step_seconds * lead.len() as f32) as usize;
    let mut samples = Vec::with_capacity(total);

    for (step, lead_freq) in lead.iter().enumerate() {
        let bass_freq = bass[step];
        let step_samples = (sample_rate as f32 * step_seconds) as usize;
        for sample in 0..step_samples {
            let t = sample as f32 / sample_rate as f32;
            let env = (1.0 - sample as f32 / step_samples as f32).powf(0.7);
            let lead_value = if (lead_freq * t).fract() < 0.35 {
                1.0
            } else {
                -1.0
            };
            let bass_value = (2.0 * PI * bass_freq * t).sin();
            samples.push((lead_value * 0.10 + bass_value * 0.06) * env);
        }
    }

    make_pcm_wav(sample_rate, &samples)
}

fn make_tone_samples(
    freq: f32,
    seconds: f32,
    volume: f32,
    waveform: Waveform,
    sample_rate: u32,
) -> Vec<f32> {
    let total = (sample_rate as f32 * seconds) as usize;
    let mut samples = Vec::with_capacity(total);
    for index in 0..total {
        let t = index as f32 / sample_rate as f32;
        let env = (1.0 - index as f32 / total as f32).powf(1.3);
        let signal = match waveform {
            Waveform::Sine => (2.0 * PI * freq * t).sin(),
            Waveform::Pulse(duty) => {
                let phase = (freq * t).fract();
                if phase < duty { 1.0 } else { -1.0 }
            }
        };
        samples.push(signal * env * volume);
    }
    samples
}

fn make_pcm_wav(sample_rate: u32, samples: &[f32]) -> Vec<u8> {
    let byte_rate = sample_rate * 2;
    let data_size = (samples.len() * 2) as u32;
    let riff_size = 36 + data_size;
    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());

    for sample in samples {
        let clamped = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        bytes.extend_from_slice(&clamped.to_le_bytes());
    }

    bytes
}
