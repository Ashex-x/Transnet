//! Fail-closed public-address admission for the live HTTP fetch boundary.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Returns true only for addresses outside every forbidden local, private, documentation, and reserved range.
pub fn is_public_live_address(address: IpAddr) -> bool {
  match address {
    IpAddr::V4(ip) => public_v4(ip),
    IpAddr::V6(ip) => public_v6(ip),
  }
}

fn public_v4(ip: Ipv4Addr) -> bool {
  let n = u32::from(ip);
  ![
    (0x00000000, 8),
    (0x0a000000, 8),
    (0x64400000, 10),
    (0x7f000000, 8),
    (0xa9fe0000, 16),
    (0xac100000, 12),
    (0xc0000000, 24),
    (0xc0000200, 24),
    (0xc0586300, 24),
    (0xc0a80000, 16),
    (0xc6120000, 15),
    (0xc6336400, 24),
    (0xcb007100, 24),
    (0xe0000000, 4),
    (0xf0000000, 4),
  ]
  .into_iter()
  .any(|(base, bits)| n >> (32 - bits) == base >> (32 - bits))
}

fn public_v6(ip: Ipv6Addr) -> bool {
  if let Some(v4) = ip.to_ipv4_mapped() {
    return public_v4(v4);
  }
  let n = u128::from(ip);
  ![
    (0u128, 128),
    (1, 128),
    (0xfc00u128 << 112, 7),
    (0xfe80u128 << 112, 10),
    (0xff00u128 << 112, 8),
    (0x20010db8u128 << 96, 32),
    (0x100u128 << 112, 64),
    (0x20010002u128 << 96, 48),
  ]
  .into_iter()
  .any(|(base, bits)| n >> (128 - bits) == base >> (128 - bits))
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn rejects_private_reserved_and_mapped_addresses() {
    for value in [
      "127.0.0.1",
      "10.0.0.1",
      "100.64.0.1",
      "169.254.1.1",
      "172.16.0.1",
      "192.168.0.1",
      "192.0.2.1",
      "198.18.0.1",
      "203.0.113.1",
      "224.0.0.1",
      "::",
      "::1",
      "fc00::1",
      "fe80::1",
      "ff00::1",
      "2001:db8::1",
      "::ffff:127.0.0.1",
    ] {
      assert!(!is_public_live_address(value.parse().unwrap()), "{value}");
    }
    assert!(is_public_live_address("93.184.216.34".parse().unwrap()));
    assert!(is_public_live_address(
      "2606:2800:220:1:248:1893:25c8:1946".parse().unwrap()
    ));
  }
}
