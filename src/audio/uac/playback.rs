use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use alsa::pcm::{Access, Format, Frames, HwParams, State};
use alsa::{Direction, ValueOr, PCM};
use tracing::{info, warn};

use crate::error::{AppError, Result};

const RETRY_BACKOFF: Duration = Duration::from_secs(1);
const PERIOD_FRAMES: Frames = 960;
const BUFFER_FRAMES: Frames = 4_800;
const START_THRESHOLD_PERIODS: Frames = 4;
const SINK_STALL_TIMEOUT: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UacPlaybackState {
    Idle,
    Waiting,
    Active,
    Stalled,
}

impl UacPlaybackState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Waiting => "waiting",
            Self::Active => "active",
            Self::Stalled => "stalled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UacPlaybackConfig {
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for UacPlaybackConfig {
    fn default() -> Self {
        Self {
            device_name: crate::otg::uac::find_uac_pcm_device()
                .unwrap_or_else(crate::otg::uac::uac_pcm_device),
            sample_rate: 48_000,
            channels: 2,
        }
    }
}

struct PlaybackInner {
    config: UacPlaybackConfig,
    stopped: AtomicBool,
    active_session: Mutex<Option<Arc<Mutex<SessionRuntime>>>>,
}

enum SessionSink {
    Closed { retry_at: Option<Instant> },
    Probing { pcm: PCM, stalled: bool },
    Active { pcm: PCM, last_progress: Instant },
}

impl SessionSink {
    fn state(&self) -> UacPlaybackState {
        match self {
            Self::Closed { retry_at: None } => UacPlaybackState::Waiting,
            Self::Closed { retry_at: Some(_) } => UacPlaybackState::Stalled,
            Self::Probing { stalled: false, .. } => UacPlaybackState::Waiting,
            Self::Probing { stalled: true, .. } => UacPlaybackState::Stalled,
            Self::Active { .. } => UacPlaybackState::Active,
        }
    }
}

struct SessionRuntime {
    sink: SessionSink,
}

impl SessionRuntime {
    fn new() -> Self {
        Self {
            sink: SessionSink::Closed { retry_at: None },
        }
    }

    fn state(&self) -> UacPlaybackState {
        self.sink.state()
    }

    fn close(&mut self) {
        self.sink = SessionSink::Closed { retry_at: None };
    }

    /// Advance playback only when a WebSocket frame arrives. All ALSA handles
    /// are non-blocking, so a slow or absent USB host drops the current frame
    /// instead of occupying a worker thread or accumulating stale speech.
    fn write(&mut self, config: &UacPlaybackConfig, samples: &[i16]) -> bool {
        let sink = std::mem::replace(&mut self.sink, SessionSink::Closed { retry_at: None });
        let (next_sink, accepted) = drive_sink(sink, config, samples);
        self.sink = next_sink;
        accepted
    }
}

#[derive(Clone)]
pub struct UacPlayback {
    inner: Arc<PlaybackInner>,
}

pub struct UacSession {
    playback: UacPlayback,
    runtime: Arc<Mutex<SessionRuntime>>,
}

impl UacPlayback {
    pub fn start(config: UacPlaybackConfig) -> Result<Self> {
        if config.sample_rate != 48_000 || config.channels != 2 {
            return Err(AppError::BadRequest(
                "UAC playback supports only 48000 Hz stereo".to_string(),
            ));
        }

        Ok(Self {
            inner: Arc::new(PlaybackInner {
                config,
                stopped: AtomicBool::new(false),
                active_session: Mutex::new(None),
            }),
        })
    }

    pub fn acquire_session(&self) -> Result<UacSession> {
        let mut active = self.inner.active_session.lock().unwrap();
        if self.inner.stopped.load(Ordering::Acquire) {
            return Err(AppError::ServiceUnavailable(
                "UAC playback is stopping".to_string(),
            ));
        }
        if active.is_some() {
            return Err(AppError::ServiceUnavailable(
                "another UAC microphone session is already active".to_string(),
            ));
        }

        let runtime = Arc::new(Mutex::new(SessionRuntime::new()));
        *active = Some(Arc::clone(&runtime));
        Ok(UacSession {
            playback: self.clone(),
            runtime,
        })
    }

