# What changed

Each version of the Omsi-Hub add-on, newest first. The release on GitHub links here.

## 0.1.0 (pre-release, for testing)

The first Omsi-Hub that is an add-on: its own app beside the openOMSI you have installed, instead
of a game of its own. It needs **openOMSI 0.2.22 or newer** and an original **OMSI 2**, and it
never changes their files.

### In it

- **The first start** asks for your openOMSI folder and your OMSI 2 folder (it suggests the ones it
  finds, on Steam too) and checks openOMSI's version. Before the first drive, Omsi-Hub puts its
  plugin into openOMSI (Settings, "openOMSI and OMSI 2", switches it off or removes it).
- **Your old saves** from the Omsi-Hub that came with its own game are offered on the first start:
  shown first, then copied (your companies, careers and settings). The old folder is left as it was.
- **Four ways to drive**: a work shift built from the timetable's real trips, a tour, a free drive,
  and the bus company. Omsi-Hub starts openOMSI with the map, the bus and the duty; the plugin
  shows the duty in the game and sends the trips back when you are done.
- **The bus company and RosterFlow**: buses, drivers, lines, the days' plans and the rosters, with
  control-room messages and instructions carried out in the game.
- **The editors**: the timetable editor (a map's lines and tours) and the depot editor (depot files
  of your own, for the IBIS and the destination displays). What they write goes into openOMSI's
  content folder or Omsi-Hub's own data folder, never into the OMSI 2 folder.
- **Online, optional**: an account (email or Google) for friends, the leaderboard and up to two
  cloud saves. Without one, everything works offline.
- **Windows, macOS (Apple silicon and Intel) and Linux** (an AppImage, or a `.deb` / `.rpm`).
- **English and six translations**: Dutch, German, French, Russian, Ukrainian and Polish, as the old
  Omsi-Hub had them.
- **openOMSI's own settings in Omsi-Hub** (Settings → openOMSI's settings): every key of
  `~/.openomsi/settings.cfg` that openOMSI's user guide names, 62 of them: the graphics (with
  Enhanced and Enhanced+, the view distance, the video memory, LED displays, rain and puddles),
  the mirrors, the screen and the navigator, the game's language, sound and the radio, driving and
  controls, and the passengers and the traffic. Omsi-Hub leaves the rest of the file as it is;
  "openOMSI's default" takes a key out again. Not while the game runs.
- **The installers in Omsi-Hub's look**: on Windows a welcome and a finish page with the logo, a
  picture beside every page's title, the licence, a choice of language (English, Dutch, German,
  French, Russian, Ukrainian) and a folder of its own in the Start menu; on macOS a disk image
  with its own window to drag Omsi-Hub into Applications; on Linux the packages' description.
- **The maps are drawn**: the route map of the drive's steps, the company's duty map and its fleet
  map show the map's roads and the way each trip drives, read from the map's own tiles and spline
  files. A junction's own lanes are not read yet: a route crosses it in a straight line.
- **The bus in 3D** on the bus step, for every bus openOMSI drives: openOMSI itself writes the bus
  (`--export-glb`) in the livery and with the bus options chosen, and Omsi-Hub draws it - turned
  with the mouse, zoomed with the wheel, turning by itself, with views from the front, the side
  and the back. The first time a look takes a while; then it is kept. Needs an openOMSI that has
  `--export-glb`.
- **The buses' photos** on the fleet's, the dealer's and Casual's tiles and on the bus step: a bus
  without one is photographed from the model openOMSI writes for the 3D view, one bus after the
  other, while its tile is shown, and the photo is kept. The photos the old Omsi-Hub took are
  still found.
- **The game's buses and traffic on the fleet map**: while you drive on the company's map, your
  bus and every bus of the company's tours the game drives are where the game has them, with the
  game's delay; the game's cars, trams and bicycles, the buses of the lines the company does not
  run and the other players' buses in a LAN game are drawn too (a switch in the panel hides the
  traffic). Needs an openOMSI with `omsi.traffic.list` and `omsi.timetable.buses` (0.2.22); the
  launcher shows whether yours has them.

### Not in it yet

- **Playing together** (it needs a relay server), **the line editor** (it needs the map's lanes)
  and **the livery studio**: their pages say so.
- **What needs a newer openOMSI plugin API** than the official releases have: the bus's own display
  font, the IBIS typed in for you, closed stops struck through on the navigator, and the duty board
  docked in the navigator. The launcher shows which ones your openOMSI supports.

### Good to know

- **Updates.** This pre-release does not install updates itself yet: when a newer Omsi-Hub is out,
  the app says so and opens its page to download it.
- **macOS.** The app is not notarised: the first time, right-click it and choose Open.
- **Something wrong?** Tell us at [info@omsihub.com](mailto:info@omsihub.com) or in an issue, with
  the log: `omsi-hub.log` in Omsi-Hub's data folder (`%APPDATA%\Omsi-Hub` on Windows,
  `~/Library/Application Support/Omsi-Hub` on macOS, `~/.local/share/Omsi-Hub` on Linux).
