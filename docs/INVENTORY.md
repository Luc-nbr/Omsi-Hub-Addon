# Phase 1: inventory of Omsi-Hub's code in the old repository

Source: `Luc-nbr/openOMSI` (the interim fork, formerly `omsi-hub`), branch `beta` (2026-10-17).
Upstream: `openOMSI-org/openOMSI` `main` (f571ca81). The merge-base is 55e0c68e.
Four read-only surveys produced this document, one per area.

## Summary

- **About 1,000 files and about 230k lines are Omsi-Hub's own.** Most of the company code uses
  **no openOMSI crate at all**: about 47k lines of `company/*` depend only on the upstream launcher's
  timetable types (`LineInfo`/`TourInfo`/`TripInfo`/`StopInfo`), on `Duty`, and on a few helpers.
  These port almost as they are.
- **The real openOMSI dependencies are concentrated in a few places:**
  - Reading OMSI 2 files: timetables, `.hof`, `.bus`, `global.cfg`, `ailists.cfg`, `Holidays.txt`
    and `.oft`. This becomes **`hub-omsi`**, moderate work.
  - **The 3D bus preview and the livery studio:** `.o3d`, `model.cfg`, textures, DDS/BC encoding, and
    running the bus's scripts for its look. This is the **largest and riskiest** part.
  - **The game's renderer** (`Platform::renderer`), used for:
    - bus photos;
    - the fleet map;
    - the display-font preview;
    - the line editor (`hub_le`);
    - the livery studio (`hub_studio`).

    These are redone in the app (WebGL, or our own map drawing) or dropped.
  - **Play together:** it uses openOMSI's tunnel and **openOMSI's bridge relay server** (`omsi_net`).
    It needs our own relay, or LAN/VPN only.
  - **The upstream launcher shell and pages:** a minimal rebuild of about 1.8–2k lines.
- **The plugin and its bridge** (`omsihub-plugin`) are already independent of openOMSI internals:
  plain files in `Plugins/omsihub/` (`BRIDGE = 1`). `omsihub-online` and `omsihub-locales` have no
  openOMSI dependency at all. All three port as they are.
- **The minimum official openOMSI is 0.2.22,** not 0.2.0. `duty.get` and `duty.trips` came with the
  plugin API rewrite in v0.2.22. This is fixed in the old repository's `compat.rs`.

## 1. Company core (`crates/omsi-launcher-core`, ours: about 75k lines, 632 tests)

| Module | LOC | openOMSI used | Plan |
|---|---|---|---|
| `company/*` (about 55 files: plan, studio, staff, clock, day, concessions, contracts, messages, crew, cloud, store, career, …) | 46.7k | none | **Port as is**, with our own copies of `LineInfo`/`TourInfo`/`TripInfo`/`StopInfo`/`Duty` |
| `company/rosterflow*`, `plandays*`, `econ_uk*`, `compose.rs`, `busoptions.rs`, `qr*` | 10k | none | Port as is |
| `lines.rs` (own lines; registry and TTData export) | 3.2k | `omsi_timetable` 25, `omsi_cfg` 7, `omsi_map` 1 | hub-omsi **TTData reader and writer** (`.ttl`/`.ttp`/`.ttr`, `StnLinks.cfg`, `Busstops.cfg`), keeping the codepage |
| `linehof.rs`, `owndepot.rs`, `depot.rs` | 3.3k | `omsi_vehicle::Hof` …, `omsi_cfg` VFS/codepage | hub-omsi **`.hof` reader and writer**, `ailists.cfg` reader, our own path resolver |
| `service.rs`, `company/agenda.rs` | 1.8k | `omsi_map::Calendar` | hub-omsi `Holidays.txt` reader |
| `company/dealer.rs`, `market.rs`, `registrations.rs`, `vehicles.rs` | 3.8k | `omsi_vehicle::Vehicle::load`, `PassengerCabin` | hub-omsi **`.bus` reader** plus `passengercabin.cfg` |
| `busfonts.rs` | 0.4k | `omsi_content::dotfont`, `font` | `.oft` name reader plus TTF faces, or drop |
| `ttstore.rs` | 0.2k | `omsi_cfg` VFS | Our own file helper |
| `company/together/*` | 3.8k | `omsi_net` tunnel, bridge, hostkey, http | Our own helpers. **The relay (bridge) is openOMSI's server**: we need our own relay or LAN only |

**Upstream launcher items our code calls** (to rebuild in `hub-core`):
- `list_lines` / `lines_on` (the timetable for a date, with Chrono and the calendar applied);
- the timetable types;
- `Duty`;
- `resolve_content`, `data_dir`, `content_dir`, `root`, `bases`, `merged_folders`;
- `log_line`, `display_bus_name`, `vehicle_type_label`, `VehicleInfo`/`list_vehicles`;
- `DEFAULT_DATE`.