    /// Stop accepting frames and synchronously close an active ALSA handle.
    /// This guarantees configfs may rebuild the UAC function after this call.
    pub fn stop(&self) {
        if self.inner.stopped.swap(true, Ordering::AcqRel) {
            return;
        }
        let runtime = self.inner.active_session.lock().unwrap().take();
        if let Some(runtime) = runtime {
            runtime.lock().unwrap().close();
        }
    }
}

impl UacSession {
    pub fn state(&self) -> UacPlaybackState {
        self.runtime.lock().unwrap().state()
    }

    /// Returns whether the frame was accepted and the resulting target state.
    pub fn try_write(&self, pcm: &[i16]) -> Result<(bool, UacPlaybackState)> {
        let channels = self.playback.inner.config.channels as usize;
        if pcm.is_empty() || !pcm.len().is_multiple_of(channels) {
            return Err(AppError::BadRequest(
                "UAC PCM must contain complete stereo frames".to_string(),
            ));
        }
        if self.playback.inner.stopped.load(Ordering::Acquire) {
            return Err(AppError::ServiceUnavailable(
                "UAC playback has stopped".to_string(),
            ));
        }

        let mut runtime = self.runtime.lock().unwrap();
        if self.playback.inner.stopped.load(Ordering::Acquire) {
            runtime.close();
            return Err(AppError::ServiceUnavailable(
                "UAC playback has stopped".to_string(),
            ));
        }
        let accepted = runtime.write(&self.playback.inner.config, pcm);
        Ok((accepted, runtime.state()))
    }
}

impl Drop for UacSession {
    fn drop(&mut self) {
        let mut active = self.playback.inner.active_session.lock().unwrap();
        if active
            .as_ref()
            .is_some_and(|session| Arc::ptr_eq(session, &self.runtime))
        {
            *active = None;
        }
        drop(active);
        self.runtime.lock().unwrap().close();
    }
}

fn drive_sink(
    sink: SessionSink,
    config: &UacPlaybackConfig,
    samples: &[i16],
) -> (SessionSink, bool) {
    match sink {
        SessionSink::Closed { retry_at } => {
            if retry_at.is_some_and(|deadline| Instant::now() < deadline) {
                return (SessionSink::Closed { retry_at }, false);
            }

            match open_pcm(config).and_then(|pcm| {
                prime_pcm_with_silence(&pcm, config.channels as usize)?;
                Ok(pcm)
            }) {
                Ok(pcm) => drive_probe(pcm, false, config, samples),
                Err(error) => {
                    warn!("Failed to open UAC playback device; retrying later: {error}");
                    (
                        SessionSink::Closed {
                            retry_at: Some(Instant::now() + RETRY_BACKOFF),
                        },
                        false,
                    )
                }
            }
        }
        SessionSink::Probing { pcm, stalled } => drive_probe(pcm, stalled, config, samples),
        SessionSink::Active { pcm, last_progress } => {
            drive_active(pcm, last_progress, config, samples)
        }
    }
}

fn drive_probe(
    pcm: PCM,
    stalled: bool,
    config: &UacPlaybackConfig,
    samples: &[i16],
) -> (SessionSink, bool) {
    match sink_is_consuming(&pcm) {
        Ok(false) => (SessionSink::Probing { pcm, stalled }, false),
        Ok(true) => {
            if let Err(error) = reset_pcm_buffer(&pcm) {
                warn!("Failed to activate UAC playback; retrying later: {error}");
                return retry_later();
            }
            info!("UAC target started consuming microphone audio");
            drive_active(pcm, Instant::now(), config, samples)
        }
        Err(error) => {
            warn!("Failed to probe UAC playback; retrying later: {error}");
            retry_later()
        }
    }
}

