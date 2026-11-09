// SPDX-License-Identifier: GPL-2.0-or-later

//! Camera capability and device discovery.

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

/// GTK-independent information needed to open a Linux camera later.
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
    use nokhwa::utils::{ApiBackend, CameraIndex};

    discover_with(|| {
        nokhwa::query(ApiBackend::Video4Linux)
            .map_err(|error| CameraDiscoveryError::new(error.to_string()))?
            .into_iter()
            .map(|camera| {
                let index = match camera.index() {
                    CameraIndex::Index(index) => *index,
                    CameraIndex::String(identifier) => {
                        return Err(CameraDiscoveryError::new(format!(
                            "backend returned unsupported device identifier {identifier}"
                        )));
                    }
                };
                Ok(CameraDevice::new(
                    index,
                    camera.human_name(),
                    camera.description().to_owned(),
                ))
            })
            .collect()
    })
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
