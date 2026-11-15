// SPDX-License-Identifier: GPL-2.0-or-later

//! Camera discovery, frames, and bounded background capture.

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
}

#[cfg(feature = "camera")]
impl CameraDevice {
    fn new(index: u32, name: String) -> Self {
        Self { index, name }
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
        devices.push(CameraDevice::new(index, name));
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
mod capture;

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
            CameraDevice::new(2, "Integrated camera".to_owned()),
            CameraDevice::new(7, "USB camera".to_owned()),
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