fn drive_active(
    pcm: PCM,
    last_progress: Instant,
    config: &UacPlaybackConfig,
    samples: &[i16],
) -> (SessionSink, bool) {
    match write_pcm_nonblocking(&pcm, samples, config.channels as usize) {
        Ok(WriteOutcome::Progress) => (
            SessionSink::Active {
                pcm,
                last_progress: Instant::now(),
            },
            true,
        ),
        Ok(WriteOutcome::Recovered) => (
            SessionSink::Active {
                pcm,
                last_progress: Instant::now(),
            },
            false,
        ),
        Ok(WriteOutcome::Blocked) if last_progress.elapsed() < SINK_STALL_TIMEOUT => {
            (SessionSink::Active { pcm, last_progress }, false)
        }
        Ok(WriteOutcome::Blocked) => {
            if let Err(error) = reset_pcm_buffer(&pcm)
                .and_then(|_| prime_pcm_with_silence(&pcm, config.channels as usize))
            {
                warn!("Failed to reset stalled UAC playback: {error}");
                return retry_later();
            }
            info!("UAC target stopped consuming audio; waiting for playback activity");
            (SessionSink::Probing { pcm, stalled: true }, false)
        }
        Err(error) => {
            warn!("UAC playback write failed; retrying later: {error}");
            retry_later()
        }
    }
}

fn retry_later() -> (SessionSink, bool) {
    (
        SessionSink::Closed {
            retry_at: Some(Instant::now() + RETRY_BACKOFF),
        },
        false,
    )
}

