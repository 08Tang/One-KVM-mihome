//! Native Amlogic AMLENC bindings for the S912/GXM vendor Linux 4.9 stack.
//!
//! The vendor libraries are deliberately loaded at runtime.  They must be built
//! with the One-KVM ABI v1 patch from the standalone `amlenc` repository;
//! unpatched 0.4 libraries
//! are rejected before any device access is attempted.

use std::env;
use std::ffi::{c_int, c_long, c_uchar, c_uint, OsStr};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use bytes::Bytes;
use libloading::Library;
use tracing::{debug, warn};

use crate::error::{AppError, Result};
use crate::video::format::Resolution;

pub const AMLENC_ABI_VERSION: c_int = 1;
pub const AMLENC_H264_CODEC_NAME: &str = "h264_amlenc";
pub const AMLENC_H265_CODEC_NAME: &str = "hevc_amlenc";
pub const AMLENC_H264_DEFAULT_LIBRARY: &str = "libvpcodec.so";
pub const AMLENC_H265_DEFAULT_LIBRARY: &str = "libvphevcodec.so";

const AMLENC_MAX_WIDTH: u32 = 1920;
const AMLENC_MAX_HEIGHT: u32 = 1080;
const AMLENC_MAX_FPS: u32 = 60;
const MIN_OUTPUT_BUFFER_SIZE: usize = 1024 * 1024;
const OUTPUT_STALL_TIMEOUT: Duration = Duration::from_secs(1);
const CODEC_ID_H264: c_int = 4;
const CODEC_ID_H265: c_int = 5;
const IMG_FMT_NV12: c_int = 1;
const FRAME_TYPE_AUTO: c_int = 1;
const FRAME_TYPE_IDR: c_int = 2;
const H264_NV12_FORMAT: c_int = 0;
const H265_NV12_FORMAT: c_int = 1;

type AbiVersionFn = unsafe extern "C" fn() -> c_int;
type H264InitFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int, c_int) -> c_long;
type H265InitFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int, c_int, c_int) -> c_long;
type H264EncodeFn =
    unsafe extern "C" fn(c_long, c_int, *mut c_uchar, c_int, *mut c_uchar, c_int) -> c_int;
type H265EncodeFn =
    unsafe extern "C" fn(c_long, c_int, *mut c_uchar, c_uint, *mut c_uchar, c_int) -> c_int;
type DestroyFn = unsafe extern "C" fn(c_long) -> c_int;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmlencCodec {
    H264,
    H265,
}

