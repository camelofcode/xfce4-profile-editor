// SPDX-License-Identifier: GPL-2.0-or-later

//! AccountsService access for the current profile.

use std::path::{Path, PathBuf};

use thiserror::Error;
use zbus::proxy::MethodFlags;
use zbus::{Connection, DynamicTuple, OwnedObjectPath, Proxy, proxy};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AccountDetails {
    pub(crate) display_name: String,
    pub(crate) icon_file: Option<PathBuf>,
}

pub(crate) trait AccountsSource {
    fn details_for_uid(&self, uid: u32) -> Result<AccountDetails, AccountsError>;
}

pub(super) trait DisplayNameClient {
    fn set_display_name(&self, uid: u32, display_name: &str) -> Result<(), AccountsError>;
}

pub(super) trait IconClient {
    fn set_icon_file(&self, uid: u32, path: &Path) -> Result<(), AccountsError>;
    fn clear_icon_file(&self, uid: u32) -> Result<(), AccountsError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SystemAccountsSource;

impl AccountsSource for SystemAccountsSource {
    fn details_for_uid(&self, uid: u32) -> Result<AccountDetails, AccountsError> {
        zbus::block_on(load_details(uid)).map_err(AccountsError::dbus)
    }
}

impl DisplayNameClient for SystemAccountsSource {
    fn set_display_name(&self, uid: u32, display_name: &str) -> Result<(), AccountsError> {
        zbus::block_on(set_real_name(uid, display_name)).map_err(AccountsError::dbus)
    }
}

impl IconClient for SystemAccountsSource {
    fn set_icon_file(&self, uid: u32, path: &Path) -> Result<(), AccountsError> {
        let path = path.to_str().ok_or_else(AccountsError::invalid_icon_path)?;
        zbus::block_on(set_icon_file(uid, path)).map_err(AccountsError::dbus)
    }

    fn clear_icon_file(&self, uid: u32) -> Result<(), AccountsError> {
        zbus::block_on(set_icon_file(uid, "")).map_err(AccountsError::dbus)
    }
}

#[derive(Debug, Error)]
#[error("AccountsService failed: {message}")]
pub(crate) struct AccountsError {
    message: String,
}

impl AccountsError {
    fn dbus(error: zbus::Error) -> Self {
        Self {
            message: error.to_string(),
        }
    }

    fn invalid_icon_path() -> Self {
        Self {
            message: "the icon path is not valid UTF-8".to_owned(),
        }
    }

    #[cfg(test)]
    pub(crate) fn test_unavailable() -> Self {
        Self {
            message: "test service unavailable".to_owned(),
        }
    }
}

#[proxy(
    default_service = "org.freedesktop.Accounts",
    default_path = "/org/freedesktop/Accounts",
    interface = "org.freedesktop.Accounts"
)]
trait Accounts {
    fn find_user_by_id(&self, id: i64) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    default_service = "org.freedesktop.Accounts",
    interface = "org.freedesktop.Accounts.User"
)]
trait AccountsUser {
    #[zbus(property)]
    fn real_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn icon_file(&self) -> zbus::Result<String>;
}

async fn set_real_name(uid: u32, display_name: &str) -> zbus::Result<()> {
    call_string_method(uid, "SetRealName", display_name).await
}

async fn set_icon_file(uid: u32, icon_file: &str) -> zbus::Result<()> {
    call_string_method(uid, "SetIconFile", icon_file).await
}

async fn call_string_method(uid: u32, method: &str, value: &str) -> zbus::Result<()> {
    let connection = Connection::system().await?;
    let accounts = AccountsProxy::new(&connection).await?;
    let path = accounts.find_user_by_id(i64::from(uid)).await?;
    let user = Proxy::new(
        &connection,
        "org.freedesktop.Accounts",
        path,
        "org.freedesktop.Accounts.User",
    )
    .await?;
    let _: Option<()> = user
        .call_with_flags(
            method,
            MethodFlags::ALLOW_INTERACTIVE_AUTH,
            &DynamicTuple((value,)),
        )
        .await?;
    Ok(())
}

async fn load_details(uid: u32) -> zbus::Result<AccountDetails> {
    let connection = Connection::system().await?;
    let accounts = AccountsProxy::new(&connection).await?;
    let path = accounts.find_user_by_id(i64::from(uid)).await?;
    let user = AccountsUserProxy::builder(&connection)
        .path(path)
        .build()
        .await?;
    let display_name = user.real_name().await?;
    let icon_file = user.icon_file().await?;

    Ok(AccountDetails {
        display_name,
        icon_file: (!icon_file.is_empty()).then(|| PathBuf::from(icon_file)),
    })
}
