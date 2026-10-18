# What changed

Each version of the Omsi-Hub add-on, newest first. The release on GitHub links here.

## 0.1.0 (pre-release, for testing)

The first Omsi-Hub that is an add-on: its own app beside the openOMSI you have installed, instead
of a game of its own. It needs **openOMSI 0.2.22 or newer** and an original **OMSI 2**, and it
never changes their files.

### In it

- **The welcome** on the first start, as in the old Omsi-Hub: the language, the animations, your
  openOMSI folder and your OMSI 2 folder (it suggests the ones it finds, on Steam too, and checks
  openOMSI's version), your **account** - sign in, or continue locally without sync -, your driver
  and the way on (the tour, or straight to the start). Before the first drive, Omsi-Hub puts its
  plugin into openOMSI (Settings, "openOMSI and OMSI 2", switches it off or removes it).
- **The leaderboard gets your figures**: signed in, Omsi-Hub sends your driver's points, level,
  duties, time driven and kilometres whenever they change (after every drive).
- **One account, one driver**: signed in, your driver is your account's (the display name's), with
  the friends, the leaderboard and the cloud saves. Other drivers on the same computer play locally.
- **Your old saves** from the Omsi-Hub that came with its own game are offered on the first start:
  shown first, then copied (your companies, careers and settings). The old folder is left as it was.
- **The bus stop signs** on the maps, as you choose them in the welcome or the settings: the
  German H (the default), the British roundel, the French or the Dutch sign. The route map of the
  drive's steps is the old one again: its stops with their times, dragged to move, the wheel or
  the + and - to zoom.
- **Blue** is Omsi-Hub's colour unless you choose another (Settings, Accent colour).
- **The drivers on this computer**, as in the old Omsi-Hub: "Who is driving today?" is the drive's
  first step again, with a tile for each driver (OMSI's own personnel files, `Drivers/<name>.odr`),
  a new one made in place and one deleted after asking. The game is started with the driver
  chosen (`--driver`), so it keeps their figures; the **service record** shows them with the
  level, the experience and the recent runs openOMSI wrote down.
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
- **The maps are drawn**, as the old Omsi-Hub drew them: the route map of the drive's steps, the
  company's duty map and its fleet map show the roads the game's own city map draws, read from the
  map's tiles (its splines' and its junctions' lanes), and the way each trip drives on its own
  lanes, with its stops.
- **Depot files read right**: a destination's strings were read a place off (a line OMSI's files
  do not have was taken for the stop it ends at), so its IBIS text was lost.
- **The bus in 3D** on the bus step, as the old Omsi-Hub drew it: the page's own 3D view reads
  every bus as the game reads it (its meshes, its textures, the livery and the bus options chosen,
  what its scripts show at rest), turned and zoomed with the mouse, with the views from the
  front, the side, the back and the corner, and the driver's and a passenger's view.
- **The buses' photos** on the fleet's, the dealer's and Casual's tiles and on the bus step, taken
  by the same 3D view (transparent, from the front corner) and kept until the bus changes. The
  photos the old Omsi-Hub took are still shown until then.
- **The livery studio**, as in the old Omsi-Hub: paint a bus in layers on the bus itself -
  colour areas, stripes, texts, pictures, own shapes and the brush, mirrored onto the other side -
  and save it into the game as a livery of its own (openOMSI's content folder, never the OMSI 2
  folder). The liveries begun before are opened from the same folder.
- **The line editor**, as in the old Omsi-Hub: a line of your own on any map - click its stops
  on the map and the way between them is found over the map's own lanes (the kerb side, lane
  changes), the way back made from the stops across the road, a leg dragged through a point;
  its destinations, its timetable and its kind of service; saved into the map's timetable so
  you and the timetable's buses drive it. For the bus company: its map, its lines, the estimate
  and "Confirm and pay". A line of the map's own opens to change it.
- **Multiplayer**, as in the old Omsi-Hub (on the start, "Multiplayer"): host your next duty
  and give friends its session code (at home and over the internet), join a friend's session by
  their code, or join a server that is always on - openOMSI's official one is listed first, and
  you can add your own. The game opens and joins the session itself (openOMSI's `--lan-host` and
  `--lan-join`), with the map, time and weather of the host; your driver's name is the one the
  others see.
- **Playing together**, as in the old Omsi-Hub: the boss shares the open company with a session
  code, friends join it in their own Omsi-Hub with the role the boss gives them and drive their
  duties in their own game. Over the LAN or a VPN, and over the internet through a Cloudflare
  tunnel whose address goes over openOMSI's own relay (no port forwarding needed).
- **The game's buses and traffic on the fleet map**: while you drive on the company's map, your
  bus and every bus of the company's tours the game drives are where the game has them, with the
  game's delay; the game's cars, trams and bicycles, the buses of the lines the company does not
  run and the other players' buses in a LAN game are drawn too (a switch in the panel hides the
  traffic). Needs an openOMSI with `omsi.traffic.list` and `omsi.timetable.buses` (0.2.22); the
  launcher shows whether yours has them.

### Not in it yet

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