impl AmlencCodec {
    pub fn codec_name(self) -> &'static str {
        match self {
            Self::H264 => AMLENC_H264_CODEC_NAME,
            Self::H265 => AMLENC_H265_CODEC_NAME,
        }
    }

    pub fn default_library(self) -> &'static str {
        match self {
            Self::H264 => AMLENC_H264_DEFAULT_LIBRARY,
            Self::H265 => AMLENC_H265_DEFAULT_LIBRARY,
        }
    }

    pub fn library_env(self) -> &'static str {
        match self {
            Self::H264 => "ONE_KVM_AMLENC_H264_LIB",
            Self::H265 => "ONE_KVM_AMLENC_H265_LIB",
        }
    }

    pub fn device_node(self) -> &'static str {
        match self {
            Self::H264 => "/dev/amvenc_avc",
            Self::H265 => "/dev/HevcEnc",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AmlencConfig {
    pub codec: AmlencCodec,
    pub resolution: Resolution,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub gop: u32,
}

impl AmlencConfig {
    pub fn validate(self) -> Result<()> {
        let width = self.resolution.width;
        let height = self.resolution.height;
        if width == 0
            || height == 0
            || width > AMLENC_MAX_WIDTH
            || height > AMLENC_MAX_HEIGHT
            || width % 16 != 0
            || height % 2 != 0
        {
            return Err(AppError::VideoError(format!(
                "AMLENC requires NV12 with 16-aligned width, even height, and at most 1920x1080 (got {}x{})",
                width, height
            )));
        }
        if !(1..=AMLENC_MAX_FPS).contains(&self.fps) {
            return Err(AppError::VideoError(format!(
                "AMLENC supports 1-60 fps (got {})",
                self.fps
            )));
        }
        if self.bitrate_kbps == 0 || self.bitrate_kbps > (c_int::MAX as u32 / 1000) {
            return Err(AppError::VideoError(format!(
                "Invalid AMLENC bitrate: {} kbps",
                self.bitrate_kbps
            )));
        }
        if self.gop > c_int::MAX as u32 {
            return Err(AppError::VideoError("AMLENC GOP is too large".to_string()));
        }
        nv12_frame_size(self.resolution)?;
        Ok(())
    }

    fn bitrate_bps(self) -> c_int {
        (self.bitrate_kbps * 1000) as c_int
    }

    fn vendor_gop(self) -> c_int {
        match self.codec {
            // GXM's H.264 microcode can time out on a later natural IDR for
            // complex 1080p pictures.  The pinned vendor library defines zero
            // as an infinite GOP (one IDR when the instance is created).
            AmlencCodec::H264 => 0,
            AmlencCodec::H265 => self.gop as c_int,
        }
    }
}

pub fn nv12_frame_size(resolution: Resolution) -> Result<usize> {
    let pixels = (resolution.width as usize)
        .checked_mul(resolution.height as usize)
        .ok_or_else(|| AppError::VideoError("AMLENC NV12 frame size overflow".to_string()))?;
    pixels
        .checked_mul(3)
        .map(|value| value / 2)
        .ok_or_else(|| AppError::VideoError("AMLENC NV12 frame size overflow".to_string()))
}

fn validate_abi_version(version: c_int, path: &Path) -> Result<()> {
    if version != AMLENC_ABI_VERSION {
        return Err(AppError::VideoError(format!(
            "AMLENC library {} has ABI {}, expected ABI v{}; apply the one-kvm-amlenc-abi-v1.patch from the standalone amlenc repository",
            path.display(),
            version,
            AMLENC_ABI_VERSION
        )));
    }
    Ok(())
}

struct H264Api {
    _library: Library,
    init: H264InitFn,
    encode: H264EncodeFn,
    destroy: DestroyFn,
}

struct H265Api {
    _library: Library,
    init: H265InitFn,
    encode: H265EncodeFn,
    destroy: DestroyFn,
}

enum AmlencApi {
    H264(H264Api),
    H265(H265Api),
}

unsafe fn required_symbol<T: Copy>(library: &Library, name: &[u8], path: &Path) -> Result<T> {
    // SAFETY: the caller supplies the signature from the fixed upstream headers.
    unsafe { library.get::<T>(name) }
        .map(|symbol| *symbol)
        .map_err(|error| {
            AppError::VideoError(format!(
                "AMLENC library {} is missing {}: {}",
                path.display(),
                String::from_utf8_lossy(name).trim_end_matches('\0'),
                error
            ))
        })
}

impl AmlencApi {
    fn load(codec: AmlencCodec, path: &Path) -> Result<Self> {
        // SAFETY: all calls are made through signatures checked against the pinned headers,
        // and the Library remains owned by the API object for the lifetime of the pointers.
        let library = unsafe { Library::new(path) }.map_err(|error| {
            AppError::VideoError(format!(
                "Failed to load AMLENC {} library {}: {}",
                codec.codec_name(),
                path.display(),
                error
            ))
        })?;
        let abi_version: AbiVersionFn =
            unsafe { required_symbol(&library, b"one_kvm_amlenc_abi_version\0", path)? };
        // SAFETY: the ABI marker has no arguments or side effects.
        validate_abi_version(unsafe { abi_version() }, path)?;

        Ok(match codec {
            AmlencCodec::H264 => {
                let init: H264InitFn =
                    unsafe { required_symbol(&library, b"vl_video_encoder_init\0", path)? };
                let encode: H264EncodeFn =
                    unsafe { required_symbol(&library, b"vl_video_encoder_encode\0", path)? };
                let destroy: DestroyFn =
                    unsafe { required_symbol(&library, b"vl_video_encoder_destory\0", path)? };
                Self::H264(H264Api {
                    _library: library,
                    init,
                    encode,
                    destroy,
                })
            }
            AmlencCodec::H265 => {
                let init: H265InitFn =
                    unsafe { required_symbol(&library, b"vl_video_encoder_init\0", path)? };
                let encode: H265EncodeFn =
                    unsafe { required_symbol(&library, b"vl_video_encoder_encode\0", path)? };
                let destroy: DestroyFn =
                    unsafe { required_symbol(&library, b"vl_video_encoder_destory\0", path)? };
                Self::H265(H265Api {
                    _library: library,
                    init,
                    encode,
                    destroy,
                })
            }
        })
    }

    unsafe fn init(&self, config: AmlencConfig) -> c_long {
        let width = config.resolution.width as c_int;
        let height = config.resolution.height as c_int;
        match self {
            Self::H264(api) => unsafe {
                (api.init)(
                    CODEC_ID_H264,
                    width,
                    height,
                    config.fps as c_int,
                    config.bitrate_bps(),
                    config.vendor_gop(),
                    IMG_FMT_NV12,
                )
            },
            Self::H265(api) => unsafe {
                (api.init)(
                    CODEC_ID_H265,
                    width,
                    height,
                    config.fps as c_int,
                    config.bitrate_bps(),
                    config.gop as c_int,
                )
            },
        }
    }

    unsafe fn encode(
        &self,
        handle: c_long,
        frame_type: c_int,
        input: *mut c_uchar,
        output: *mut c_uchar,
        output_len: usize,
    ) -> c_int {
        match self {
            // H.264's fourth argument is documented as input length, but the pinned
            // implementation uses it exclusively as output capacity.
            Self::H264(api) => unsafe {
                (api.encode)(
                    handle,
                    frame_type,
                    input,
                    output_len as c_int,
                    output,
                    H264_NV12_FORMAT,
                )
            },
            Self::H265(api) => unsafe {
                (api.encode)(
                    handle,
                    frame_type,
                    input,
                    output_len as c_uint,
                    output,
                    H265_NV12_FORMAT,
                )
            },
        }
    }

    unsafe fn destroy(&self, handle: c_long) {
        match self {
            Self::H264(api) => {
                unsafe { (api.destroy)(handle) };
            }
            Self::H265(api) => {
                unsafe { (api.destroy)(handle) };
            }
        }
    }
}

static AMLENC_INSTANCE_ACTIVE: AtomicBool = AtomicBool::new(false);

struct ExclusiveInstance;

impl ExclusiveInstance {
    fn acquire() -> Result<Self> {
        AMLENC_INSTANCE_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                AppError::VideoError(
                    "AMLENC hardware is already in use by another encoder or self-check"
                        .to_string(),
                )
            })?;
        Ok(Self)
    }
}

