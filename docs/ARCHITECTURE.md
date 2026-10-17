# Architecture

Omsi-Hub is its own desktop app. It contains no openOMSI code: it works with the openOMSI the
player installed (0.2.22 or newer) through openOMSI's command line and its plugin API, and reads
OMSI 2's files with its own readers. See `PLAN.md` for why, `INVENTORY.md` for what is ported.

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
  `to_bytes()`; `Reader` with `next_keyword`, `str`, `value`, `values(n)`, `run`, `i32`, `f64`
  and `until`; and `parse_i32`/`parse_f64`.
- `vfs`: `Vfs::new(root, content)`. It has `resolve(rel)`, `read`, `read_cfg`, `list_dir`,
  `is_file`/`is_dir` and `relative`. It searches the content folder first, then OMSI 2, in any
  case, with either slash. Also `find_child`, `resolve_in` and `missing_omsi2_essentials`.
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

**All of it works** (the hub-omsi port), with tests on a synthetic OMSI 2 folder
(`crates/hub-omsi/tests/fixtures`) and an ignored test on a real install
(`HUB_OMSI_REAL_ROOT=<OMSI 2 folder> cargo test -p hub-omsi --test real_data -- --ignored`).
The field names follow what our old code used, so a port mostly changes its `use` lines.
Renamed:

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
| `Vehicle::cameras_driver[0].pos[0]` | `Bus::driver_camera_x` (and `left_hand_drive()`) |
| `Vehicle::scripts.{scripts, constfiles, varlists}` | `Bus::scripts.{scripts, constfiles, varlists}` (`BusScripts`, paths with `/`) |
| `Vehicle::registration_list: Option<(file, _, _)>` | `Bus::registration_list: Option<String>`, `registration_automatic`, `registration_free` |
| `Vehicle::name`, `numbers_with_plates()` | `Bus::name()`, `Bus::numbers` (filled by `Bus::load`) |
| `typgroup_entry_valid(e, d)` | `TypGroupEntry::valid(d)` |
| `ailists_with_chrono(dir, chrono)` | `AiLists::with_chrono(dir, chrono)` |
| `GlobalCfg.raw_tiles` | `GlobalCfg.tiles` |
| `omsi_map::TimeZone` | `map::TimeZone` (`GlobalCfg::load` fills `GlobalCfg.timezone`) |

**Added by the hub-omsi port** (nothing renamed or taken away):

