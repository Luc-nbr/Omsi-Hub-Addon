# Architecture

Omsi-Hub is its own desktop app. It contains no openOMSI code: it works with the openOMSI the
player installed (0.2.22 or newer) through openOMSI's command line and its plugin API, and reads
OMSI 2's files with its own readers. See `PLAN.md` for why, `INVENTORY.md` for what is ported.

It runs on **Windows, macOS and Linux** from the first version. The platform specifics are
collected under [Platforms](#platforms).

## The parts

```
app/            the desktop shell: Tauri 2 (stock), the window, the commands, the bytes route
ui/             the page: Preact + htm (vendor/), our api.js, store.js, i18n.js, the pages
crates/
  hub-omsi      our OMSI 2 readers and writers (no openOMSI code)
  hub-core      the domain: the company, RosterFlow, careers; our timetable types; data folder
  hub-api       the commands: call(cmd, args) routed by prefix
  hub-game      the player's openOMSI: found, checked, started; the plugin installed; the bridge
  hub-locales   the texts (ported as they were)
  hub-online    the account, friends, leaderboard, cloud saves (ported as they were)
plugin/         the Lua plugin that runs inside openOMSI; harness/ tests it on mlua
fixtures/       fake openOMSI and OMSI 2 folders for the setup's tests
```

Dependencies go one way: `app → hub-api → hub-core → hub-omsi`, and `hub-api → hub-game,
hub-locales, hub-online`. `hub-omsi` and `hub-game` depend on nothing of ours.

## The contracts

**`hub-omsi`** (the readers):

- `text`: `Codepage` with `detect`, `decode`, `decode_text` and `encode`; `CfgFile { path,
  lines, codepage, bom }` with `from_bytes`, `read`, `reader()`, `blocks(kw)`, `value(kw)` and
  `to_bytes()`; `Reader` with `next_keyword`, `str`, `i32`, `f64` and `until`; and
  `parse_i32`/`parse_f64`. **Works now.**
- `vfs`: `Vfs::new(root, content)`. It has `resolve(rel)`, `read`, `read_cfg`, `list_dir`,
  `is_file`/`is_dir` and `relative`. It searches the content folder first, then OMSI 2, in any
  case, with either slash. Also `find_child`, `resolve_in` and `missing_omsi2_essentials`.
  **Works now.**
- `timetable`:
  - the types `Timetable { lines, trips, tracks, stn_links, bus_stops }`, `Line`, `Tour`
    (`days: DayMask`), `TourTrip`, `Trip`, `TripStop`, `TripProfile`, `Track`, `TrackEntry`,
    `StnLink` and `BusStopEntry`;
  - their `parse`/`to_text`, `parse_stnlinks`, `parse_busstops`, `replace_block` and
    `stop_times`;
  - `write_text(path, text, codepage)`;
  - the day: `lines_for(&Vfs, map, NaiveDate) -> Vec<ServiceLine>`, with `ServiceTour`,
    `ServiceTrip` and `ServiceStop`.
- `map`: `GlobalCfg`, `Calendar` (`Holidays.txt`: `is_holiday`, `in_holiday_range`),
  `ChronoDir`, `active_chrono_dirs`, `chrono_deactivated_lines`, `AiLists`/`AiGroup`/
  `TypGroup`, `depot_hof_on`, `date_code`.
- `hof`: `Hof { name, service_trip, global_strings, string_count_*, termini, bus_stops,
  info_trips, info_busstop_lists }` with `Terminus`, `BusStop`, `InfoTrip` and `Comments`;
  `load`, `parse`, `read_name`, `to_text`, `has_line`, `depot_files`.
- `vehicle`: `Bus` (the `.bus` header) and `PassengerCabin` (`seats`, `standing`).
- `font::font_names` (`.oft`) and `weather::Weather` (`.owt`).

The types are fixed. The readers marked `todo!()` are the **hub-omsi port**'s work, with tests on
synthetic fixtures. The field names follow what our old code used, so a port mostly changes its
`use` lines. Renamed:

| Old | New |
|---|---|
| `Tour.extra` (the day mask as text) | `Tour.days: DayMask` |
| `Trip.stations`, `stations_legacy` | `Trip.stops: Vec<TripStop>` |
| `TripProfile.factor` | `minutes` |
| `man_dep_time`, `man_arr_time`, `other_stopping` | `fixed_departures`, `fixed_arrivals`, `stopping` |
| `BusStopEntry.group` | `tile` |
| `Vehicle` | `vehicle::Bus` |
| `PassengerCabin.pass_positions` | `places` |
| Calendar and Chrono `i32` dates | `NaiveDate` (`date_code` still gives `YYYYMMDD`) |

**`hub-core`** (the domain):

- `timetable::{LineInfo, TourInfo, TripInfo, StopInfo}`: our own, with the old JSON field names
  (the saves and the pages read them). `list_lines(map, date)` builds them from
  `hub_omsi::timetable::lines_for`, and `DEFAULT_DATE` comes with them.
- `Duty`: a drive as the page sends it, with the old JSON.
- `paths`: `data_dir()`, `root()`, `content_dir()`, `vfs()` and `resolve_content(rel)`.
- `settings::Settings` (`settings.json`).
- `log_line`.
- `migrate::{scan, import}`.
- `display_bus_name` and `vehicle_type_label`.
- `company`: empty. Its module text maps every old `crate::…` the company code uses to its new
  place.

**`hub-api`** (the commands):

- `call(cmd, &Value, &dyn Platform) -> Result<Value, String>`.
- `call_bytes(cmd, &Value, &dyn Platform) -> Result<Vec<u8>, String>`.
- `Platform { emit, quit, emitter }`.
- `arg::<T>(args, name)`, `to_json` and `not_ported(cmd)`.

| Prefix | Module | Now |
|---|---|---|
| `hub_` | `hub`, `drive` | Boot, texts, first-run setup, migration scan, drives |
| `addon_` | `addon` | `addon_state` and `addon_enable`; the rest goes to the plugin port |
| `co_` | `company` | Not ported |
| `rf_` | `rosterflow` | Not ported |
| `tg_` | `together` | Later (needs a relay) |
| `ol_` | `online` | Not ported |
| `cs_` | `cloud` | Not ported |
| `tt_` | `timetable` | Not ported |
| `de_` | `depot` | Not ported |

**`hub-game`** (the game):

- `detect::inspect(folder, home) -> Install { exe, version, version_from, plugins_dir }`.
- `check_folder(folder, home) -> Checked { install, compat }`, and `check_folder_probing(folder,
  home, cache)`, which asks `openomsi --version` once when no file says the version.
- `compat::{FIRST_SUPPORTED = 0.2.22, estimate, negotiate, check}`.
- `install::{install, disable, status, plugin_dir}`.
- `bridge::{Assignment, Hello, Report, write_assignment, read_hello, ReportReader, from_log}`.
- `launch::LaunchSpec::args()` (official flags only, `OFFICIAL_FLAGS`).
- `process::{start, Game, ask_version, ensure_executable}`.
- `find::{omsi2_candidates, openomsi_candidates, steam_roots, steam_libraries, xdg_user_dir}`.

## The data folder

Omsi-Hub keeps its data in its own folder, never in `~/.openomsi`:

- `%APPDATA%\Omsi-Hub` on Windows;
- `~/Library/Application Support/Omsi-Hub` on macOS;
- `$XDG_DATA_HOME/Omsi-Hub` on Linux.

`OMSIHUB_DATA_DIR` moves it (the tests and the selftest use that). It holds `settings.json` (the
two folders, the language, `setup_done`, `migrated`) and `omsi-hub.log`. The ports add the
company saves here under the old file names (`companies/`, `careers/`, `trips/`, `lines/` …).

The old Omsi-Hub's saves in `~/.openomsi` are found by `hub_core::migrate::scan`, which only
reads. The setup offers them once. `import` is a stub until the company port knows the formats.

## Finding and starting openOMSI

1. **The first-run setup** (`ui/js/setup.js`, the `hub_setup_*` commands):
   - The player picks the openOMSI folder, or one `find` offers.
   - `hub_game::check_folder` finds the program (`openomsi.exe`, `openomsi` or the macOS bundle,
     in any case).
   - It reads the version without starting the game where a file says it: the Windows
     `ProductVersion`, the bundle's `Info.plist`, or the folder's name
     (`openOMSI-0.2.30-linux-x64`).
   - Otherwise it asks `openomsi --version` once. openOMSI answers that from its command-line
     parser and ends before any of the game starts. The answer is kept per program file in
     `openomsi-versions.json` in the data folder. This is the usual case on Linux.
   - `compat` refuses anything older than 0.2.22, with a message.
   - The OMSI 2 folder must have `Omsi.exe`, `maps`, `Vehicles`, `Sceneryobjects` and
     `Splines`.
   - Both folders are saved in `settings.json`.
2. **The plugin**: `addon_enable` installs `plugin/*.lua` (embedded at build time) into
   openOMSI's `Plugins/omsihub/`, switches it on and parks it off. The content folder follows
   openOMSI's rule, the same on every system:
   - the program's folder (beside the bundle on macOS), or `openOMSI/` inside the OMSI 2
     folder;
   - `~/.openomsi/content` when that folder may not be written to (Program Files, `/opt`,
     `/Applications`).

   `hub_core::paths::content_dir_of` finds the same folder without writing anything.
3. **A drive** (`hub_drive {duty}`):
   - The page's `Duty` becomes a `LaunchSpec` (`hub_api::drive::launch_spec`): `--root`,
     `--map`, `--bus`, `--line`/`--tour`/`--trip`, `--time`, `--date` … official flags only.
   - What the command line cannot say goes to the plugin as its assignment
     (`hub_game::bridge`): a company's duty, its legs, a free line, the display font.
   - The game starts in its own folder (`process::start`, which sets the executable bit a ZIP
     may have lost), and a thread watches it.
   - When it ends, the page gets `game-ended`. The reports are then read from the plugin's
     `data/` (`bridge::ReportReader`, or the log for an openOMSI without file functions).

## How a feature port plugs in

1. **Data and logic** go into `hub-core` (`company/…`). OMSI 2 files come from `hub-omsi`
   (fill in the `todo!()` readers you need, with synthetic tests), never from openOMSI.
2. **Commands** go into the prefix's module in `hub-api` (`co_` → `company.rs` …):
   - replace `not_ported` with a `match cmd`;
   - read the arguments with `arg`;
   - answer with `to_json`.
   Bytes answers go in `call_bytes`, and the page shows them with `bytesUrl(cmd, args)`.
3. **The page**:
   - a module under `ui/js/` (`ui/js/company/…`), its entry in `ui/js/pages/index.js`;
   - it calls `call` from `api.js`, keeps state through `store.js` (`set`, `useStore`, under its
     own key) and translates with `t` from `i18n.js`;
   - its CSS uses the `--hub-*` tokens (`ui/css/tokens.css`);
   - its mock answers go in `ui/js/mock.js`, so it opens in a browser (`node tools/serve-ui.mjs`,
     then `http://localhost:5174/?mock`).
4. **Texts** go into `crates/hub-locales/locales/*.yml` (English key, then translations).
5. **Checks**:
   - Rust tests beside the code;
   - page checks in `ui/js/selftest.js` (`CHECKS`), run in the app with `OMSIHUB_SELFTEST=1`.
     `OMSIHUB_SELFTEST_FIXTURES=<repo>/fixtures` also goes through the first-run setup.
   - The plugin's behaviour in `plugin/harness/tests/`.

## Building

```
cargo test --workspace          # every crate, the plugin harness, the app's unit tests
cargo build -p omsi-hub         # the app (target/debug/omsi-hub)
cd app && npx @tauri-apps/cli@2 build   # the packages: NSIS; .app/.dmg; AppImage, .deb, .rpm
```

The toolchain is pinned in `rust-toolchain.toml`. CI (`.github/workflows/ci.yml`) runs on
Windows, macOS and Linux (ubuntu-latest):

- fmt, clippy and the tests;
- `node --check` on the page's scripts;
- a Tauri build. On Linux it also makes the AppImage, `.deb` and `.rpm`, and runs the app's
  selftest through the first-run setup under Xvfb.

## Platforms

| | Windows | macOS | Linux |
|---|---|---|---|
| Packages | NSIS installer, with WebView2's bootstrapper | `.app`, `.dmg` | AppImage (the updater's), `.deb`, `.rpm` (`app/tauri.linux.conf.json`) |
| Web view | WebView2 | WKWebView | WebKitGTK 4.1 |
| Data folder | `%APPDATA%Omsi-Hub` | `~/Library/Application Support/Omsi-Hub` | `$XDG_DATA_HOME/Omsi-Hub` (`~/.local/share/Omsi-Hub`) |
| openOMSI's program | `openomsi.exe` | `openOMSI.app/Contents/MacOS/openomsi` | `openomsi`, from the `openOMSI-<version>-linux-x64.zip` release |
| Its version | `ProductVersion`, else the folder's name | `Info.plist`, else the folder's name | the folder's name, else `openomsi --version` (once) |
| Bytes route | `http://hub.localhost/bytes/…` | `hub://localhost/bytes/…` | `hub://localhost/bytes/…` |