impl Drop for ExclusiveInstance {
    fn drop(&mut self) {
        AMLENC_INSTANCE_ACTIVE.store(false, Ordering::Release);
    }
}

pub struct AmlencEncoder {
    api: AmlencApi,
    handle: c_long,
    config: AmlencConfig,
    output: Vec<u8>,
    force_keyframe: bool,
    rebuild_before_next_frame: bool,
    expect_parameterized_keyframe: bool,
    last_output: Instant,
    _exclusive: ExclusiveInstance,
}

impl AmlencEncoder {
    pub fn new(config: AmlencConfig) -> Result<Self> {
        let path = env::var_os(config.codec.library_env())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(config.codec.default_library()));
        Self::with_library(config, path)
    }

    pub fn with_library(config: AmlencConfig, path: impl AsRef<OsStr>) -> Result<Self> {
        config.validate()?;
        let exclusive = ExclusiveInstance::acquire()?;
        let path = PathBuf::from(path.as_ref());
        let api = AmlencApi::load(config.codec, &path)?;
        let frame_size = nv12_frame_size(config.resolution)?;
        let output = vec![0; frame_size.max(MIN_OUTPUT_BUFFER_SIZE)];
        let mut encoder = Self {
            api,
            handle: 0,
            config,
            output,
            force_keyframe: false,
            rebuild_before_next_frame: false,
            expect_parameterized_keyframe: true,
            last_output: Instant::now(),
            _exclusive: exclusive,
        };
        encoder.create_handle()?;
        Ok(encoder)
    }

    pub fn codec_name(&self) -> &'static str {
        self.config.codec.codec_name()
    }

    pub fn config(&self) -> AmlencConfig {
        self.config
    }

    fn create_handle(&mut self) -> Result<()> {
        debug!(
            "Creating {} at {}x{} {} fps {} kbps",
            self.codec_name(),
            self.config.resolution.width,
            self.config.resolution.height,
            self.config.fps,
            self.config.bitrate_kbps
        );
        // SAFETY: config validation guarantees values accepted by ABI v1.
        self.handle = unsafe { self.api.init(self.config) };
        if self.handle <= 0 {
            return Err(AppError::VideoError(format!(
                "AMLENC {} initialization failed; check {}, firmware, CMA, and device permissions",
                self.codec_name(),
                self.config.codec.device_node()
            )));
        }
        // The first H.264 picture is naturally an IDR.  Never pass the
        // in-place FORCE_IDR command to the GXM H.264 microcode: later IDRs can
        // wedge it. H.265 does not share that observed defect and retains its
        // ABI-v1 forced-IRAP behavior.
        self.force_keyframe = self.config.codec == AmlencCodec::H265;
        self.rebuild_before_next_frame = false;
        self.expect_parameterized_keyframe = true;
        self.last_output = Instant::now();
        Ok(())
    }

    fn destroy_handle(&mut self) {
        if self.handle > 0 {
            // SAFETY: the handle was returned by this API instance and is destroyed once.
            unsafe { self.api.destroy(self.handle) };
            self.handle = 0;
        }
    }

    fn rebuild(&mut self, reason: &str) -> Result<()> {
        warn!("Rebuilding {} encoder: {}", self.codec_name(), reason);
        self.destroy_handle();
        self.create_handle()
    }

    pub fn request_keyframe(&mut self) {
        if self.config.codec == AmlencCodec::H264 {
            // A fresh encoder reliably emits SPS/PPS + IDR on its first AUTO
            // frame. Coalesce repeated client requests while a rebuild or
            // fresh first frame is already pending.
            if !self.expect_parameterized_keyframe {
                self.rebuild_before_next_frame = true;
            }
        } else {
            self.force_keyframe = true;
        }
    }

    pub fn set_bitrate(&mut self, bitrate_kbps: u32) -> Result<()> {
        let mut updated = self.config;
        updated.bitrate_kbps = bitrate_kbps;
        updated.validate()?;
        self.config = updated;
        self.rebuild("bitrate changed")
    }

    pub fn encode_raw(&mut self, data: &[u8]) -> Result<Option<(Bytes, bool)>> {
        let expected = nv12_frame_size(self.config.resolution)?;
        if data.len() != expected {
            return Err(AppError::VideoError(format!(
                "AMLENC requires contiguous NV12 data of exactly {} bytes (got {})",
                expected,
                data.len()
            )));
        }

        if self.rebuild_before_next_frame {
            self.rebuild("H.264 keyframe requested")?;
        }

        match self.encode_once(data) {
            Ok(frame) => Ok(frame),
            Err(first_error) => {
                self.rebuild(&format!("vendor encode call failed: {first_error}"))?;
                self.encode_once(data).map_err(|retry_error| {
                    AppError::VideoError(format!(
                        "AMLENC encode failed after one rebuild: {}; retry: {}",
                        first_error, retry_error
                    ))
                })
            }
        }
    }

    fn encode_once(&mut self, data: &[u8]) -> Result<Option<(Bytes, bool)>> {
        if self.handle <= 0 {
            return Err(AppError::VideoError(
                "AMLENC handle is not initialized".to_string(),
            ));
        }
        let forced = self.force_keyframe;
        let require_parameterized_keyframe = self.expect_parameterized_keyframe || forced;
        let frame_type = if forced {
            FRAME_TYPE_IDR
        } else {
            FRAME_TYPE_AUTO
        };
        // The vendor API takes a mutable pointer but does not modify VMALLOC input.
        // SAFETY: input/output live for the call, capacities are ABI-sized and the
        // output length is validated before any slice is formed.
        let length = unsafe {
            self.api.encode(
                self.handle,
                frame_type,
                data.as_ptr() as *mut c_uchar,
                self.output.as_mut_ptr(),
                self.output.len(),
            )
        };
        if length < 0 {
            return Err(AppError::VideoError(format!(
                "{} vendor library returned {}",
                self.codec_name(),
                length
            )));
        }
        // A keyframe request applies to one submitted frame. Repeating IDR on
        // every zero-output call can trap the S912 driver in its light-reset
        // loop; WebRTC will issue another request if this attempt was skipped.
        if forced {
            self.force_keyframe = false;
        }
        let length = length as usize;
        if length > self.output.len() {
            return Err(AppError::VideoError(format!(
                "{} returned oversized output {} > {}",
                self.codec_name(),
                length,
                self.output.len()
            )));
        }
        if length == 0 {
            if forced {
                return Err(AppError::VideoError(format!(
                    "{} produced no output for a forced keyframe",
                    self.codec_name()
                )));
            }
            // The vendor ABI uses zero for rate-control skips and recoverable
            // hardware timeouts.  Do not rebuild for a few skipped frames, but
            // recover if the vendor stops producing output altogether.
            if self.last_output.elapsed() >= OUTPUT_STALL_TIMEOUT {
                self.rebuild("no encoded output for one second")?;
            }
            return Ok(None);
        }

        let encoded = &self.output[..length];
        let nal_summary = inspect_annex_b(self.config.codec, encoded);
        let keyframe = nal_summary.keyframe;
        if require_parameterized_keyframe
            && (!keyframe || !nal_summary.has_parameter_sets(self.config.codec))
        {
            return Err(AppError::VideoError(format!(
                "{} fresh/forced keyframe did not contain an IRAP/IDR and complete parameter sets",
                self.codec_name()
            )));
        }
        self.force_keyframe = false;
        self.expect_parameterized_keyframe = false;
        self.last_output = Instant::now();
        Ok(Some((Bytes::copy_from_slice(encoded), keyframe)))
    }
}