- `text`: `Codepage::Utf16Le`/`Utf16Be` (maps' `global.cfg` and some weather files are UTF-16),
  `Codepage::bom()`/`is_utf16()`; `Reader::value`, `values(n)` and `run` (values that stop at
  the next keyword). **A keyword counts only at the start of its line** (`keyword_of`), as OMSI
  reads it: depot and bus files keep indented examples (`<TAB>[infosystem_trip]`) and
  switched-off keywords (`<TAB>[registration_free]`) in their comments.
- `timetable`: `DayMask::runs_on(&Calendar, date)` and the constants `HOLIDAY`, `SCHOOL_DAYS`,
  `SCHOOL_HOLIDAYS`, `ALWAYS`; `Trip.train_reverse`; `TripStop.extra` (`[station]`'s values
  after the name); `Trip::load`, `Track::load`; `Timetable::stop_name`, `track_of`;
  `leg_lengths`, `trip_length`, `service_lines`, `parse_i64`.
- `map`: `GlobalCfg.timezone` and `TimeZone`/`Dst` (`timezone.txt`); `date_of_code`;
  `ChronoDir.deactivate_lines`, `.title`, `ChronoDir::read`, `active_on`;
  `TypGroupEntry.plate`; `AiLists::update`, `depot_group`.
- `hof`: `Comments::read(&CfgFile)`; `Hof::to_bytes`, `save`, `terminus(code)`, `bus_stop(name)`.
- `vehicle`: `Bus.has_friendly_name`, `paints`, `ctc_dir`, `scripts: BusScripts`, `sound_ai`,
  `driver_camera_x`, `coupling_front`, `number_list`, `registration_automatic`,
  `registration_free`; `Bus::name`, `read_numbers`, `couple_back_path`, `passenger_cabin_path`,
  `left_hand_drive`; `paints_in(dir)`; `capacity(&Bus)` (seats and standing places of all its
  sections).
- `weather`: `Weather.fog_height`, `wind_dir`, `wind_speed`, `dew_point`, `pressure`,
  `cloud_base_m`, `precip_values`, `ground_wet_values`, `snow_on_road`, `codepage`;
  `Weather::to_text`, `save`; `Precipitation::code`/`of_code`.

**What the files turned out to mean** (checked on the OMSI 2 maps, buses and depot files
installed here):

- A tour's day mask: bits 0-6 Monday to Sunday, bit 7 public holidays, bit 8 school days, bit 9
  school holidays. It runs when the day's bit (the public holiday's on one, else the weekday's)
  and its school bit are set; a mask without school bits (older maps) ignores the school
  calendar.
- Chrono folders apply in the order of their names, from `[startdate]` to `[enddate]`. Their
  `.ttl`/`.ttp`/`.ttr` take the place of the map's; their `StnLinks.cfg`/`Busstops.cfg` hold
  only what they add (a link of the same two stops, a stop of the same id, replaced); their
  `ailists.cfg` replaces the map's, their `ailists_#upd.cfg` replaces its groups of the same
  name; `[deactivate_lines]` switch lines off.
- `[standarddepot]` in `global.cfg` names the AI depot group (`ailists.cfg`) whose depot file
  the player's bus gets: `depot_hof_on` takes it, else the first depot group.
- A `.ttp`'s `[trip]` names its track first (empty: the track named like the trip):
  `Trip.display_name` is that value as written, `Trip.track` the track driven.
- In a `.hof`, `stringcount_terminus` and `stringcount_busstop` have no brackets;
  `[addterminus_list]`/`[addbusstop_list]` hold one entry a line, tab-separated; older files
  give a route's stops as `[infosystem_busstop]` one by one.
- `replace_block` tells a `[StnLink]` by its two stops and a `[busstop]` by its object id
  (their first values are a length and a name). The old `replace_block(&text, "")`, which took
  the line editor's marked part out, has no counterpart: an empty block leaves the text as it is.

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
- `check_folder(folder, home) -> Checked { install, compat }`.
- `compat::{FIRST_SUPPORTED = 0.2.22, estimate, negotiate, check}`.
- `install::{install, disable, status, plugin_dir}`.
- `bridge::{Assignment, Hello, Report, write_assignment, read_hello, ReportReader, from_log}`.
- `launch::LaunchSpec::args()` (official flags only, `OFFICIAL_FLAGS`).
- `process::{start, Game, ask_version}`.
- `find::{omsi2_candidates, openomsi_candidates}`.

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
   - It reads the version without starting the game: from the Windows `ProductVersion`, the
     bundle's `Info.plist` or the folder's name.
   - `compat` refuses anything older than 0.2.22, with a message.
   - The OMSI 2 folder must have `Omsi.exe`, `maps`, `Vehicles`, `Sceneryobjects` and
     `Splines`.
   - Both folders are saved in `settings.json`.
2. **The plugin**: `addon_enable` installs `plugin/*.lua` (embedded at build time) into
   openOMSI's `Plugins/omsihub/`, switches it on and parks it off. The content folder follows
   openOMSI's rule: the program's folder, `openOMSI/` inside the OMSI 2 folder, or
   `~/.openomsi/content`.
3. **A drive** (`hub_drive {duty}`):
   - The page's `Duty` becomes a `LaunchSpec` (`hub_api::drive::launch_spec`): `--root`,
     `--map`, `--bus`, `--line`/`--tour`/`--trip`, `--time`, `--date` … official flags only.
   - What the command line cannot say goes to the plugin as its assignment
     (`hub_game::bridge`): a company's duty, its legs, a free line, the display font.
   - The game starts in its own folder (`process::start`), and a thread watches it.
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
cd app && npx @tauri-apps/cli@2 build   # the installers (NSIS, .app/.dmg)
```

The toolchain is pinned in `rust-toolchain.toml`. CI (`.github/workflows/ci.yml`) runs fmt,
clippy, the tests, `node --check` on the page's scripts, and a Tauri build on Windows and macOS.
