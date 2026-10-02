//! Deployment configuration contract tests for production dependency composition.

use std::path::PathBuf;

use transnet::AppConfig;

#[test]
fn production_config_enables_the_island_port_boundary() {
  let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("deploy/transnet.production.toml");
  let config = AppConfig::load(&path).expect("production configuration must parse");

  assert!(config.canonical.enabled);
  assert_eq!(
    config.canonical.socket_path.as_deref(),
    Some("/run/island-port/island-port.sock")
  );
  assert_eq!(config.canonical.timeout_ms, Some(5_000));
  assert!(config.knowledge.enabled);
  assert!(!config.knowledge.required);
  assert_eq!(
    config.knowledge.cursor_secret_env.as_deref(),
    Some("TRANSNET_KNOWLEDGE_CURSOR_SECRET")
  );
  assert!(config.knowledge.cursor_secret_file.is_none());
}

#[test]
fn systemd_unit_orders_transnet_after_island_port_and_loads_external_secrets() {
  let unit = include_str!("../deploy/island.transnet.service");

  assert!(unit.contains("After=network-online.target island-port.service"));
  assert!(unit.contains("Wants=network-online.target island-port.service"));
  assert!(unit.contains("EnvironmentFile=/etc/island/transnet.env"));
  assert!(!unit.contains("TRANSNET_KNOWLEDGE_CURSOR_SECRET="));
}
