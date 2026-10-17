# What changed

Each version of the Omsi-Hub add-on, newest first. The release on GitHub links here.

## Not released yet

- **openOMSI's own settings in Omsi-Hub** (Settings → openOMSI's settings): the graphics, the
  screen and the navigator, the game's language, driving and controls, and the passengers, as
  openOMSI keeps them in `~/.openomsi/settings.cfg`. Omsi-Hub changes only these keys and leaves
  the rest of the file as it is; "openOMSI's default" takes a key out again. Not while the game
  runs.

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

### Not in it yet

- **Playing together** (it needs a relay server), **the line editor** (it needs the map's lanes),
  **the livery studio** and **the 3D preview**: their pages say so.
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
