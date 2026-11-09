// SPDX-License-Identifier: GPL-2.0-or-later

//! Camera discovery and bounded background capture.

use thiserror::Error;

/// The camera devices currently usable by the application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CameraAvailability {
    /// Camera support was omitted at compile time.
    #[cfg(not(feature = "camera"))]
    NotCompiled,
    /// Camera support is present, but the system reported no devices.
    #[cfg(feature = "camera")]
    NoDevices,
    /// One or more devices are available for capture.
    #[cfg(feature = "camera")]
    Available(Vec<CameraDevice>),
}

/// GTK-independent information needed to open a Linux camera.
#[cfg(feature = "camera")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CameraDevice {
    index: u32,
    name: String,
    description: String,
}

#[cfg(feature = "camera")]
impl CameraDevice {
    fn new(index: u32, name: String, description: String) -> Self {
        Self {
            index,
            name,
            description,
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

/// A recoverable failure while querying the camera backend.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("camera device discovery failed: {details}")]
pub(crate) struct CameraDiscoveryError {
    details: String,
}

#[cfg(feature = "camera")]
impl CameraDiscoveryError {
    fn new(details: impl Into<String>) -> Self {
        Self {
            details: details.into(),
        }
    }
}

#[cfg(feature = "camera")]
pub(crate) fn discover_cameras() -> Result<CameraAvailability, CameraDiscoveryError> {
    discover_with(query_v4l_capture_devices)
}

#[cfg(not(feature = "camera"))]
pub(crate) fn discover_cameras() -> Result<CameraAvailability, CameraDiscoveryError> {
    Ok(CameraAvailability::NotCompiled)
}

#[cfg(feature = "camera")]
fn discover_with(
    query: impl FnOnce() -> Result<Vec<CameraDevice>, CameraDiscoveryError>,
) -> Result<CameraAvailability, CameraDiscoveryError> {
    match query()? {
        devices if devices.is_empty() => Ok(CameraAvailability::NoDevices),
        devices => Ok(CameraAvailability::Available(devices)),
    }
}

#[cfg(feature = "camera")]
fn query_v4l_capture_devices() -> Result<Vec<CameraDevice>, CameraDiscoveryError> {
    let mut devices = Vec::new();
    let mut first_error = None;
    for node in v4l::context::enum_devices() {
        let index = match u32::try_from(node.index()) {
            Ok(index) => index,
            Err(error) => {
                first_error.get_or_insert_with(|| error.to_string());
                continue;
            }
        };
        let device = match v4l::Device::new(node.index()) {
            Ok(device) => device,
            Err(error) => {
                first_error.get_or_insert_with(|| error.to_string());
                continue;
            }
        };
        let capabilities = match device.query_caps() {
            Ok(capabilities) => capabilities,
            Err(error) => {
                first_error.get_or_insert_with(|| error.to_string());
                continue;
            }
        };
        if !supports_nokhwa_capture(capabilities.capabilities) {
            continue;
        }

        let name = node.name().unwrap_or_else(|| capabilities.card.clone());
        devices.push(CameraDevice::new(
            index,
            name,
            format!("Video4Linux capture device at {}", node.path().display()),
        ));
    }
    devices.sort_by_key(|device| device.index);

    if devices.is_empty()
        && let Some(details) = first_error
    {
        return Err(CameraDiscoveryError::new(details));
    }
    Ok(devices)
}

#[cfg(feature = "camera")]
fn supports_nokhwa_capture(capabilities: v4l::capability::Flags) -> bool {
    use v4l::capability::Flags;

    capabilities.contains(Flags::VIDEO_CAPTURE | Flags::STREAMING)
}

#[cfg(feature = "camera")]
mod capture {
    use std::io::Cursor;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, MutexGuard};
    use std::thread::{self, JoinHandle};

    use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
    use nokhwa::pixel_format::RgbFormat;
    use nokhwa::utils::{
        ApiBackend, CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType,
    };
    use nokhwa::{Buffer, Camera};
    use thiserror::Error;

    use super::CameraDevice;

    const BYTES_PER_PIXEL: usize = 3;
    const MAX_FRAME_DIMENSION: u32 = 4_096;
    const MAX_FRAME_PIXELS: u64 = 32_000_000;
    const MAX_FRAME_ALLOCATION: u64 = 128 * 1024 * 1024;
    const TARGET_WIDTH: u32 = 640;
    const TARGET_HEIGHT: u32 = 480;
    const TARGET_FRAME_RATE: u32 = 30;
    const SUPPORTED_FRAME_FORMATS: [FrameFormat; 6] = [
        FrameFormat::MJPEG,
        FrameFormat::YUYV,
        FrameFormat::NV12,
        FrameFormat::RAWRGB,
        FrameFormat::RAWBGR,
        FrameFormat::GRAY,
    ];

    /// An owned RGB888 frame that contains no GTK or backend-specific types.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct CameraFrame {
        width: u32,
        height: u32,
        rgb: Vec<u8>,
    }

    impl CameraFrame {
        pub(crate) fn new(
            width: u32,
            height: u32,
            rgb: Vec<u8>,
        ) -> Result<Self, CameraCaptureError> {
            validate_frame(width, height, rgb.len())?;
            Ok(Self { width, height, rgb })
        }

        pub(crate) fn width(&self) -> u32 {
            self.width
        }

        pub(crate) fn height(&self) -> u32 {
            self.height
        }

        pub(crate) fn rgb(&self) -> &[u8] {
            &self.rgb
        }

        pub(crate) fn into_parts(self) -> (u32, u32, Vec<u8>) {
            (self.width, self.height, self.rgb)
        }
    }

    /// The observable state of a capture worker.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) enum CameraCaptureStatus {
        Starting,
        Running,
        Stopped,
        Failed(CameraCaptureError),
    }

