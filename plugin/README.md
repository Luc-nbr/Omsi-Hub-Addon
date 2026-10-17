# The plugin

Omsi-Hub's Lua plugin: the only part of Omsi-Hub that runs inside the player's openOMSI, on its
official plugin API. `hub-game` installs these files into openOMSI's `Plugins/omsihub/` and talks to
the plugin through plain files (`crates/hub-game/src/bridge.rs`).

- `main.lua`: the entry point (the duty, the reports, the handshake); the other files are its
  modules (`board`, `card`, `career`, `company`, `duty_strip`, `messages`, `ontime`, `signon`,
  `trip`, `watch`, `hubjson`).
- `harness/`: our own test harness (`hub-plugin-harness`, on mlua). It mocks the official API
  functions the plugin uses, so `cargo test` runs the plugin without openOMSI.

A change to a `.lua` file needs the plugin's `VERSION` raised in `main.lua` and in
`crates/hub-game/src/install.rs` (a test keeps the two the same).
