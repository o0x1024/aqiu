// Split from previous monolithic `src-tauri/src/core.rs` into smaller units.
// Keep ordering to preserve item visibility and cfg gating.

include!("base.rs");
include!("macos_and_lifecycle.rs");
include!("tun.rs");
include!("proxy_and_mode.rs");