    /// A recoverable camera open, stream, frame, or worker failure.
    #[derive(Clone, Debug, Error, Eq, PartialEq)]
    pub(crate) enum CameraCaptureError {
        #[error("camera could not be opened: {details}")]
        Open { details: String },
        #[error("camera format could not be configured: {details}")]
        Configure { details: String },
        #[error("camera stream could not be started: {details}")]
        Stream { details: String },
        #[error("camera frame could not be read: {details}")]
        Frame { details: String },
        #[error("camera frame could not be decoded: {details}")]
        Decode { details: String },
        #[error(
            "camera frame dimensions or data length are invalid: {width}x{height}, {bytes} bytes"
        )]
        InvalidFrame {
            width: u32,
            height: u32,
            bytes: usize,
        },
        #[error("camera capture worker stopped unexpectedly")]
        WorkerPanicked,
        #[error("camera capture worker could not be started: {details}")]
        WorkerStart { details: String },
    }

    /// A worker-backed capture whose frame storage is always bounded to one frame.
    pub(crate) struct CameraCapture {
        latest: Arc<Mutex<Option<CameraFrame>>>,
        status: Arc<Mutex<CameraCaptureStatus>>,
        stop_requested: Arc<AtomicBool>,
        worker: Option<JoinHandle<()>>,
    }

    impl CameraCapture {
        pub(crate) fn take_latest_frame(&self) -> Option<CameraFrame> {
            lock(&self.latest).take()
        }

        pub(crate) fn status(&self) -> CameraCaptureStatus {
            lock(&self.status).clone()
        }

        pub(crate) fn request_stop(&self) {
            self.stop_requested.store(true, Ordering::Release);
        }

        pub(crate) fn shutdown(mut self) -> Result<(), CameraCaptureError> {
            self.request_stop();
            if self
                .worker
                .take()
                .is_some_and(|worker| worker.join().is_err())
            {
                return Err(CameraCaptureError::WorkerPanicked);
            }
            Ok(())
        }
    }

    impl Drop for CameraCapture {
        fn drop(&mut self) {
            self.request_stop();
        }
    }

    trait FrameSource {
        fn next_frame(&mut self) -> Result<CameraFrame, CameraCaptureError>;
    }

    type SourceFactory =
        Box<dyn FnOnce() -> Result<Box<dyn FrameSource>, CameraCaptureError> + Send + 'static>;

    pub(crate) fn start_camera_capture(device: CameraDevice) -> CameraCapture {
        start_capture_with(Box::new(move || {
            NokhwaFrameSource::open(device).map(|source| Box::new(source) as Box<dyn FrameSource>)
        }))
    }

    fn start_capture_with(factory: SourceFactory) -> CameraCapture {
        let latest = Arc::new(Mutex::new(None));
        let status = Arc::new(Mutex::new(CameraCaptureStatus::Starting));
        let stop_requested = Arc::new(AtomicBool::new(false));

        let worker_latest = Arc::clone(&latest);
        let worker_status = Arc::clone(&status);
        let worker_stop = Arc::clone(&stop_requested);
        let worker = thread::Builder::new()
            .name("profile-camera-capture".to_owned())
            .spawn(move || {
                if worker_stop.load(Ordering::Acquire) {
                    set_status(&worker_status, CameraCaptureStatus::Stopped);
                    return;
                }
                let mut source = match factory() {
                    Ok(source) => source,
                    Err(error) => {
                        set_status(&worker_status, CameraCaptureStatus::Failed(error));
                        return;
                    }
                };
                set_status(&worker_status, CameraCaptureStatus::Running);

                while !worker_stop.load(Ordering::Acquire) {
                    match source.next_frame() {
                        Ok(frame) => *lock(&worker_latest) = Some(frame),
                        Err(error) => {
                            set_status(&worker_status, CameraCaptureStatus::Failed(error));
                            return;
                        }
                    }
                }
                set_status(&worker_status, CameraCaptureStatus::Stopped);
            });

        let worker = match worker {
            Ok(worker) => Some(worker),
            Err(error) => {
                set_status(
                    &status,
                    CameraCaptureStatus::Failed(CameraCaptureError::WorkerStart {
                        details: error.to_string(),
                    }),
                );
                None
            }
        };

        CameraCapture {
            latest,
            status,
            stop_requested,
            worker,
        }
    }

    fn set_status(status: &Mutex<CameraCaptureStatus>, value: CameraCaptureStatus) {
        *lock(status) = value;
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().unwrap_or_else(|error| error.into_inner())
    }

    struct NokhwaFrameSource {
        camera: Camera,
    }

    impl NokhwaFrameSource {
        fn open(device: CameraDevice) -> Result<Self, CameraCaptureError> {
            let request =
                RequestedFormat::with_formats(RequestedFormatType::None, &SUPPORTED_FRAME_FORMATS);
            let mut camera = Camera::with_backend(
                CameraIndex::Index(device.index),
                request,
                ApiBackend::Video4Linux,
            )
            .map_err(|error| CameraCaptureError::Open {
                details: error.to_string(),
            })?;

            let formats = camera.compatible_camera_formats().map_err(|error| {
                CameraCaptureError::Configure {
                    details: error.to_string(),
                }
            })?;
            let format =
                preferred_format(&formats).ok_or_else(|| CameraCaptureError::Configure {
                    details: "the device exposes no supported frame format".to_owned(),
                })?;
            camera
                .set_camera_requset(RequestedFormat::with_formats(
                    RequestedFormatType::Exact(format),
                    &[format.format()],
                ))
                .map_err(|error| CameraCaptureError::Configure {
                    details: error.to_string(),
                })?;
            camera
                .open_stream()
                .map_err(|error| CameraCaptureError::Stream {
                    details: error.to_string(),
                })?;
            Ok(Self { camera })
        }
    }

    impl FrameSource for NokhwaFrameSource {
        fn next_frame(&mut self) -> Result<CameraFrame, CameraCaptureError> {
            let buffer = self
                .camera
                .frame()
                .map_err(|error| CameraCaptureError::Frame {
                    details: error.to_string(),
                })?;
            decode_frame(&buffer)
        }
    }

    fn preferred_format(formats: &[CameraFormat]) -> Option<CameraFormat> {
        formats
            .iter()
            .filter(|format| SUPPORTED_FRAME_FORMATS.contains(&format.format()))
            .filter(|format| {
                let resolution = format.resolution();
                resolution.width() <= MAX_FRAME_DIMENSION
                    && resolution.height() <= MAX_FRAME_DIMENSION
                    && u64::from(resolution.width()) * u64::from(resolution.height())
                        <= MAX_FRAME_PIXELS
            })
            .min_by_key(|format| {
                let resolution = format.resolution();
                (
                    resolution.width().abs_diff(TARGET_WIDTH)
                        + resolution.height().abs_diff(TARGET_HEIGHT),
                    frame_format_priority(format.format()),
                    format.frame_rate().abs_diff(TARGET_FRAME_RATE),
                )
            })
            .copied()
    }

    fn frame_format_priority(format: FrameFormat) -> u8 {
        match format {
            FrameFormat::MJPEG => 0,
            FrameFormat::YUYV => 1,
            FrameFormat::NV12 => 2,
            FrameFormat::RAWRGB => 3,
            FrameFormat::RAWBGR => 4,
            FrameFormat::GRAY => 5,
        }
    }

    fn decode_frame(buffer: &Buffer) -> Result<CameraFrame, CameraCaptureError> {
        if buffer.source_frame_format() == FrameFormat::MJPEG {
            return decode_mjpeg(buffer.buffer());
        }

        let resolution = buffer.resolution();
        let rgb =
            buffer
                .decode_image::<RgbFormat>()
                .map_err(|error| CameraCaptureError::Decode {
                    details: error.to_string(),
                })?;
        CameraFrame::new(resolution.width(), resolution.height(), rgb.into_raw())
    }

    fn decode_mjpeg(bytes: &[u8]) -> Result<CameraFrame, CameraCaptureError> {
        let reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Jpeg);
        let mut decoder = reader
            .into_decoder()
            .map_err(|error| CameraCaptureError::Decode {
                details: error.to_string(),
            })?;
        let (width, height) = decoder.dimensions();
        validate_frame_dimensions(width, height)?;
        let mut limits = Limits::default();
        limits.max_image_width = Some(MAX_FRAME_DIMENSION);
        limits.max_image_height = Some(MAX_FRAME_DIMENSION);
        limits.max_alloc = Some(MAX_FRAME_ALLOCATION);
        decoder
            .set_limits(limits)
            .map_err(|error| CameraCaptureError::Decode {
                details: error.to_string(),
            })?;
        let rgb = DynamicImage::from_decoder(decoder)
            .map_err(|error| CameraCaptureError::Decode {
                details: error.to_string(),
            })?
            .into_rgb8();
        CameraFrame::new(rgb.width(), rgb.height(), rgb.into_raw())
    }

    fn validate_frame_dimensions(width: u32, height: u32) -> Result<(), CameraCaptureError> {
        let pixels = u64::from(width) * u64::from(height);
        if width == 0
            || height == 0
            || width > MAX_FRAME_DIMENSION
            || height > MAX_FRAME_DIMENSION
            || pixels > MAX_FRAME_PIXELS
        {
            return Err(CameraCaptureError::InvalidFrame {
                width,
                height,
                bytes: 0,
            });
        }
        Ok(())
    }

    fn validate_frame(
        width: u32,
        height: u32,
        byte_length: usize,
    ) -> Result<(), CameraCaptureError> {
        validate_frame_dimensions(width, height)?;
        let expected = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(BYTES_PER_PIXEL as u64))
            .and_then(|bytes| usize::try_from(bytes).ok());
        if expected != Some(byte_length) {
            return Err(CameraCaptureError::InvalidFrame {
                width,
                height,
                bytes: byte_length,
            });
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use std::collections::VecDeque;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::{Duration, Instant};

        use image::codecs::jpeg::JpegEncoder;
        use image::{ColorType, ImageEncoder, Rgb, RgbImage};

        use super::*;

        struct FakeSource {
            results: VecDeque<Result<CameraFrame, CameraCaptureError>>,
            repeat: Option<CameraFrame>,
            releases: Arc<AtomicUsize>,
        }

        impl FrameSource for FakeSource {
            fn next_frame(&mut self) -> Result<CameraFrame, CameraCaptureError> {
                if let Some(result) = self.results.pop_front() {
                    return result;
                }
                if let Some(frame) = &self.repeat {
                    thread::sleep(Duration::from_millis(1));
                    return Ok(frame.clone());
                }
                Err(CameraCaptureError::Frame {
                    details: "end of fake stream".to_owned(),
                })
            }
        }

        impl Drop for FakeSource {
            fn drop(&mut self) {
                self.releases.fetch_add(1, Ordering::SeqCst);
            }
        }

        fn frame(value: u8) -> CameraFrame {
            CameraFrame::new(2, 1, vec![value; 6]).unwrap()
        }

        fn wait_for(mut predicate: impl FnMut() -> bool) {
            let deadline = Instant::now() + Duration::from_secs(2);
            while !predicate() {
                assert!(Instant::now() < deadline, "timed out waiting for worker");
                thread::sleep(Duration::from_millis(2));
            }
        }

        #[test]
        fn latest_frame_slot_replaces_older_frames_without_queueing() {
            let releases = Arc::new(AtomicUsize::new(0));
            let source = FakeSource {
                results: VecDeque::from([Ok(frame(1)), Ok(frame(2)), Ok(frame(3))]),
                repeat: None,
                releases: Arc::clone(&releases),
            };
            let capture = start_capture_with(Box::new(|| Ok(Box::new(source))));

            wait_for(|| matches!(capture.status(), CameraCaptureStatus::Failed(_)));

            assert_eq!(capture.take_latest_frame(), Some(frame(3)));
            capture.shutdown().unwrap();
            assert_eq!(releases.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn shutdown_joins_the_worker_and_releases_the_source() {
            let releases = Arc::new(AtomicUsize::new(0));
            let source = FakeSource {
                results: VecDeque::new(),
                repeat: Some(frame(4)),
                releases: Arc::clone(&releases),
            };
            let capture = start_capture_with(Box::new(|| Ok(Box::new(source))));
            wait_for(|| capture.status() == CameraCaptureStatus::Running);

            capture.shutdown().unwrap();

            assert_eq!(releases.load(Ordering::SeqCst), 1);
        }

        #[test]
        fn worker_reports_frame_failures() {
            let releases = Arc::new(AtomicUsize::new(0));
            let error = CameraCaptureError::Frame {
                details: "camera unplugged".to_owned(),
            };
            let source = FakeSource {
                results: VecDeque::from([Err(error.clone())]),
                repeat: None,
                releases,
            };
            let capture = start_capture_with(Box::new(|| Ok(Box::new(source))));

            wait_for(|| capture.status() == CameraCaptureStatus::Failed(error.clone()));

            capture.shutdown().unwrap();
        }

        #[test]
        fn selects_the_closest_safe_format_and_prefers_mjpeg_on_a_tie() {
            let formats = [
                CameraFormat::new_from(1_920, 1_080, FrameFormat::MJPEG, 30),
                CameraFormat::new_from(640, 480, FrameFormat::YUYV, 30),
                CameraFormat::new_from(640, 480, FrameFormat::MJPEG, 15),
                CameraFormat::new_from(640, 480, FrameFormat::MJPEG, 30),
            ];

            assert_eq!(preferred_format(&formats), Some(formats[3]));
        }

        #[test]
        fn decodes_mjpeg_with_the_existing_image_stack() {
            let image = RgbImage::from_pixel(2, 1, Rgb([20, 40, 60]));
            let mut jpeg = Vec::new();
            JpegEncoder::new_with_quality(&mut jpeg, 95)
                .write_image(image.as_raw(), 2, 1, ColorType::Rgb8.into())
                .unwrap();

            let decoded = decode_mjpeg(&jpeg).unwrap();

            assert_eq!((decoded.width(), decoded.height()), (2, 1));
            assert_eq!(decoded.rgb().len(), 6);
        }

        #[test]
        fn rejects_malformed_frame_lengths() {
            assert!(matches!(
                CameraFrame::new(2, 2, vec![0; 11]),
                Err(CameraCaptureError::InvalidFrame {
                    width: 2,
                    height: 2,
                    bytes: 11
                })
            ));
        }
    }
}

#[cfg(feature = "camera")]
pub(crate) use capture::{
    CameraCapture, CameraCaptureError, CameraCaptureStatus, CameraFrame, start_camera_capture,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "camera")]
    #[test]
    fn reports_no_devices_without_assuming_a_default_index() {
        assert_eq!(
            discover_with(|| Ok(Vec::new())),
            Ok(CameraAvailability::NoDevices)
        );
    }

    #[cfg(feature = "camera")]
    #[test]
    fn accepts_only_streaming_video_capture_nodes() {
        use v4l::capability::Flags;

        assert!(supports_nokhwa_capture(
            Flags::VIDEO_CAPTURE | Flags::STREAMING
        ));
        assert!(!supports_nokhwa_capture(
            Flags::META_CAPTURE | Flags::STREAMING
        ));
        assert!(!supports_nokhwa_capture(Flags::VIDEO_CAPTURE));
    }

    #[cfg(feature = "camera")]
    #[test]
    fn preserves_every_discovered_device_and_its_real_index() {
        let devices = vec![
            CameraDevice::new(
                2,
                "Integrated camera".to_owned(),
                "Video4Linux device at /dev/video2".to_owned(),
            ),
            CameraDevice::new(
                7,
                "USB camera".to_owned(),
                "Video4Linux device at /dev/video7".to_owned(),
            ),
        ];

        assert_eq!(
            discover_with(|| Ok(devices.clone())),
            Ok(CameraAvailability::Available(devices))
        );
    }

    #[cfg(feature = "camera")]
    #[test]
    fn preserves_backend_failures() {
        let error = CameraDiscoveryError::new("query failed");

        assert_eq!(discover_with(|| Err(error.clone())), Err(error));
    }

    #[cfg(not(feature = "camera"))]
    #[test]
    fn reports_when_camera_support_is_not_compiled() {
        assert_eq!(discover_cameras(), Ok(CameraAvailability::NotCompiled));
    }
}
