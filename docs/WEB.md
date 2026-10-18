# The bus company on the website

The company mode, RosterFlow included, runs on omsihub.com for a company that is in the cloud
(a cloud save, `ONLINE.md`). It is the app's own code: no second version to keep up.

- **Rust:** `crates/hub-web` builds hub-api for the browser (`wasm32-wasip1`), with
  `hub-api` and `hub-core` without their "native" feature. That leaves out what needs the
  computer: the game and the plugin, openOMSI's readers (the 3D view, the livery studio, the line
  editor, the map's tiles), the network (cloud, account, updates, Play together). Work that runs
  on a thread on the desktop runs after the command on the web (`hub_api::spawn`, `run_later`).
- **Files:** WASI's folders live in the page's memory (`ui/vendor/browser_wasi_shim`): `/data` is
  Omsi-Hub's data folder, `/omsi` the OMSI 2 content. The save is laid out in `/data` as the
  desktop lays one out (`hub_install`: `cloud::install`), and packed again the same way
  (`hub_pack`, zstd by `ruzstd`; the desktop reads it).
- **Page:** `ui/js/web/backend.js` starts the module and answers `api.js` (`window.__OMSIHUB_WEB__`,
  `onWeb`); `ui/js/web/company.js` mounts the app's company screen alone.
- **Build:** `node tools/build-web.mjs [folder]` makes `dist/web/` (the module, `profile.web`,
  and the page files); the website serves that folder.

## What the save does not carry yet: the content

Without the OMSI 2 files the company's own data shows (money, staff, fleet, agenda, inbox), but
what reads the map or the buses has nothing to read. The desktop will send a content pack beside
the save; what it must hold (paths under the OMSI 2 folder):

| What | Files | Read by |
|---|---|---|
| The day's lines | `maps/<m>/global.cfg`, `timezone.txt`, `Holidays.txt`, `ailists.cfg`, `TTData/*` (`.ttl`, `.ttp`, `.ttr`, `StnLinks.cfg`, `Busstops.cfg`, `.ttl.orig`), `Chrono/*/Chrono.cfg` with their `TTData/` and `ailists*.cfg` | `hub_omsi::timetable::lines_for`, RosterFlow's timeplan, the duty card |
| Buses (all of the computer's) | `Vehicles/<pack>/*.bus`, what each names: the `[model]` cfg (only its `[CTC]`), the CTC folder's `*.cti`, the number and registration lists, `passengercabin.cfg`, a `couple_back` rear part | `hub_omsi::vehicle::Bus::load`, the dealer, the market, registrations |
| Depot files | `*.hof` in `Vehicles/<pack>/`, `Vehicles/<pack>/<sub>/` and the map's folders | `linehof`, `depot`, the duty card |
| Map facts | `laststn.osn` and `Situations/*.osn` (their header), `Weather/ICAO.txt`, `Weather/*.owt`, `picture.*` | `content/facts.rs`, the weather |
| Data folder | `lines/<map>.json`, `depots/*.hof`, `trips/<driver>.jsonl`, `careers/<driver>.json`, `bus-fonts.json`, `bus-options.json`, `vehicle-classes.json`, the bus photos (`cache/busphotos/`), the logo | the company's pages |
| The map drawn | `web/map.json`: the desktop's `mapview::fleet_export` (roads, colours, each trip's route and stops) | `content/mapview_web.rs` |

Every folder on the way must be listable (names are found in any case), and the bus photos are
named by the bus file's path and time: the pack keeps the names the desktop gave them.