**Building on Linux** needs Tauri's packages. On Debian or Ubuntu:

```
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev   librsvg2-dev patchelf libssl-dev build-essential
```

**openOMSI on Linux.** The setup offers the folders it finds (`hub_game::find`):

- `~/openOMSI*`;
- `openOMSI` and `openOMSI-*-linux-x64` folders in the Downloads folder (and the desktop's
  translated name for it, from `~/.config/user-dirs.dirs`), on the Desktop and in `~/Games`,
  `~/Applications`, `~/.local/share` and `/opt`.

A program unpacked without its executable bit gets it before it is started
(`process::ensure_executable`).

**OMSI 2 on Linux** is the Windows game in Steam, through Proton. The setup looks in:

- `~/.steam/steam`, `~/.steam/root` and `~/.local/share/Steam`;
- the Flatpak's `~/.var/app/com.valvesoftware.Steam/.local/share/Steam`;
- every extra library their `libraryfolders.vdf` lists.

In each one it checks `steamapps/common/OMSI 2`.

**File names in any case.** OMSI 2's content names its files carelessly, and Linux's file
systems keep case apart. `hub_omsi::vfs` finds every part of a path in any case; where two names
differ only in case, the exact one wins. `crates/hub-omsi/tests/case_sensitive.rs` proves this on
a case-sensitive temp folder. The Linux CI job sets `OMSIHUB_REQUIRE_CASE_SENSITIVE=1`, so the
test cannot skip there unnoticed. On Windows the same proof runs in a folder made case-sensitive
with `fsutil file setCaseSensitiveInfo <dir> enable`, by pointing `TMP` at it.
