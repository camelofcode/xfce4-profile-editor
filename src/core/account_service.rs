// SPDX-License-Identifier: GPL-2.0-or-later

//! AccountsService access for the current profile.

use std::path::PathBuf;

use thiserror::Error;
use zbus::{Connection, OwnedObjectPath, proxy};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AccountDetails {
    pub(crate) email: String,
    pub(crate) icon_file: Option<PathBuf>,
}

pub(crate) trait AccountsSource {
    fn details_for_uid(&self, uid: u32) -> Result<AccountDetails, AccountsError>;
}

pub(super) trait EmailClient {
    fn set_email(&self, uid: u32, email: &str) -> Result<(), AccountsError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SystemAccountsSource;

impl AccountsSource for SystemAccountsSource {
    fn details_for_uid(&self, uid: u32) -> Result<AccountDetails, AccountsError> {
        zbus::block_on(load_details(uid)).map_err(AccountsError::dbus)
    }
}

impl EmailClient for SystemAccountsSource {
    fn set_email(&self, uid: u32, email: &str) -> Result<(), AccountsError> {
        zbus::block_on(set_email(uid, email)).map_err(AccountsError::dbus)
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
    fn set_email(&self, email: &str) -> zbus::Result<()>;

    #[zbus(property)]
    fn email(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn icon_file(&self) -> zbus::Result<String>;
}

async fn set_email(uid: u32, email: &str) -> zbus::Result<()> {
    let connection = Connection::system().await?;
    let accounts = AccountsProxy::new(&connection).await?;
    let path = accounts.find_user_by_id(i64::from(uid)).await?;
    let user = AccountsUserProxy::builder(&connection)
        .path(path)
        .build()
        .await?;
    user.set_email(email).await
}

async fn load_details(uid: u32) -> zbus::Result<AccountDetails> {
    let connection = Connection::system().await?;
    let accounts = AccountsProxy::new(&connection).await?;
    let path = accounts.find_user_by_id(i64::from(uid)).await?;
    let user = AccountsUserProxy::builder(&connection)
        .path(path)
        .build()
        .await?;
    let email = user.email().await?;
    let icon_file = user.icon_file().await?;

    Ok(AccountDetails {
        email,
        icon_file: (!icon_file.is_empty()).then(|| PathBuf::from(icon_file)),
    })
}
