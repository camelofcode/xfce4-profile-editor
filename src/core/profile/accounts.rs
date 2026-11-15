// SPDX-License-Identifier: GPL-2.0-or-later

//! AccountsService access for profile data.

use std::path::{Path, PathBuf};

use thiserror::Error;
use zbus::proxy::MethodFlags;
use zbus::{Connection, DynamicTuple, OwnedObjectPath, Proxy, proxy};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AccountDetails {
    pub(crate) display_name: String,
    pub(crate) icon_file: Option<PathBuf>,
}

pub(super) fn load_account_details(uid: u32) -> Result<AccountDetails, AccountsError> {
    zbus::block_on(load_details(uid)).map_err(AccountsError::Dbus)
}

pub(super) fn set_display_name(uid: u32, display_name: &str) -> Result<(), AccountsError> {
    zbus::block_on(set_real_name(uid, display_name)).map_err(AccountsError::Dbus)
}

pub(super) fn set_account_icon(uid: u32, path: &Path) -> Result<(), AccountsError> {
    let path = path.to_str().ok_or(AccountsError::InvalidIconPath)?;
    zbus::block_on(set_icon_file(uid, path)).map_err(AccountsError::Dbus)
}

pub(super) fn clear_account_icon(uid: u32) -> Result<(), AccountsError> {
    zbus::block_on(set_icon_file(uid, "")).map_err(AccountsError::Dbus)
}

#[derive(Debug, Error)]
pub(crate) enum AccountsError {
    #[error("AccountsService failed: {0}")]
    Dbus(#[source] zbus::Error),
    #[error("AccountsService failed: the icon path is not valid UTF-8")]
    InvalidIconPath,
    #[cfg(test)]
    #[error("AccountsService failed: test service unavailable")]
    TestUnavailable,
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
