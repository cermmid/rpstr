//! Recorder audio w dedykowanym wątku.
//!
//! `cpal::Stream` jest `!Send`, więc nie da się go trzymać w state Tauri, który
//! jest współdzielony przez wątki async runtime.  Rozwiązanie: wątek recordera
//! żyje cały czas, a command'y (`start_recording`, `stop_recording`) rozmawiają
//! z nim przez `mpsc::channel`.
//!
//! Bufor PCM (f32 mono, w natywnym sample rate mikrofonu) trzymany jest w
//! `Arc<Mutex<Vec<f32>>>` — wątek recordera i wątek wywołujący stop mogą do
//! niego jednocześnie zajrzeć.  `Pcm` zwracany ze stop-a niesie sample rate,
//! którym whisper-wrapper zrobi resample do 16 kHz.
//!
//! Tryb bez feature `audio-live`: cały kod po stronie cpal jest zastąpiony
//! szybkim stubem, który zwraca pusty bufor.  Dzięki temu CI/Linux build bez
//! ALSA w ogóle się kompilują.

use parking_lot::Mutex;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
#[cfg(feature = "audio-live")]
use std::time::Instant;

/// Surowy PCM — f32 mono, natywny sample rate mikrofonu.
#[derive(Clone, Debug)]
pub struct Pcm {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub duration_ms: u64,
}

enum Cmd {
    Start(Sender<Result<(), String>>),
    Stop(Sender<Result<Pcm, String>>),
}

pub struct AudioController {
    tx: Sender<Cmd>,
}

impl AudioController {
    pub fn spawn() -> Self {
        let (tx, rx) = channel::<Cmd>();
        thread::spawn(move || recorder_loop(rx));
        Self { tx }
    }

    pub fn start(&self) -> Result<(), String> {
        let (resp_tx, resp_rx) = channel();
        self.tx
            .send(Cmd::Start(resp_tx))
            .map_err(|_| "wątek recordera nie żyje".to_string())?;
        resp_rx
            .recv()
            .map_err(|_| "brak odpowiedzi z recordera".to_string())?
    }

    pub fn stop(&self) -> Result<Pcm, String> {
        let (resp_tx, resp_rx) = channel();
        self.tx
            .send(Cmd::Stop(resp_tx))
            .map_err(|_| "wątek recordera nie żyje".to_string())?;
        resp_rx
            .recv()
            .map_err(|_| "brak odpowiedzi z recordera".to_string())?
    }
}

// ─────────────────────────────── wątek recordera ───────────────────────────────

#[cfg(feature = "audio-live")]
fn recorder_loop(rx: std::sync::mpsc::Receiver<Cmd>) {
    use cpal::traits::StreamTrait;

    // Stan wewnątrz wątku — trzymamy tu stream (cpal, !Send), bufor i czas startu.
    let mut stream: Option<cpal::Stream> = None;
    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let mut sample_rate: u32 = 16_000;
    let mut started_at: Option<Instant> = None;

    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Start(resp) => {
                if stream.is_some() {
                    let _ = resp.send(Err("nagrywanie już trwa".into()));
                    continue;
                }
                match start_stream(buffer.clone()) {
                    Ok((s, rate)) => {
                        sample_rate = rate;
                        stream = Some(s);
                        started_at = Some(Instant::now());
                        buffer.lock().clear();
                        let _ = resp.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = resp.send(Err(e));
                    }
                }
            }
            Cmd::Stop(resp) => {
                let Some(s) = stream.take() else {
                    let _ = resp.send(Err("nie ma aktywnego nagrywania".into()));
                    continue;
                };
                let _ = s.pause();
                drop(s); // zwolnij urządzenie
                let samples = std::mem::take(&mut *buffer.lock());
                let duration_ms = started_at
                    .take()
                    .map(|i| i.elapsed().as_millis() as u64)
                    .unwrap_or(0);
                let _ = resp.send(Ok(Pcm {
                    samples,
                    sample_rate,
                    duration_ms,
                }));
            }
        }
    }
}

#[cfg(feature = "audio-live")]
fn start_stream(
    buffer: Arc<Mutex<Vec<f32>>>,
) -> Result<(cpal::Stream, u32), String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "brak domyślnego urządzenia wejściowego".to_string())?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("konfiguracja wejścia: {e}"))?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let err_fn = |e| eprintln!("[rpstr/audio] stream error: {e}");

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _| append_mono(&buffer, data, channels),
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _| {
                let converted: Vec<f32> =
                    data.iter().map(|s| *s as f32 / i16::MAX as f32).collect();
                append_mono(&buffer, &converted, channels);
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config.into(),
            move |data: &[u16], _| {
                let converted: Vec<f32> = data
                    .iter()
                    .map(|s| (*s as f32 - 32768.0) / 32768.0)
                    .collect();
                append_mono(&buffer, &converted, channels);
            },
            err_fn,
            None,
        ),
        fmt => return Err(format!("nieobsługiwany format próbki: {fmt:?}")),
    }
    .map_err(|e| format!("build_input_stream: {e}"))?;

    stream
        .play()
        .map_err(|e| format!("play: {e}"))?;

    Ok((stream, sample_rate))
}

#[cfg(feature = "audio-live")]
fn append_mono(buffer: &Arc<Mutex<Vec<f32>>>, data: &[f32], channels: usize) {
    let mut buf = buffer.lock();
    if channels <= 1 {
        buf.extend_from_slice(data);
    } else {
        buf.reserve(data.len() / channels);
        for frame in data.chunks_exact(channels) {
            let sum: f32 = frame.iter().sum();
            buf.push(sum / channels as f32);
        }
    }
}

// ─────────────────────────────── Stub bez feature ────────────────────────────

#[cfg(not(feature = "audio-live"))]
fn recorder_loop(rx: std::sync::mpsc::Receiver<Cmd>) {
    let _ = Arc::new(Mutex::new(Vec::<f32>::new()));
    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Start(resp) => {
                let _ = resp.send(Err(
                    "feature `audio-live` nie jest włączony — zbuduj z `--features audio-live`"
                        .into(),
                ));
            }
            Cmd::Stop(resp) => {
                let _ = resp.send(Err(
                    "feature `audio-live` nie jest włączony".into(),
                ));
            }
        }
    }
}

// ─────────────────────────────── Resample do 16 kHz ──────────────────────────

/// Prosta liniowa interpolacja do docelowego sample rate.  Whisper przyjmuje
/// 16 kHz mono f32 — dla wizyty medycznej liniowa interpolacja jest
/// wystarczająca (anti-alias wykonuje mikrofon + driver), a unika ciężkiego
/// crate'a `rubato`.
pub fn resample_linear(samples: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = from_hz as f64 / to_hz as f64;
    let out_len = ((samples.len() as f64) / ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 * ratio;
        let idx = src.floor() as usize;
        let frac = (src - idx as f64) as f32;
        let a = samples.get(idx).copied().unwrap_or(0.0);
        let b = samples.get(idx + 1).copied().unwrap_or(a);
        out.push(a * (1.0 - frac) + b * frac);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_identity() {
        let s = vec![0.1, 0.2, 0.3];
        assert_eq!(resample_linear(&s, 16_000, 16_000), s);
    }

    #[test]
    fn resample_downsample_halves_length() {
        let s: Vec<f32> = (0..100).map(|i| i as f32 * 0.01).collect();
        let out = resample_linear(&s, 32_000, 16_000);
        assert!(out.len() >= 49 && out.len() <= 51);
    }
}