Stop times are interpolated as link length × profile factor.

**Launching.** `--company`, `--duty-leg` and `--free-line` are game arguments only our game knows. On
official openOMSI the plugin does these instead: `duty.start`, with a relief mid-trip and multi-leg
chaining on finish. The company-only AI (`--company`) needs an upstream PR, or it is dropped.

**Saves to migrate** (all under `~/.openomsi`):
- `companies/<id>.json` (VERSION 3), plus `.live.jsonl`, `.orders.jsonl`, `.inbox.json`, `.liveplan.json` and `.taken`;
- `careers/<driver>.json`, `trips/<driver>.jsonl`, `hidden-drivers.txt`;
- `lines/<map>.json` (`REGISTRY_VERSION` 2);
- `depots/*.hof`, `bus-fonts.json`, `bus-options.json`, `vehicle-classes.json`;
- `settings.cfg`: Omsi-Hub keys mixed with upstream keys;
- `launcher.json`.

In the map's `TTData`: `openomsi-lines.txt`, `openomsi-maplines.txt`, `oo_vehicles.json`, `oo_*` and `.orig`.

## 2. API layer (`crates/omsi-launcher-api/src/omsihub`, 42k lines, 146 tests)

| Module | LOC | openOMSI used | Plan |
|---|---|---|---|
| `company*`, `company_more/*` | 21.6k | a little: `omsi_cfg` paths, `Hof`, Chrono, `omsi_sim::timetable_run::{TripTimes, ibis_target}` | Port, with hub-omsi and our own trip-times/IBIS helper. Fleet map: our own drawing |
| `rosterflow*` | 7.1k | `omsi_timetable`, `GlobalCfg`, Chrono, ailists | Port onto hub-omsi |
| `editors/*` | 1.4k | `.hof`, timetable read/write, **renderer** (`hub_le`, `hub_studio`, display preview) | Timetable and depot editors: port. Line editor: needs our own lane graph from the map (see §4). Studio and preview: web UI |
| `hub_more*` | 1.6k | `omsi_sim::texttex` (displays), weather `.owt`, ailists, `Model::load` | Weather and ailists: hub-omsi. **Display font / texttex: drop, or a simple bitmap preview** |
| `bus3d/*` | 2.7k | `omsi_sim::VehicleType`/`VehicleInstance` (**runs the bus scripts**), `omsi_o3d`, `omsi_model`, `omsi_texture` | **Biggest risk.** hub-omsi `.o3d` (about 800 LOC), `model.cfg` (about 1k), `.bus` (about 500) and textures/DDS (about 600). The look needs a `[visible]`/varlist evaluator from paint vars and setvars instead of the script engine |
| `lak/*` + `omsihub-livery` | 1.2k + 6k | textures, a DDS/BC1/BC3 encoder, CTC paint schemes, `Model::load` | hub-omsi texture module plus a DDS encoder (the `image` and texture crates on crates.io are fine: they are not openOMSI) |
| `together*` | 2.6k | `omsi_net` addrs, hostkey, random | Our own helpers |
| `addon.rs`, `online.rs`, `cloud.rs`, `privacy.rs`, `ontime.rs` | 3.3k | none (`addon` uses `omsihub-plugin`) | Port as is |
| `update.rs` | 0.2k | `omsi_platform::updater` | Our own GitHub-releases updater |
| `mod.rs` (dispatcher) | 0.7k | some `omsi_cfg`, `Line::load`, timezone, metar | Port; hub-omsi bits |

**The UI-to-Rust bridge** is upstream's mechanism:
- page: `api.js` `call()`;
- Tauri: the commands `call` and `call_bytes`;
- Rust: `omsi_launcher_api::call`, with our `omsihub::call` as the fallback;
- `protocol.rs` route `omsi://localhost/bytes/<cmd>`.

The new app keeps the same shape with its own minimal `Platform` (`emit`, `quit`). There are about
170+ Omsi-Hub commands (prefixes `co_`, `rf_`, `tg_`, `ol_`, `cs_`, `lak_`, `b3d_`, `hub_`, `addon_`,
`tt_`, `de_`).

## 3. UI and shell (`launcher/`: ours about 38k JS plus 7k CSS)

- **Our areas** port as they are: company 11.4k, rosterflow 9k, bus3d with lak 7.5k, editors 3.3k,
  steps 2k, welcome, online, tour, cloud, privacy, theme, home, bar and others. They import upstream
  modules that the new app needs its own versions of:
  - `store.js` (81);
  - `api.js` (110);
  - `data.js` (457; the boot, choice, launch and lists);
  - `i18n.js` (87);
  - `ui.js` (218);
  - `pages/drive.js` helpers;
  - `routemap.js`, `preview.js`, `crash.js`, `symbols.js`.

  `vendor/preact-htm.js` (Preact MIT + htm Apache-2.0) is a third-party library and fine to keep.