impl Drop for AmlencEncoder {
    fn drop(&mut self) {
        self.destroy_handle();
    }
}

#[derive(Default)]
struct AnnexBNalSummary {
    keyframe: bool,
    vps: bool,
    sps: bool,
    pps: bool,
}

impl AnnexBNalSummary {
    fn has_parameter_sets(&self, codec: AmlencCodec) -> bool {
        match codec {
            AmlencCodec::H264 => self.sps && self.pps,
            AmlencCodec::H265 => self.vps && self.sps && self.pps,
        }
    }
}

fn inspect_annex_b(codec: AmlencCodec, data: &[u8]) -> AnnexBNalSummary {
    let mut summary = AnnexBNalSummary::default();
    let mut index = 0;
    while index + 3 <= data.len() {
        let start_len = if index + 4 <= data.len() && data[index..index + 4] == [0, 0, 0, 1] {
            4
        } else if data[index..index + 3] == [0, 0, 1] {
            3
        } else {
            index += 1;
            continue;
        };
        let nal = index + start_len;
        if nal >= data.len() {
            break;
        }
        let nal_type = match codec {
            AmlencCodec::H264 => data[nal] & 0x1f,
            AmlencCodec::H265 => (data[nal] >> 1) & 0x3f,
        };
        match codec {
            AmlencCodec::H264 => match nal_type {
                5 => summary.keyframe = true,
                7 => summary.sps = true,
                8 => summary.pps = true,
                _ => {}
            },
            AmlencCodec::H265 => match nal_type {
                16..=23 => summary.keyframe = true,
                32 => summary.vps = true,
                33 => summary.sps = true,
                34 => summary.pps = true,
                _ => {}
            },
        }
        index = nal + 1;
    }
    summary
}

