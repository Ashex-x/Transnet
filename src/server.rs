//! Safe ownership and serving of the target HTTP/1.1 Unix-domain socket.

use std::{
  fs,
  future::Future,
  io,
  os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt},
  path::{Component, Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use axum::Router;
use hyper_util::{rt::TokioIo, server::conn::auto::Builder, service::TowerToHyperService};
use tokio::{net::UnixListener, task::JoinSet};

const CONNECTION_DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Linux-compatible maximum pathname length excluding the trailing NUL byte.
pub const MAX_SOCKET_PATH_BYTES: usize = 107;

/// Validated configuration for one owned Unix-domain listener.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnixListenerConfig {
  path: PathBuf,
  mode: u32,
}

impl UnixListenerConfig {
  /// Validates an absolute, bounded socket path and an owner-writable octal mode.
  ///
  /// # Errors
  ///
  /// Returns an error for an unsafe path or a mode outside `0600..=0770`.
  pub fn new(path: impl Into<PathBuf>, mode: &str) -> Result<Self> {
    let path = path.into();
    let path_text = path
      .to_str()
      .context("server.socket_path must be valid UTF-8")?;
    if !path.is_absolute()
      || path_text.len() > MAX_SOCKET_PATH_BYTES
      || path_text.chars().any(char::is_whitespace)
      || path
        .components()
        .any(|component| component == Component::ParentDir)
    {
      bail!("server.socket_path must be an absolute bounded path without whitespace or '..'");
    }
    let parsed_mode =
      u32::from_str_radix(mode, 8).context("server.socket_mode must be a four-digit octal mode")?;
    if mode.len() != 4 || !mode.starts_with('0') || !(0o600..=0o770).contains(&parsed_mode) {
      bail!("server.socket_mode must be a four-digit octal mode from 0600 through 0770");
    }
    Ok(Self {
      path,
      mode: parsed_mode,
    })
  }

  /// Returns the validated socket path.
  pub fn path(&self) -> &Path {
    &self.path
  }
}

/// An owned listener that removes only the exact socket inode it created.
#[derive(Debug)]
pub struct OwnedUnixListener {
  listener: UnixListener,
  path: PathBuf,
  device: u64,
  inode: u64,
}

impl OwnedUnixListener {
  /// Proves an existing socket is inactive, binds a new listener, and applies its mode.
  ///
  /// The parent directory must already exist and be managed by the service supervisor.
  ///
  /// # Errors
  ///
  /// Refuses active sockets, non-socket filesystem entries, missing parents, and unsafe paths.
  pub async fn bind(config: &UnixListenerConfig) -> Result<Self> {
    let parent = config
      .path
      .parent()
      .context("server.socket_path has no parent")?;
    let parent_metadata =
      fs::symlink_metadata(parent).context("socket parent must already exist")?;
    if !parent_metadata.is_dir() || parent_metadata.file_type().is_symlink() {
      bail!("socket parent must be an existing non-symlink directory");
    }

    match fs::symlink_metadata(&config.path) {
      Ok(metadata) => {
        if !metadata.file_type().is_socket() {
          bail!("refusing to replace a non-socket entry at server.socket_path");
        }
        match tokio::net::UnixStream::connect(&config.path).await {
          Ok(_) => bail!("refusing to replace an active Unix socket"),
          Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => {
            fs::remove_file(&config.path).context("failed to remove proven stale Unix socket")?;
          }
          Err(error) => return Err(error).context("could not prove existing Unix socket is stale"),
        }
      }
      Err(error) if error.kind() == io::ErrorKind::NotFound => {}
      Err(error) => return Err(error).context("failed to inspect server.socket_path"),
    }

    let listener = UnixListener::bind(&config.path).context("failed to bind server.socket_path")?;
    fs::set_permissions(&config.path, fs::Permissions::from_mode(config.mode))
      .context("failed to set server.socket_mode")?;
    let metadata = fs::symlink_metadata(&config.path).context("failed to inspect bound socket")?;
    Ok(Self {
      listener,
      path: config.path.clone(),
      device: metadata.dev(),
      inode: metadata.ino(),
    })
  }

