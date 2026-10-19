# The companion: handing over to a session on Luc's computer

Written at the end of the cloud session that built the companion (October 2026). That session
had no OMSI 2 and no openOMSI: everything below ran against mocks, a synthetic bus and the
harness's fake game. The next step needs the real game, so it continues on Luc's computer.

## What there is

Two repositories, both on branch `claude/pensive-planck-lan958` (no pull requests yet):

| Repository | What |
| --- | --- |
| `Omsi-Hub/Omsi-Hub-Addon` (this one) | `crates/hub-companion` (the server the app pairs with), `crates/hub-devices` (a bus's devices read from its own files), `crates/hub-api/src/companion.rs` (run by the app), `plugin/companion.lua` (plugin 1.11.0), Settings › General › Phone & tablet, `plugin/harness/examples/fakegame.rs` |
| `Omsi-Hub/Omsi-hub-companion-app` | The app (Expo SDK 58, iPhone, iPad, Android): navigator, stops, trips, duty, messages, break, widgets, the IBIS and ticket printer |

The protocol between them is `docs/COMPANION.md` here, the same text as `docs/PROTOCOL.md` in
the app; keep the two the same.

**The devices drawn as the game draws them** (the last piece, and the one to prove now): per
bus, `hub-devices` reads `model.cfg`, the `.o3d` parts, the textures and the fonts and makes a
form of each device's screen - the pictures of its pages as triangles, its text fields in the
bus's font, its touch fields with their triggers. The plugin sends the variables the forms need
ten times a second; the app draws the form with Skia and a tap fires the trigger
(`omsi.trigger`). It is a port, rule by rule, of Luc's career add-on
(`Luc-nbr/Omsi-career-addon`: `core/busmodule.ts`, `core/schermvorm.ts`, `shared/scherm.ts`,
`renderer/apparaatscherm.tsx`; HANDOVER.md §5.0 there), with one simplification: openOMSI fires
a trigger directly, so nothing is written into `keyboard.cfg`.

## Proven, and not

Proven (here): the Rust tests (hub-devices, hub-companion, the plugin on the harness, hub-api);
the app's typecheck and lint; the app's web build against the real `hub-companion` server and
the real Lua plugin on the harness (`fakegame`): pairing by code and by the QR link, the
navigator, the duty, IBIS keys and tickets reaching the bus, and a device form of the synthetic
bus drawn on iPhone and iPad size, where a tap on its MENU field pressed `IBIS_menu` in the game
and the menu page showed.

**Not proven - the work for this session, in this order:**

1. **The plugin on the real openOMSI.** Mocked only: `omsi.bus.file`, `omsi.bus.triggers`,
   `omsi.trigger`, `omsi.press` / `omsi.release` (API 5), `bus.get_vars` / `get_strings` (does
   a NaN pass?), `omsi.vars("str")`, `map.lane().name`, `world.date`, `world.time`, and whether
   `omsi.every(0.1)` really fires ten times a second. Check `hello.json` and the game log.
2. **Real buses' forms.** Start with the bus that worked in the career add-on: the HH20's ALMEX
   (`vehicles/hh20_ebus2021`, `model_21_main.cfg`, menu 0 and menu 26, `17_almex_s_0.jpg` with
   uv v -0,88..-0,12). Then a stock MAN (the Krüger IBIS) and the Setra's AFR 200 (five
   displays on one body texture). Compare the app's drawing with a screenshot of the cab.
   Uncertain in the port: resolving `bus.file()` through the content folders (fonts and shared
   textures of a bus in a mod folder), encrypted `.o3d` files, which touch field wins where
   two overlap, the alpha-test threshold of 128.
3. **Visibility from numbers only.** openOMSI has no call for the game's own per-mesh
   visibility (the career add-on read it from memory, `z` in its stand). Where a page or a
   field shows wrongly, the `[visible]` variables and the `[animparent]` chain are where to
   look; a plugin API call for mesh visibility would settle it (an upstream PR).
4. **The app on a real iPhone, iPad and Android** (an EAS development build; Skia, the camera,
   the widgets, the Live Activity and the Android notification have only run in a browser).
   Long press (hold and release) has not been exercised.
5. Then the pull requests.

## Running it on this computer

```sh
# the add-on: build and start it, turn on Settings › General › Phone & tablet
cargo build -p omsi-hub && cargo run -p omsi-hub

# the app, in the browser on the same computer or a phone in the same network
cd ../Omsi-hub-companion-app && npm install && npm run web
# or on a device: npx eas-cli@latest build --profile development (a development build)

# without the game: the plugin on the harness with the server and a synthetic bus
cargo run -p hub-plugin-harness --example fakegame   # prints the pairing code
```

Pair the app with the address and code the launcher (or `fakegame`) shows; after one pairing
the code changes. Built forms are cached in the data folder under `companion-forms/` (keyed by
the model's files and their times): delete it after changing `hub-devices`.

## Where to look when something is wrong

| Symptom | Where |
| --- | --- |
| No devices for a bus | `hub-devices` grouping (`busmodule` port); the bus file the plugin reports (`live.bus_file`) |
| A device but no form | the anchor vote / flat surface (`schermvorm` port); `incomplete` in the form says what could not be drawn |
| The picture in the wrong place | the `[animparent]` chain and its live values (asked from the plugin before building, 3 s at most) |
| A field shows or hides wrongly | `nums` / `[visible]` and Delphi rounding; see point 3 above |
| A tap does nothing | the trigger in `taps` and whether the plugin's whitelist has it (`companion.json`), then `omsi.trigger` in the game log |
| Text in the wrong place or font | the font JSON (`oft` port) and `src/lib/font-text.ts` in the app |
