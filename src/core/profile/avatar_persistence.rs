// SPDX-License-Identifier: GPL-2.0-or-later

//! Atomic profile-avatar persistence and AccountsService coordination.

use std::fs::{self, File, OpenOptions, Permissions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use thiserror::Error;

use super::accounts::{AccountsError, clear_account_icon, set_account_icon};
use super::current_user_lookup::{CurrentUserError, current_user};
#[cfg(feature = "camera")]
use super::image::normalise_avatar_rgb;
use super::image::{ImageNormalisationError, normalise_avatar_file};
use super::models::{AvatarSaveOutcome, CurrentUser};
#[cfg(feature = "camera")]
use crate::core::camera::CameraFrame;

const FACE_FILE_MODE: u32 = 0o644;
const TEMPORARY_FILE_MODE: u32 = 0o600;
const MAX_TEMPORARY_FILE_ATTEMPTS: usize = 100;
static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Error)]
pub(crate) enum AvatarSaveError {
    #[error("the current user account could not be read: {0}")]
    User(#[from] CurrentUserError),
    #[error(transparent)]
    Image(#[from] ImageNormalisationError),
    #[error(transparent)]
    Local(#[from] LocalAvatarWriteError),
}

#[derive(Debug, Error)]
pub(crate) enum AvatarRemovalError {
    #[error("the current user account could not be read: {0}")]
    User(#[from] CurrentUserError),
    #[error("the local profile image could not be removed: {0}")]
    Local(#[source] std::io::Error),
}

#[derive(Debug, Error)]
pub(crate) enum LocalAvatarWriteError {
    #[error("a temporary profile image could not be created: {0}")]
    Create(#[source] std::io::Error),
    #[error("the temporary profile image could not be written: {0}")]
    Write(#[source] std::io::Error),
    #[error("the temporary profile image could not be synchronized: {0}")]
    Sync(#[source] std::io::Error),
    #[error("profile image permissions could not be set: {0}")]
    Permissions(#[source] std::io::Error),
    #[error("the profile image could not be atomically replaced: {0}")]
    Rename(#[source] std::io::Error),
}

pub(crate) fn save_current_avatar(source_path: PathBuf) -> Result<AvatarSaveOutcome, AvatarSaveError> {
    let png = normalise_avatar_file(&source_path)?;
    save_normalised_with(current_user()?, set_account_icon, png)
}

#[cfg(feature = "camera")]
pub(crate) fn save_current_camera_avatar(frame: CameraFrame) -> Result<AvatarSaveOutcome, AvatarSaveError> {
    let (width, height, rgb) = frame.into_parts();
    let png = normalise_avatar_rgb(width, height, rgb)?;
    save_normalised_with(current_user()?, set_account_icon, png)
}

pub(crate) fn remove_current_avatar() -> Result<Option<AccountsError>, AvatarRemovalError> {
    remove_with(current_user()?, clear_account_icon)
}

#[cfg(test)]
fn save_with(
    user: CurrentUser,
    set_icon: impl FnOnce(u32, &Path) -> Result<(), AccountsError>,
    source_path: &Path,
) -> Result<AvatarSaveOutcome, AvatarSaveError> {
    let png = normalise_avatar_file(source_path)?;
    save_normalised_with(user, set_icon, png)
}

#[cfg(all(feature = "camera", test))]
fn save_camera_with(
    user: CurrentUser,
    set_icon: impl FnOnce(u32, &Path) -> Result<(), AccountsError>,
    frame: CameraFrame,
) -> Result<AvatarSaveOutcome, AvatarSaveError> {
    let (width, height, rgb) = frame.into_parts();
    let png = normalise_avatar_rgb(width, height, rgb)?;
    save_normalised_with(user, set_icon, png)
}

fn save_normalised_with(
    user: CurrentUser,
    set_icon: impl FnOnce(u32, &Path) -> Result<(), AccountsError>,
    png: Vec<u8>,
) -> Result<AvatarSaveOutcome, AvatarSaveError> {
    let face_path = user.home_directory.join(".face");
    write_avatar_atomically(&face_path, &png)?;
    let accounts_error = set_icon(user.uid, &face_path).err();

    Ok(AvatarSaveOutcome::new(png, face_path, accounts_error))
}

fn remove_with(
    user: CurrentUser,
    clear_icon: impl FnOnce(u32) -> Result<(), AccountsError>,
) -> Result<Option<AccountsError>, AvatarRemovalError> {
    let face_path = user.home_directory.join(".face");
    match fs::remove_file(face_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(AvatarRemovalError::Local(error)),
    }
    Ok(clear_icon(user.uid).err())
}

fn write_avatar_atomically(path: &Path, png: &[u8]) -> Result<(), LocalAvatarWriteError> {
    let parent = path.parent().ok_or_else(|| {
        LocalAvatarWriteError::Create(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "profile image path has no parent directory",
        ))
    })?;
    let (mut file, temporary_path) = create_temporary_file(parent)?;
    let result = (|| {
        file.write_all(png).map_err(LocalAvatarWriteError::Write)?;
        file.sync_all().map_err(LocalAvatarWriteError::Sync)?;
        file.set_permissions(Permissions::from_mode(FACE_FILE_MODE))
            .map_err(LocalAvatarWriteError::Permissions)?;
        file.sync_all().map_err(LocalAvatarWriteError::Sync)?;
        drop(file);
        fs::rename(&temporary_path, path).map_err(LocalAvatarWriteError::Rename)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn create_temporary_file(parent: &Path) -> Result<(File, PathBuf), LocalAvatarWriteError> {
    let mut last_error = None;
    for _ in 0..MAX_TEMPORARY_FILE_ATTEMPTS {
        let sequence = TEMPORARY_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".face.tmp-{}-{sequence}", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(TEMPORARY_FILE_MODE)
            .open(&path)
        {
            Ok(file) => return Ok((file, path)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                last_error = Some(error);
            }
            Err(error) => return Err(LocalAvatarWriteError::Create(error)),
        }
    }

    Err(LocalAvatarWriteError::Create(last_error.unwrap_or_else(|| {
        std::io::Error::other("no temporary profile-image name was available")
    })))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::core::profile::models::CurrentUser;
    use image::codecs::png::PngEncoder;
    use image::{ExtendedColorType, ImageEncoder, Rgba, RgbaImage};

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let sequence = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "xfce4-profile-editor-avatar-test-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[derive(Default)]
    struct FakeIconClient {
        calls: RefCell<Vec<(u32, PathBuf)>>,
        fail: bool,
    }

    impl FakeIconClient {
        fn set_icon_file(&self, uid: u32, path: &Path) -> Result<(), AccountsError> {
            self.calls.borrow_mut().push((uid, path.to_owned()));
            if self.fail {
                Err(AccountsError::TestUnavailable)
            } else {
                Ok(())
            }
        }

        fn clear_icon_file(&self, uid: u32) -> Result<(), AccountsError> {
            self.calls.borrow_mut().push((uid, PathBuf::new()));
            if self.fail {
                Err(AccountsError::TestUnavailable)
            } else {
                Ok(())
            }
        }
    }

    fn user(home_directory: PathBuf) -> CurrentUser {
        CurrentUser {
            uid: 1000,
            home_directory,
        }
    }

    fn source_png(path: &Path) {
        let image = RgbaImage::from_pixel(4, 2, Rgba([20, 40, 60, 255]));
        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(image.as_raw(), image.width(), image.height(), ExtendedColorType::Rgba8)
            .unwrap();
        fs::write(path, png).unwrap();
    }

    #[test]
    fn atomically_replaces_face_and_updates_accounts_service_with_stable_path() {
        let directory = TestDirectory::new();
        let source = directory.path().join("source.png");
        let face = directory.path().join(".face");
        source_png(&source);
        fs::write(&face, b"old image").unwrap();
        let accounts = FakeIconClient::default();

        let outcome = save_with(
            user(directory.path().to_owned()),
            |uid, path| accounts.set_icon_file(uid, path),
            &source,
        )
        .unwrap();
        let (png, face_path, accounts_error) = outcome.into_parts();

        assert_eq!(face_path, face);
        assert_eq!(fs::read(&face).unwrap(), png);
        assert_eq!(
            fs::metadata(&face).unwrap().permissions().mode() & 0o777,
            FACE_FILE_MODE
        );
        assert_eq!(accounts.calls.into_inner(), vec![(1000, face.clone())]);
        assert!(accounts_error.is_none());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    }

    #[test]
    fn accounts_service_failure_is_reported_after_local_file_success() {
        let directory = TestDirectory::new();
        let source = directory.path().join("source.png");
        source_png(&source);
        let accounts = FakeIconClient {
            fail: true,
            ..Default::default()
        };

        let outcome = save_with(
            user(directory.path().to_owned()),
            |uid, path| accounts.set_icon_file(uid, path),
            &source,
        )
        .unwrap();
        let (png, face_path, accounts_error) = outcome.into_parts();

        assert_eq!(fs::read(face_path).unwrap(), png);
        assert!(accounts_error.is_some());
    }

    #[cfg(feature = "camera")]
    #[test]
    fn camera_frame_uses_the_same_atomic_persistence_path() {
        let directory = TestDirectory::new();
        let face = directory.path().join(".face");
        let accounts = FakeIconClient::default();
        let frame = CameraFrame::new(4, 2, vec![80; 4 * 2 * 3]).unwrap();

        let outcome = save_camera_with(
            user(directory.path().to_owned()),
            |uid, path| accounts.set_icon_file(uid, path),
            frame,
        )
        .unwrap();
        let (png, face_path, accounts_error) = outcome.into_parts();

        assert_eq!(face_path, face);
        assert_eq!(fs::read(&face).unwrap(), png);
        assert!(accounts_error.is_none());
        assert_eq!(accounts.calls.into_inner(), vec![(1000, face)]);
    }

    #[test]
    fn invalid_source_does_not_replace_face_or_call_accounts_service() {
        let directory = TestDirectory::new();
        let source = directory.path().join("source.txt");
        let face = directory.path().join(".face");
        fs::write(&source, b"not an image").unwrap();
        fs::write(&face, b"old image").unwrap();
        let accounts = FakeIconClient::default();

        let result = save_with(
            user(directory.path().to_owned()),
            |uid, path| accounts.set_icon_file(uid, path),
            &source,
        );

        assert!(matches!(result, Err(AvatarSaveError::Image(_))));
        assert_eq!(fs::read(face).unwrap(), b"old image");
        assert!(accounts.calls.borrow().is_empty());
    }

    #[test]
    fn removal_deletes_face_and_clears_accounts_service() {
        let directory = TestDirectory::new();
        let face = directory.path().join(".face");
        fs::write(&face, b"image").unwrap();
        let accounts = FakeIconClient::default();

        let outcome = remove_with(user(directory.path().to_owned()), |uid| accounts.clear_icon_file(uid)).unwrap();

        assert!(!face.exists());
        assert!(outcome.is_none());
        assert_eq!(accounts.calls.into_inner(), vec![(1000, PathBuf::new())]);
    }

    #[test]
    fn missing_face_is_already_removed_but_accounts_service_is_still_cleared() {
        let directory = TestDirectory::new();
        let accounts = FakeIconClient::default();

        let outcome = remove_with(user(directory.path().to_owned()), |uid| accounts.clear_icon_file(uid)).unwrap();

        assert!(outcome.is_none());
        assert_eq!(accounts.calls.borrow().len(), 1);
    }

    #[test]
    fn local_removal_failure_does_not_clear_accounts_service() {
        let directory = TestDirectory::new();
        fs::create_dir(directory.path().join(".face")).unwrap();
        let accounts = FakeIconClient::default();

        let result = remove_with(user(directory.path().to_owned()), |uid| accounts.clear_icon_file(uid));

        assert!(matches!(result, Err(AvatarRemovalError::Local(_))));
        assert!(accounts.calls.borrow().is_empty());
    }

    #[test]
    fn accounts_service_failure_is_reported_after_local_removal() {
        let directory = TestDirectory::new();
        let face = directory.path().join(".face");
        fs::write(&face, b"image").unwrap();
        let accounts = FakeIconClient {
            fail: true,
            ..Default::default()
        };

        let outcome = remove_with(user(directory.path().to_owned()), |uid| accounts.clear_icon_file(uid)).unwrap();

        assert!(!face.exists());
        assert!(outcome.is_some());
    }
}