  /// Serves HTTP/1.1 connections until shutdown, then drains accepted connections.
  ///
  /// # Errors
  ///
  /// Returns when accepting a connection fails unexpectedly.
  pub async fn serve(self, router: Router, shutdown: impl Future<Output = ()>) -> Result<()> {
    let mut connections = JoinSet::new();
    tokio::pin!(shutdown);
    loop {
      tokio::select! {
        _ = &mut shutdown => break,
        accepted = self.listener.accept() => {
          let (stream, _) = accepted.context("Unix listener accept failed")?;
          let service = TowerToHyperService::new(router.clone());
          connections.spawn(async move {
            if let Err(error) = Builder::new(hyper_util::rt::TokioExecutor::new())
              .serve_connection(TokioIo::new(stream), service)
              .await
            {
              tracing::debug!(error = %error, "Unix HTTP connection ended with an error");
            }
          });
        }
      }
    }
    if tokio::time::timeout(CONNECTION_DRAIN_TIMEOUT, async {
      while connections.join_next().await.is_some() {}
    })
    .await
    .is_err()
    {
      connections.abort_all();
      while connections.join_next().await.is_some() {}
    }
    Ok(())
  }
}

impl Drop for OwnedUnixListener {
  fn drop(&mut self) {
    let Ok(metadata) = fs::symlink_metadata(&self.path) else {
      return;
    };
    if metadata.file_type().is_socket()
      && metadata.dev() == self.device
      && metadata.ino() == self.inode
    {
      if let Err(error) = fs::remove_file(&self.path) {
        tracing::warn!(error = %error, "failed to remove owned Unix socket");
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::{fs::File, os::unix::fs::PermissionsExt};

  use super::*;

  fn temporary_directory(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("transnet-{name}-{}", ulid::Ulid::new()));
    fs::create_dir(&directory).unwrap();
    directory
  }

  #[test]
  fn configuration_rejects_unsafe_paths_and_modes() {
    assert!(UnixListenerConfig::new("relative.sock", "0660").is_err());
    assert!(UnixListenerConfig::new("/run/transnet/../other.sock", "0660").is_err());
    assert!(UnixListenerConfig::new("/run/transnet/bad socket", "0660").is_err());
    assert!(UnixListenerConfig::new("/run/transnet/transnet.sock", "0777").is_err());
    assert!(UnixListenerConfig::new("/run/transnet/transnet.sock", "660").is_err());
    assert!(UnixListenerConfig::new("/run/transnet/transnet.sock", "0660").is_ok());
  }

  #[tokio::test]
  async fn bind_refuses_non_socket_entries_and_active_sockets() {
    let directory = temporary_directory("bind-refusal");
    let path = directory.join("transnet.sock");
    let config = UnixListenerConfig::new(&path, "0660").unwrap();
    File::create(&path).unwrap();
    assert!(OwnedUnixListener::bind(&config).await.is_err());
    fs::remove_file(&path).unwrap();

    let active = UnixListener::bind(&path).unwrap();
    assert!(OwnedUnixListener::bind(&config).await.is_err());
    drop(active);
    fs::remove_file(&path).unwrap();
    fs::remove_dir(&directory).unwrap();
  }

  #[tokio::test]
  async fn bind_replaces_only_stale_socket_and_cleans_owned_inode() {
    let directory = temporary_directory("owned-lifecycle");
    let path = directory.join("transnet.sock");
    let stale = UnixListener::bind(&path).unwrap();
    drop(stale);
    let config = UnixListenerConfig::new(&path, "0660").unwrap();

    let owned = OwnedUnixListener::bind(&config).await.unwrap();
    assert_eq!(
      fs::metadata(&path).unwrap().permissions().mode() & 0o777,
      0o660
    );
    drop(owned);
    assert!(!path.exists());
    fs::remove_dir(&directory).unwrap();
  }
}