fn open_pcm(config: &UacPlaybackConfig) -> Result<PCM> {
    let pcm = PCM::new(&config.device_name, Direction::Playback, true).map_err(|error| {
        AppError::AudioError(format!(
            "Failed to open UAC device {}: {error}",
            config.device_name
        ))
    })?;
    {
        let params = HwParams::any(&pcm)
            .map_err(|error| AppError::AudioError(format!("UAC HwParams failed: {error}")))?;
        params
            .set_channels(config.channels as u32)
            .and_then(|_| params.set_rate(config.sample_rate, ValueOr::Nearest))
            .and_then(|_| params.set_format(Format::s16()))
            .and_then(|_| params.set_access(Access::RWInterleaved))
            .and_then(|_| params.set_period_size_near(PERIOD_FRAMES, ValueOr::Nearest))
            .and_then(|_| params.set_buffer_size_near(BUFFER_FRAMES))
            .and_then(|_| pcm.hw_params(&params))
            .map_err(|error| {
                AppError::AudioError(format!("Failed to configure UAC playback: {error}"))
            })?;
    }

    let (buffer_frames, period_frames) = pcm.get_params().map_err(|error| {
        AppError::AudioError(format!("Failed to read UAC PCM parameters: {error}"))
    })?;
    {
        let params = pcm.sw_params_current().map_err(|error| {
            AppError::AudioError(format!("Failed to read UAC SwParams: {error}"))
        })?;
        let start_threshold =
            (period_frames as Frames * START_THRESHOLD_PERIODS).min(buffer_frames as Frames);
        params
            .set_start_threshold(start_threshold)
            .and_then(|_| params.set_avail_min(period_frames as Frames))
            .and_then(|_| pcm.sw_params(&params))
            .map_err(|error| {
                AppError::AudioError(format!("Failed to configure UAC SwParams: {error}"))
            })?;
    }
    pcm.prepare().map_err(|error| {
        AppError::AudioError(format!("Failed to prepare UAC playback: {error}"))
    })?;
    info!(
        "UAC playback opened on {} (buffer={} frames, period={} frames)",
        config.device_name, buffer_frames, period_frames
    );
    Ok(pcm)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteOutcome {
    Progress,
    Blocked,
    Recovered,
}

fn write_pcm_nonblocking(pcm: &PCM, samples: &[i16], channels: usize) -> Result<WriteOutcome> {
    let total_frames = samples.len() / channels;
    match pcm.avail() {
        Ok(available) if available < total_frames as Frames => return Ok(WriteOutcome::Blocked),
        Ok(_) => {}
        Err(error) => {
            recover_pcm(pcm, error)?;
            return Ok(WriteOutcome::Recovered);
        }
    }

    let io = pcm
        .io_i16()
        .map_err(|error| AppError::AudioError(format!("UAC PCM I/O failed: {error}")))?;
    match io.writei(samples) {
        Ok(0) => Ok(WriteOutcome::Blocked),
        Ok(_) => Ok(WriteOutcome::Progress),
        Err(error) if error.errno() == libc::EAGAIN => Ok(WriteOutcome::Blocked),
        Err(error) => {
            recover_pcm(pcm, error)?;
            Ok(WriteOutcome::Recovered)
        }
    }
}

/// Once a full playback buffer gains at least one period of free space, the
/// USB host has enabled the UAC streaming interface and is consuming samples.
fn sink_is_consuming(pcm: &PCM) -> Result<bool> {
    if pcm.state() == State::XRun {
        return Ok(true);
    }

    match pcm.avail() {
        Ok(available) => Ok(available >= PERIOD_FRAMES),
        Err(error) if error.errno() == libc::EPIPE => Ok(true),
        Err(error) => Err(AppError::AudioError(format!(
            "Failed to query UAC playback availability: {error}"
        ))),
    }
}

fn recover_pcm(pcm: &PCM, error: alsa::Error) -> Result<()> {
    let errno = error.errno();
    pcm.try_recover(error, true).map_err(|recover_error| {
        AppError::AudioError(format!("Failed to recover UAC playback: {recover_error}"))
    })?;
    if matches!(errno, libc::EPIPE | libc::ESTRPIPE) {
        warn!("Recovered UAC playback after ALSA error {errno}");
    }
    Ok(())
}

fn reset_pcm_buffer(pcm: &PCM) -> Result<()> {
    pcm.drop()
        .and_then(|_| pcm.prepare())
        .map_err(|error| AppError::AudioError(format!("Failed to reset UAC PCM: {error}")))
}

/// Prime the non-blocking ALSA buffer with silence. Subsequent WebSocket
/// frames inspect buffer progress to detect when the USB host starts reading.
fn prime_pcm_with_silence(pcm: &PCM, channels: usize) -> Result<()> {
    let silence = vec![0i16; BUFFER_FRAMES as usize * channels];
    let io = pcm
        .io_i16()
        .map_err(|error| AppError::AudioError(format!("UAC PCM I/O failed: {error}")))?;
    let mut frame_offset = 0usize;
    while frame_offset < BUFFER_FRAMES as usize {
        match io.writei(&silence[frame_offset * channels..]) {
            Ok(0) => break,
            Ok(written) => frame_offset += written,
            Err(error) if error.errno() == libc::EAGAIN => break,
            Err(error) => {
                return Err(AppError::AudioError(format!(
                    "Failed to prime UAC PCM with silence: {error}"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permits_only_one_microphone_session() {
        let playback = UacPlayback::start(UacPlaybackConfig::default()).unwrap();
        let first = playback.acquire_session().unwrap();
        assert_eq!(first.state(), UacPlaybackState::Waiting);
        assert!(playback.acquire_session().is_err());

        drop(first);
        assert!(playback.acquire_session().is_ok());
        playback.stop();
    }

    #[test]
    fn stop_rejects_new_and_existing_session_writes() {
        let playback = UacPlayback::start(UacPlaybackConfig::default()).unwrap();
        let session = playback.acquire_session().unwrap();

        playback.stop();

        assert!(session.try_write(&[0, 0]).is_err());
        assert!(playback.acquire_session().is_err());
    }

    #[test]
    fn rejects_incomplete_stereo_frames_before_opening_alsa() {
        let playback = UacPlayback::start(UacPlaybackConfig::default()).unwrap();
        let session = playback.acquire_session().unwrap();

        assert!(session.try_write(&[0]).is_err());
        assert_eq!(session.state(), UacPlaybackState::Waiting);
    }

    #[test]
    fn closed_sink_state_reflects_retry_backoff() {
        assert_eq!(
            SessionSink::Closed { retry_at: None }.state(),
            UacPlaybackState::Waiting
        );
        assert_eq!(
            SessionSink::Closed {
                retry_at: Some(Instant::now())
            }
            .state(),
            UacPlaybackState::Stalled
        );
    }
}
