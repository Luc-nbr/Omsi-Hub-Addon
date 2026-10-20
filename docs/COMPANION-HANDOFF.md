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

Proven on Luc's computer (20 October 2026; openOMSI 0.3.37 in `E:\Open-Omsi Official`, OMSI 2
from Steam, Grundorf, `Vehicles/HH20_EBus2021/HHEBus2021_main.bus` started with `--autostart`;
the server beside it was `plugin/harness/examples/realgame.rs`, the app its web build):

- **The plugin in the real game** (point 1 below, done). Every function the companion uses is
  there (`game.api()` 1) except `duty.type_ibis`. `bus.file()` gives
  `Vehicles/HH20_EBus2021/HHEBus2021_main.bus`; `bus.triggers()` 364 names - **in lower case**
  (`almex_clicku6`), while the model files say `almex_clickU6`; `press`, `release` and `trigger`
  take either. A NaN set into a variable comes back as NaN from `var` and `get_vars` (the
  plugin sends none for it). `vars("str")` 71 names, `vars()` 1736. `world.date()` a table,
  `world.time()` seconds. `map.lane()` names the spline file on Grundorf (`invis_street.sli`),
  which the server already drops. `every(0.1)` fired 640 times in 63.9 s of game and 63.8 s of
  real time: ten a second.
- **`omsi.trigger` presses and lets go in one frame**: the ALMEX's keys set a variable that its
  script reads in its frame (`almex_clickU6`), so a trigger did nothing; `press`, then `release`
  later, works. The plugin now taps that way (held 0.15 s of game time) and compares trigger
  names without case; before, every tap of a real bus was `unknown_key`.
- **The HH20's ALMEX** (point 2, its first bus): the app's drawing compared with screenshots of
  the cab at menus 0, 2, 6 and 24 - the same pages, texts, top line (`LEE`, `LEER`, `--:--`,
  the clock) and keys; the dark column at the left is in the page pictures themselves. Taps in
  the app moved the real ALMEX (menu 6 to 24 and back). The C2 dashboard display (`LADEVORGANG`)
  matches too. The driver's note (`ZETTEL`) was black: its text lies on its own paper
  (`19_zettel.o3d`) and on the larger timetable sheet in the same clip (`19_schedule.o3d`,
  hidden); the vote now prefers, of as many votes, the surface shown under the same `[visible]`
  as its anchors (`crates/hub-devices/tests/note.rs`).
- **Only the IBIS and the ticket printer go to the app** (Luc's decision: the bus's other
  screens - the passengers' display, the dashboard's, a thermometer, the HH20's door bar and its
  note - only confused). `hub-companion` drops role `other` before anything is built, and any
  device with nothing to press; the app has no list of other devices any more (`/device` is
  gone), and of several printers it opens one with a drawn screen first.
- **The stock MAN SD80** (`Vehicles/MAN_SD200/MAN_SD80.bus`): `hub-devices` never finished its
  devices - a lone key moved to `~ibis` came back to the same line for ever (fixed,
  `modules.rs` test `a_lone_key_goes_to_the_stem_of_its_action_once`). Then its Krüger IBIS
  drew only the lit display on black: the item of its `[matl_change]` on `elec_busbar_main` has
  only a `[matl_nightmap]`, which was drawn INSTEAD of the face; it is now added to it, as the
  game lights it (`Flags::night`; the ALMEX's red delay box, black around it, looks the same).
  And it sat a few degrees askew: a face is now turned to its picture's own axes when they lie
  within 20 degrees (`tests/turn.rs`). Taps in the app typed `LINIE/KURS: 29` in the game.
- **The Setra O550** (`Vehicles/TH_Ueberlandbus/O550_Euro3.bus`): the AFR 200 is its display
  (empty until signed on, as in the cab) with its 25 keys beside it; its change tray is a second
  printer of keys. The matrix panel in the roof was drawn upside down: a face lying flat has no
  up of the bus, and now takes its picture's (`tests/turn.rs`). Taps typed `Linie: 2`.
- **The installation's buses** through `hub-devices` (a sweep, 136 of its 788 `.bus` files): no hang,
  no panic. But the Citybus 530 (Kajosoft) took 30 to 90 s each in a release build, measured
  before the filter above (it had nine devices then); worth measuring again, and making faster
  if it still is: the app waits that long for its first form.

**Not proven - the work for this session, in this order:**

1. ~~**The plugin on the real openOMSI.**~~ Done, above.
2. **Real buses' forms.** The HH20's ALMEX, the MAN SD80's IBIS and the Setra O550's AFR 200
   and matrix panel are done (above). Open: the ALMEX's menu 26 (needs a duty), the AFR 200's
   display with text (signed on), the Citybus 530's build time, a bus from a mod folder.
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

# with the real game but without the launcher: this plugin put into openOMSI and the server
cargo run -p hub-plugin-harness --example realgame -- "E:/Open-Omsi Official" "C:/Program Files (x86)/Steam/steamapps/common/OMSI 2"
# then the game itself, five minutes, no menu (the window opens on the desktop)
openomsi.exe --root "<OMSI 2>" --map maps/Grundorf/global.cfg --bus Vehicles/HH20_EBus2021/HHEBus2021_main.bus --no-menu --autostart --exit-after 300
```

A cab screenshot from inside a plugin: `omsi.game.screenshot()` (the file lands in openOMSI's
`Screenshots`); or one frame without a window, looking about the cab:
`openomsi.exe --root "<OMSI 2>" --bus <bus> --autostart --offscreen cab.png --look 35,-20`. `realgame` keeps its pairings in the temp folder (`hub-realgame-devices.json`)
and its forms in `%TEMP%\companion-forms`: delete that after changing `hub-devices`.

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
