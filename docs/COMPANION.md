# The phone and tablet companion (API 1)

How the add-on talks to the **Omsi-Hub Companion** app (iPhone, iPad, Android; its own
repository, `Omsi-Hub/Omsi-hub-companion-app`, keeps the same text as `docs/PROTOCOL.md`). The
add-on is the server, on the player's computer; the app is a client on the same home network.

```
 openOMSI ── plugin (Lua) ──UDP 127.0.0.1:47812──▶ Omsi-Hub add-on ◀──HTTP :47811── Companion app
                 ▲                                       │
                 └──── data/companion.json (commands) ───┘
```

* The **plugin** (`plugin/companion.lua`) reads the game twice a second and sends it to the
  add-on as small JSON datagrams (`omsi.send`, at most 8 KB each). It reads the commands
  the add-on writes into its data folder (`data/companion.json`) four times a second and carries
  them out in the game (IBIS keys, the ticket printer, signing on, taking a duty).
* The **add-on** (`crates/hub-companion`, run by `crates/hub-api/src/companion.rs`) keeps the
  latest state, adds what only it knows (the company's duty, the driver's pass, the control
  room's messages, the map's roads and the trip's route, the next turn) and serves it to the
  paired devices.
* The **app** shows it and sends commands.

Everything is JSON, UTF-8, `snake_case`. Times of day are **seconds since midnight** of the game's
clock (`30240` = 08:24:00); durations are seconds; distances metres; map coordinates are the
game's (x east, y north).

## Finding and pairing

The server is off by default. The player turns it on in the add-on under
**Settings › General › Phone & tablet**; the launcher then shows the address, a six-digit
**pairing code** and a QR code. The QR code holds

```
omsihub://pair?host=192.168.1.20&port=47811&code=482913&name=Luc-PC
```

(the app also accepts `http://192.168.1.20:47811/pair?code=482913`). Default port **47811**.

| Request | Auth | Answer |
| --- | --- | --- |
| `GET /api/hello` | none | `{app: "omsi-hub", api: 1, version: "0.1.0", name: "Luc-PC", pairing: true}` |
| `POST /api/pair` `{code, device}` | none | `200 {key, name}`; `403 {error: "wrong_code"}`; `429 {error: "too_many"}` (ten tries a minute) |

`device` is the phone's own name ("Luc's iPhone"), shown in the launcher's list of paired
devices, where each can be removed. `key` is 32 hex characters; the app keeps it (secure
storage) and sends it on every other request:

```
Authorization: Bearer <key>
```

A request without a valid key gets `401 {error: "not_paired"}` (the app then pairs again).

## State

`GET /api/state?after=<v>` is a **long poll**: answered at once when the server's state is newer
than `v`, else as soon as it changes, else after 20 s with the same state. The app asks again at
once after every answer (and backs off 2 s → 10 s after errors). `after` omitted: at once.

```jsonc
{
  "v": 1842,                       // version of this state; send it back as ?after=
  "game": true,                    // the plugin was heard in the last 5 s
  "clock": 30240.5,                // the game's time of day
  "paused": false,
  "signon": {                      // signing on (the driver's pass is the add-on's)
    "stage": "signon",             // "signon" | "order" | "work" | "none" (no sign-on needed)
    "number_hint": "48…",          // never the full number or the code
    "error": null                  // "wrong" after a wrong number/code
  },
  "duty": {                        // null: no duty at all
    "kind": "company",             // "company" | "shift" | "tour" | "free" | "game"
    "name": "Duty 14",
    "line": "M29", "tour": "3",
    "start": 30120, "end": 52200,
    "depot": "Bismarckplatz",
    "company": "Stadtbus",         // the company's name ("" on a drive of the player's own)
    "started": true,               // the game drives the duty (false: assigned, not taken yet)
    "trips": [
      { "n": 1, "line": "M29", "destination": "Hauptbahnhof",
        "departure": 30240, "arrival": 44640, "state": "done" }  // "done" | "now" | "next" | "later"
    ]
  },
  "trip": {                        // the trip being driven; null between trips / without duty
    "n": 3, "of": 13,
    "line": "M29", "destination": "Markgraf-Berthold-Platz",
    "departure": 30240, "arrival": 44040,
    "delay": 120,                  // seconds late, early negative
    "next": 4,                     // index into stops of the next stop
    "at_stop": false,              // the bus stands at the next stop
    "stops": [
      { "name": "Hauptbahnhof", "arrival": 30240, "departure": 30240,
        "delay": 120, "passed": false, "calls": true, "x": 1203.5, "y": -88.2 }
    ]
  },
  "bus": {                         // null on foot
    "name": "MAN Lion's City", "number": "2604",
    "x": 1180.2, "y": -64.0, "heading": 92.0, "speed": 32.5,
    "doors_open": false, "stop_requested": true, "passengers": 23
  },
  "nav": {                         // null without a route
    "maneuver": "left",            // "straight" | "slight_left" | "left" | "sharp_left" |
                                   // "slight_right" | "right" | "sharp_right" | "uturn" |
                                   // "roundabout" | "arrive"
    "distance": 200,               // metres to it
    "street": "Rathausstraße",     // the road after it, where the map names it; else ""
    "toward": "Kunsthalle",        // the next stop (always set)
    "route_v": 7                   // GET /api/route again when this changes
  },
  "pause": {                       // a break between trips; null when none
    "active": true,
    "since": 29500, "until": 29960,
    "place": "Bismarckplatz",
    "next": { "departure": 30120, "line": "307", "destination": "Hauptbahnhof" },
    "auto": true                   // started by itself (the bus stood at the terminus)
  },
  "messages": {
    "unread": 2,
    "items": [
      { "id": 77, "time": "12:58", "from": "Control centre",
        "priority": "urgent",      // "info" | "instruction" | "urgent"
        "text": "Diversion Kunsthalle: use Ringstraße until 10:00",
        "state": "delivered",      // "delivered" | "read" | "accepted" | "declined" | "sent"
        "state_label": "Delivered",
        "mine": false,
        "answers": [ { "key": "read", "label": "Acknowledge" } ],
        "action": null,            // an instruction's words ("Short-turn at Rathaus"); null: none
        "reply": "" }
    ],
    "quick": [ { "key": "late", "label": "Running late" } ]  // quick buttons to control
  },
  "ibis": {                        // the bus's IBIS; null when the bus has none
    "kind": "stock",               // "stock" (the stock MAN/Krüger IBIS keys) | "generic"
    "line": 29, "suffix": 0, "kurs": 3, "route": 1,
    "ziel": 145, "terminus": "Markgraf-Berthold-Platz",
    "stop": "Kunsthalle", "stop_index": 4,
    "mode": 0,                     // IBIS_mode as the bus's script has it
    "entry": "",                   // what is typed, where the bus shows it
    "display": ["29/03  145", "Markgraf-Berthold-Pl."], // the display's lines, where readable
    "keys": ["IBIS_0", "IBIS_1", "…", "IBIS_eingabe", "IBIS_loeschen"]
  },
  "printer": {                     // the ticket printer; null when the bus sells no tickets
    "tickets": [ { "i": 1, "name": "Single", "price": 2.9, "day": false } ],
    "request": { "name": "Single", "price": 2.9 },  // the passenger at the desk; null: nobody
    "sales": { "count": 14, "money": 40.6 },
    "currency": "€",
    "keys": ["ticketprinter_button_ticket_1", "…"],
    "change_keys": []              // the change dispenser's triggers, where the bus has one
  },
  "features": { "ibis": true, "printer": true, "nav": true, "messages": true, "signon": true }
}
```

Every key may be missing or `null` where the game, the bus or the add-on does not know it; the
app shows what it has and hides the rest.

## Map

| Request | Answer |
| --- | --- |
| `GET /api/route` | the trip's route: `{v, pts: [[x, y], …], stops: [{name, x, y}]}` (`v` = `nav.route_v`) |
| `GET /api/roads?x=&y=&r=` | the roads within `r` m (≤ 2000) of a point: `{roads: [{w, pts: [[x, y], …]}], water: [], green: []}` |
| `GET /api/lines` | the map's lines and tours, for `take_duty`: `{lines: [{name, tours: [{number, today}]}]}` |

The app fetches roads in tiles of 1000 m and keeps them; `w` is the road's width in metres.

## Commands

`POST /api/do` with one command; answered when the game has done it (at most 4 s):
`200 {ok: true}` or `200 {ok: false, error: "…"}`; `400` for a command the server does not know.

| `do` | Fields | What happens |
| --- | --- | --- |
| `sign_on` | `number`, `code` | Signs the driver on (the pass the add-on knows); ten tries a minute |
| `accept_order` | | Signs the duty order: the duty starts, the IBIS is typed |
| `drive_free` | | Drive without a duty (the plugin gives the duty up, the bus stays) |
| `take_duty` | `line`, `tour`, `trip?` | Takes a duty of the map's timetable (`omsi.duty.start`) |
| `type_ibis` | | Types the duty into the IBIS (`omsi.duty.type_ibis`) |
| `ibis_key` | `key` | Presses one of `ibis.keys` (`omsi.trigger`) |
| `ibis_line` | `line` | Types a line into the IBIS (`omsi.bus.set_line`) |
| `printer_key` | `key` | Presses one of `printer.keys` or `printer.change_keys` |
| `ticket` | `i` | Sells ticket `i` of `printer.tickets` (its key, else the ticket key `ticket_give`) |
| `msg_read` | `id` | Marks a message read |
| `msg_answer` | `id`, `key` | Answers a message with one of its `answers` (`read`, `accept`, `decline` or a canned reply's key) |
| `msg_quick` | `key` | A quick button to control |
| `pause_end` | | Ends the break early |

Only the keys the state lists can be pressed: the plugin refuses any other trigger.

The errors a command can answer: `game_not_running` (the plugin was not heard for 5 s),
`no_plugin`, `timeout` (the game did not ack it within 4 s), `unknown_key`, `too_many` (ten
`sign_on` a minute), `wrong` (not the driver's number and code), `not_now` (no duty order to
sign), `no_message`, `no_printer`, `not_supported` (this openOMSI lacks the function), or the
game's own reason a duty could not be taken.

## The plugin's datagrams (add-on internal)

To `127.0.0.1:47812` (`companion_udp_port`), each one JSON object with `t`:

| `t` | Every | Holds |
| --- | --- | --- |
| `live` | 0.5 s | `clock`, `paused`, `bus`, `duty` (line, tour, trip n, delay, next stop index, at_stop, trip_done), the plugin's `signon` stage |
| `trip` | on change | the trip's stops (paged: `page`, `pages`, ≤ 25 stops a page) |
| `trips` | on change | the duty's trips |
| `dev` | 0.5 s | the IBIS and printer variables and strings, `ticket_request`, `sales` |
| `keys` | 5 s | the bus's triggers that belong to the IBIS, the printer and the change dispenser, and the map's tickets |
| `lines` | 10 s | the map's lines and tours (paged) |
| `ack` | per command | `seq`, `ok`, `error` |

And the add-on writes `data/companion.json` in the plugin's data folder:
`{sid, port, seq: 12, cmds: [{seq: 11, do: "key", key: "IBIS_7"}, …], probe: [x, y]}` (the
last 20 commands of the server's session `sid`, the port for the datagrams, and the place whose
street name the plugin reports in `live.probe`); the plugin carries out each `seq` of the session
it has not done yet and acks it. Commands already in the file when the plugin starts count as
done. The plugin talks to the companion only while the file is there.
