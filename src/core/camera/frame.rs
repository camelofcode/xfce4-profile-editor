// SPDX-License-Identifier: GPL-2.0-or-later

//! Owned camera frame data and validation.

use super::capture::CameraCaptureError;

const BYTES_PER_PIXEL: usize = 3;
pub(super) const MAX_FRAME_DIMENSION: u32 = 4_096;
pub(super) const MAX_FRAME_PIXELS: u64 = 32_000_000;

/// An owned RGB888 frame that contains no GTK or backend-specific types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CameraFrame {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
}

impl CameraFrame {
    pub(crate) fn new(width: u32, height: u32, rgb: Vec<u8>) -> Result<Self, CameraCaptureError> {
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

pub(super) fn validate_frame_dimensions(width: u32, height: u32) -> Result<(), CameraCaptureError> {
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

fn validate_frame(width: u32, height: u32, byte_length: usize) -> Result<(), CameraCaptureError> {
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
    use super::*;

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