pub fn is_keyframe(codec: AmlencCodec, data: &[u8]) -> bool {
    inspect_annex_b(codec, data).keyframe
}

pub fn has_parameter_sets(codec: AmlencCodec, data: &[u8]) -> bool {
    inspect_annex_b(codec, data).has_parameter_sets(codec)
}

#[cfg_attr(
    not(any(test, all(target_os = "linux", target_arch = "aarch64"))),
    allow(dead_code)
)]
fn is_s912_gxm_compatible(compatible: &[u8]) -> bool {
    let compatible = String::from_utf8_lossy(compatible).to_ascii_lowercase();
    compatible.contains("amlogic,gxm")
        || compatible.contains("amlogic, gxm")
        || compatible.contains("amlogic,meson-gxm")
        || compatible.contains("amlogic,s912")
}

pub fn system_is_s912_gxm() -> Result<bool> {
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        let compatible = std::fs::read("/proc/device-tree/compatible").map_err(|error| {
            AppError::VideoError(format!(
                "Cannot read /proc/device-tree/compatible for AMLENC detection: {}",
                error
            ))
        })?;
        return Ok(is_s912_gxm_compatible(&compatible));
    }
    #[cfg(not(all(target_os = "linux", target_arch = "aarch64")))]
    Ok(false)
}