- **Upstream pages embedded today** (mods, controls, multiplayer, profile, games, tutorials,
  modeltest, setup): drop them. Controls and mods are openOMSI's own business.
- **The shell:** almost all upstream. The new app gets its own Tauri 2 shell, on stock Tauri rather
  than openOMSI's Tauri fork. It has:
  - the commands `call`, `call_bytes`, `pick_folder`/`pick_file`, `open_url`/`open_path`;
  - the bytes protocol;
  - window state and the selftest mode;
  - the updater;
  - finding and starting openOMSI;
  - installing the plugin.

  It drops `hand_over_to_game` and the WebView2 check, which the game did. The app checks for WebView2
  itself.
- **The minimal rebuild of the upstream launcher parts is about 1.8–2k lines.**
- **Assets:**
  - Hanken Grotesk (OFL), our images and our icon tables: keep. The icons come from Material Symbols
    (Apache-2.0), which is fine.
  - **The app icons are openOMSI's,** so we need our own.
  - **The logo (`logo.js`) is built on openOMSI's mark** (the ring, the bus, the line). **Decision
    needed:** a new Omsi-Hub mark of our own.
  - Add `*Switzer*` to `.gitignore`.
- **Selftests:** six JS selftests; they need our own runner (about 80 lines) instead of upstream's
  `selftest.js` driver.

## 4. Plugin, game side, companion

- **`omsihub-plugin`** (src 2k, lua 3k, tests 0.9k): **keep as is.** The runtime has no openOMSI
  dependency. The tests run the Lua inside `omsi-plugin`: replace that with our own small Lua harness
  (mlua) so the tests don't need openOMSI.
- **`omsihub-game`** (17k, all on `omsi_ui`/`omsi_render`/`omsi_sim`) is split:
  - **Line editor** (4.8k, already "headless"; the page draws it): port with our own map reader and our
    own **lane graph** (`.map` tiles, `.sli` splines). Large.
  - **Livery studio drawing** (`studio.rs`, 2.9k of `omsi_ui`): web UI with WebGL. Large.
  - **Display-font preview, bus photos:** bitmap from our `.oft` reader, or drop.
  - **Bus kinds and options, spawn arguments:** port with the `.bus` reader.
  - **Drop** (in-game drawing): the navigator duty board (`board.lua` already covers it), stop signs,
    the pause menu, splash and intro.
- **Game-side extras and what they need upstream:**

  | Extra | How it can be done |
  |---|---|
  | Pause menu | (c) cannot be an add-on; an upstream "pause-menu entries" hook would allow a part of it |
  | Display font | (b) PR: `bus.set_display_font` |
  | IBIS auto-typing | (b) PR: `duty.set_auto_ibis`. Typing itself is mostly possible today via `bus.set_line`/`set_destination` |
  | Closed stops on the navigator | (b) PR: `duty.set_closed_stops` |
  | Navigator dock, plugin UI v3 (boxes, stop lists, bar segments, Hanken font, `grab`) | (b) PRs on `omsi-plugin-ui`. The Lua already falls back on UI v2 |
  | Relief mid-trip, multi-leg duties | (a) possible today via `duty.start` and chaining |
  | `--free-line`, company-only AI | (b) CLI PR; the company-only AI is effectively (c) |

  The upstream navigator moved to `crates/omsi-navigator`, so every navigator patch must be redone
  against it.
- **The companion (phone/tablet):**
  - **Portable as is:** `http`, `pairing`, `panels`, `signon`, the static page, and the company tab.
    The app can serve these itself.
  - **The live navigator on the phone:** possible but coarser. The plugin writes `live.json` at about
    1 Hz, and the roads come from our map reader.
  - **Bus screens, live cab textures and taps:** need an upstream PR (screen texture as a PNG, plus a
    click at uv).
- **The bridge data** (`assignment.lua`, `texts.lua`, `data/*.json[l]`) is fully independent of
  openOMSI. Keep it.

## 5. Decisions for Luc

1. **The livery studio and the 3D preview:** rebuild them with our own readers and WebGL (large), or
   ship the first version of the new app without them and add them later?
2. **The line editor** (needs our own lane graph from the map): in the first version, or later?
3. **Play together:** our own relay server (hosting cost), or LAN/VPN only at first?
4. **Our own logo and mark**, because today's logo is built on openOMSI's mark.
5. **What the first version must contain.** The proposal: the company plus RosterFlow, online,
   cloud, the plugin (duties, the board, messages, on-time, crew), the timetable and depot editors,
   and settings and updates. The 3D preview, the livery studio and the line editor follow later.
