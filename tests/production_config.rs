//! Deployment configuration contract tests for production dependency composition.

use std::path::PathBuf;

use transnet::AppConfig;

#[test]
fn production_config_runs_model_only_without_unpublished_canonical_data() {
  let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("deploy/transnet.production.toml");
  let config = AppConfig::load(&path).expect("production configuration must parse");

  assert_eq!(config.http.max_request_body_bytes, 12 * 1_048_576);
  assert!(!config.canonical.enabled);
  assert!(config.canonical.resolve().unwrap().is_none());
  assert!(!config.knowledge.enabled);
  assert!(!config.knowledge.required);
  assert!(config.knowledge.resolve(None).unwrap().is_none());
  assert!(config.knowledge.cursor_secret_env.is_none());
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