/// Perform the destructive part of backend detection: initialize and encode one
/// 640x480 NV12 frame.  The caller must first check SoC compatibility and node.
pub fn smoke_test(codec: AmlencCodec) -> Result<()> {
    let resolution = Resolution::new(640, 480);
    let config = AmlencConfig {
        codec,
        resolution,
        fps: 30,
        bitrate_kbps: 1_000,
        gop: 30,
    };
    let mut encoder = AmlencEncoder::new(config)?;
    let mut frame = vec![0x80; nv12_frame_size(resolution)?];
    frame[..(resolution.width * resolution.height) as usize].fill(0x10);
    for _ in 0..3 {
        if encoder.encode_raw(&frame)?.is_some() {
            return Ok(());
        }
    }
    Err(AppError::VideoError(format!(
        "{} produced no output during the 640x480 probe",
        codec.codec_name()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::process::Command;
    #[cfg(unix)]
    use std::sync::Mutex;

    #[cfg(unix)]
    static TEST_INSTANCE_MUTEX: Mutex<()> = Mutex::new(());

    #[cfg(unix)]
    const H264_FIXTURE: &str = r#"
        static int values[16];
        static int mode;
        static int fail_pending;
        int one_kvm_amlenc_abi_version(void) { return 1; }
        long vl_video_encoder_init(int codec, int width, int height, int fps,
                                   int bitrate, int gop, int image_format) {
            values[0]++; values[1] = codec; values[2] = width; values[3] = height;
            values[4] = fps; values[5] = bitrate; values[6] = gop;
            values[7] = image_format; return 1;
        }
        int vl_video_encoder_encode(long handle, int frame_type, unsigned char *in,
                                    int in_size, unsigned char *out, int format) {
            (void)handle; (void)in; values[8]++; values[9] = frame_type;
            values[10] = in_size; values[11] = format;
            if (fail_pending) { fail_pending = 0; return -9; }
            if (mode == 2) return 0;
            if (mode == 3) return 2000000;
            { unsigned char data[] = {0,0,1,0x67,0,0,1,0x68,0,0,1,0x65};
              for (unsigned long i = 0; i < sizeof(data); i++) out[i] = data[i];
              return sizeof(data); }
        }
        int vl_video_encoder_destory(long handle) { (void)handle; values[12]++; return 1; }
        int test_get(int index) { return values[index]; }
        void test_set_mode(int value) { mode = value; }
        void test_fail_once(void) { fail_pending = 1; }
    "#;

    #[cfg(unix)]
    const H265_FIXTURE: &str = r#"
        static int values[16];
        static int mode;
        int one_kvm_amlenc_abi_version(void) { return 1; }
        long vl_video_encoder_init(int codec, int width, int height, int fps,
                                   int bitrate, int gop) {
            values[0]++; values[1] = codec; values[2] = width; values[3] = height;
            values[4] = fps; values[5] = bitrate; values[6] = gop; return 1;
        }
        int vl_video_encoder_encode(long handle, int frame_type, unsigned char *in,
                                    unsigned int output_len, unsigned char *out, int format) {
            (void)handle; (void)in; values[8]++; values[9] = frame_type;
            values[10] = output_len; values[11] = format;
            if (mode == 3) return output_len + 1;
            { unsigned char data[] = {0,0,1,0x40,1,0,0,1,0x42,1,0,0,1,0x44,1,
                                      0,0,1,0x26,1};
              for (unsigned long i = 0; i < sizeof(data); i++) out[i] = data[i];
              return sizeof(data); }
        }
        int vl_video_encoder_destory(long handle) { (void)handle; values[12]++; return 1; }
        int test_get(int index) { return values[index]; }
        void test_set_mode(int value) { mode = value; }
    "#;

    #[cfg(unix)]
    fn build_fixture(directory: &Path, name: &str, source: &str) -> PathBuf {
        let source_path = directory.join(format!("{name}.c"));
        let library_path = directory.join(format!("lib{name}.so"));
        std::fs::write(&source_path, source).unwrap();
        let status = Command::new("cc")
            .args(["-shared", "-fPIC"])
            .arg(&source_path)
            .arg("-o")
            .arg(&library_path)
            .status()
            .unwrap();
        assert!(status.success());
        library_path
    }

    #[test]
    fn validates_geometry_fps_and_nv12_size() {
        let valid = AmlencConfig {
            codec: AmlencCodec::H264,
            resolution: Resolution::new(1920, 1080),
            fps: 60,
            bitrate_kbps: 8_000,
            gop: 60,
        };
        assert!(valid.validate().is_ok());
        assert_eq!(nv12_frame_size(valid.resolution).unwrap(), 3_110_400);

        for invalid in [
            AmlencConfig {
                resolution: Resolution::new(1919, 1080),
                ..valid
            },
            AmlencConfig {
                resolution: Resolution::new(1920, 1079),
                ..valid
            },
            AmlencConfig {
                resolution: Resolution::new(2560, 1440),
                ..valid
            },
            AmlencConfig { fps: 61, ..valid },
        ] {
            assert!(invalid.validate().is_err());
        }
    }

    #[test]
    fn recognizes_vendor_and_mainline_gxm_compatibles() {
        assert!(is_s912_gxm_compatible(b"amlogic, Gxm\0khadas,kvim2"));
        assert!(is_s912_gxm_compatible(
            b"amlogic,q200\0amlogic,s912\0amlogic,meson-gxm"
        ));
        assert!(!is_s912_gxm_compatible(b"rockchip,rk3588"));
    }

    #[test]
    fn validates_abi_marker() {
        let path = Path::new("libvpcodec.so");
        assert!(validate_abi_version(AMLENC_ABI_VERSION, path).is_ok());
        assert!(validate_abi_version(0, path).is_err());
    }

    #[test]
    fn parses_h264_idr_and_parameter_sets() {
        let data = [0, 0, 0, 1, 0x67, 1, 0, 0, 1, 0x68, 2, 0, 0, 0, 1, 0x65, 3];
        assert!(is_keyframe(AmlencCodec::H264, &data));
        assert!(has_parameter_sets(AmlencCodec::H264, &data));
        assert!(!is_keyframe(AmlencCodec::H264, &[0, 0, 1, 0x41]));
    }

    #[test]
    fn parses_h265_irap_and_parameter_sets() {
        let data = [
            0,
            0,
            1,
            32 << 1,
            1,
            0,
            0,
            1,
            33 << 1,
            1,
            0,
            0,
            1,
            34 << 1,
            1,
            0,
            0,
            1,
            19 << 1,
            1,
        ];
        assert!(is_keyframe(AmlencCodec::H265, &data));
        assert!(has_parameter_sets(AmlencCodec::H265, &data));
        assert!(!is_keyframe(AmlencCodec::H265, &[0, 0, 1, 1 << 1, 1]));
    }

    #[test]
    #[cfg(unix)]
    fn loads_symbols_maps_both_abis_and_recovers() {
        let _test_instance = TEST_INSTANCE_MUTEX.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let h264_path = build_fixture(directory.path(), "amlenc_h264", H264_FIXTURE);
        let h265_path = build_fixture(directory.path(), "amlenc_h265", H265_FIXTURE);

        type GetFn = unsafe extern "C" fn(c_int) -> c_int;
        type SetModeFn = unsafe extern "C" fn(c_int);
        type FailOnceFn = unsafe extern "C" fn();

        // Keep this second dlopen alive so the fixture's counters remain available.
        let h264_control = unsafe { Library::new(&h264_path) }.unwrap();
        let h264_get: GetFn = unsafe { *h264_control.get(b"test_get\0").unwrap() };
        let h264_set_mode: SetModeFn = unsafe { *h264_control.get(b"test_set_mode\0").unwrap() };
        let h264_fail_once: FailOnceFn = unsafe { *h264_control.get(b"test_fail_once\0").unwrap() };

        let resolution = Resolution::new(640, 480);
        let frame = vec![0x80; nv12_frame_size(resolution).unwrap()];
        {
            let mut encoder = AmlencEncoder::with_library(
                AmlencConfig {
                    codec: AmlencCodec::H264,
                    resolution,
                    fps: 60,
                    bitrate_kbps: 2_000,
                    gop: 60,
                },
                &h264_path,
            )
            .unwrap();
            assert!(encoder.encode_raw(&frame).unwrap().unwrap().1);
            // SAFETY: indices and fixture signatures are fixed above.
            unsafe {
                assert_eq!(h264_get(1), CODEC_ID_H264);
                assert_eq!(h264_get(4), 60);
                assert_eq!(h264_get(5), 2_000_000);
                assert_eq!(h264_get(6), 0);
                assert_eq!(h264_get(7), IMG_FMT_NV12);
                assert_eq!(h264_get(9), FRAME_TYPE_AUTO);
                assert_eq!(h264_get(10), MIN_OUTPUT_BUFFER_SIZE as c_int);
                assert_eq!(h264_get(11), H264_NV12_FORMAT);

                h264_fail_once();
            }
            assert!(encoder.encode_raw(&frame).unwrap().is_some());
            unsafe { assert_eq!(h264_get(0), 2) };

            unsafe { h264_set_mode(2) };
            encoder.request_keyframe();
            assert!(encoder.encode_raw(&frame).unwrap().is_none());
            unsafe { assert_eq!(h264_get(9), FRAME_TYPE_AUTO) };
            unsafe { assert_eq!(h264_get(0), 3) };
            assert!(encoder.encode_raw(&frame).unwrap().is_none());
            unsafe { assert_eq!(h264_get(9), FRAME_TYPE_AUTO) };
            assert!(encoder.encode_raw(&frame).unwrap().is_none());
            unsafe { assert_eq!(h264_get(0), 3) };

            encoder.last_output = Instant::now() - OUTPUT_STALL_TIMEOUT;
            assert!(encoder.encode_raw(&frame).unwrap().is_none());
            unsafe { assert_eq!(h264_get(0), 4) };

            unsafe { h264_set_mode(0) };
            encoder.set_bitrate(3_000).unwrap();
            assert!(encoder.encode_raw(&frame).unwrap().unwrap().1);
            unsafe {
                assert_eq!(h264_get(5), 3_000_000);
                assert_eq!(h264_get(9), FRAME_TYPE_AUTO);
                assert_eq!(h264_get(0), 5);
            }
        }

        let h265_control = unsafe { Library::new(&h265_path) }.unwrap();
        let h265_get: GetFn = unsafe { *h265_control.get(b"test_get\0").unwrap() };
        let h265_set_mode: SetModeFn = unsafe { *h265_control.get(b"test_set_mode\0").unwrap() };
        {
            let mut encoder = AmlencEncoder::with_library(
                AmlencConfig {
                    codec: AmlencCodec::H265,
                    resolution,
                    fps: 30,
                    bitrate_kbps: 1_500,
                    gop: 30,
                },
                &h265_path,
            )
            .unwrap();
            assert!(encoder.encode_raw(&frame).unwrap().unwrap().1);
            unsafe {
                assert_eq!(h265_get(1), CODEC_ID_H265);
                assert_eq!(h265_get(4), 30);
                assert_eq!(h265_get(5), 1_500_000);
                assert_eq!(h265_get(9), FRAME_TYPE_IDR);
                assert_eq!(h265_get(10), MIN_OUTPUT_BUFFER_SIZE as c_int);
                assert_eq!(h265_get(11), H265_NV12_FORMAT);
                h265_set_mode(3);
            }
            let error = encoder.encode_raw(&frame).unwrap_err().to_string();
            assert!(error.contains("oversized output"));
        }
    }

    #[test]
    #[cfg(unix)]
    fn rejects_unpatched_library_without_abi_symbol() {
        let _test_instance = TEST_INSTANCE_MUTEX.lock().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = build_fixture(
            directory.path(),
            "unpatched_amlenc",
            "long vl_video_encoder_init(void) { return 1; }",
        );
        let error = AmlencEncoder::with_library(
            AmlencConfig {
                codec: AmlencCodec::H264,
                resolution: Resolution::new(640, 480),
                fps: 30,
                bitrate_kbps: 1_000,
                gop: 30,
            },
            path,
        )
        .err()
        .expect("unpatched library must be rejected")
        .to_string();
        assert!(error.contains("one_kvm_amlenc_abi_version"));
    }
}
