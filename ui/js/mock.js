// The browser mock of hub-api: what the app's Rust answers, faked, so the page can be opened
// in a plain browser (ui/index.html, or `?mock`). A port adds the answers its page needs to
// `answers` below (or a module of its own under ./mock/, merged here). The first-run setup
// accepts any folder whose name contains "openOMSI" (a version in the name, as
// `openOMSI-0.2.30`) and "OMSI 2"; what it keeps stays in this browser (`localStorage`), so a
// reload opens the start. `?intro=off` leaves the opening out, `?page=<page>` opens a page
// (`settings`, `bus company`, `drive:bus` … as `OMSIHUB_PAGE` would).

import { companyAnswers } from './mock/company.js';
import { rosterflowAnswers } from './mock/rosterflow.js';

const listeners = {};
const params = new URLSearchParams(location.search);
const KEPT = 'omsihub.mock.settings';

function kept() {
  try {
    return JSON.parse(localStorage.getItem(KEPT)) || {};
  } catch {
    return {};
  }
}
const settings = { openomsi_folder: '', omsi2_folder: '', language: '', setup_done: false, migrated: false, ...kept() };
const keep = () => {
  try {
    localStorage.setItem(KEPT, JSON.stringify(settings));
  } catch {
    // (a private window: this session only)
  }
};

function versionIn(folder) {
  const m = /(\d+)\.(\d+)(?:\.(\d+))?/.exec(folder);
  return m ? [Number(m[1]), Number(m[2]), Number(m[3] || 0)] : null;
}

function older(a, b) {
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] < b[i];
  return false;
}

function checkOpenomsi(folder) {
  if (!/openomsi/i.test(folder || '')) return { ok: false, problem: 'no_game', text: 'No openOMSI was found in this folder' };
  const v = versionIn(folder);
  const tooOld = !!v && older(v, [0, 2, 22]);
  return {
    ok: !tooOld,
    too_old: tooOld,
    version: v ? v.join('.') : null,
    version_from: v ? 'folder_name' : null,
    plugins_dir: folder + '/Plugins',
    text: tooOld ? `This openOMSI is ${v.join('.')}: Omsi-Hub needs 0.2.22 or newer. Update openOMSI.` : '',
    compat: { level: tooOld ? 'unsupported' : v ? 'supported' : 'unknown', features: [], notes: [] },
  };
}

/** The plugin's state in the mock's openOMSI. */
let addonPlugin = 'missing';

/** The old Omsi-Hub's saves the mock finds with `?migrate` (`migrate::preview`'s shape). */
const OLD = { folder: 'C:/Users/Player/.openomsi', companies: [['stadtbus', 'Stadtbus Grundorf']], careers: ['luc'], lines: ['Grundorf'], depots: ['oo_Grundorf'], settings: ['accent', 'dark_mode', 'company_layout'], language: 'de', driver: 'Luc', files: 11, bytes: 6000 };
const oldSaves = () => (params.has('migrate') && !settings.migrated ? { offer: true, folder: OLD.folder, companies: OLD.companies.map((c) => c[0]), careers: OLD.careers, files: OLD.files, preview: OLD } : { offer: false, companies: [], careers: [], files: 0 });

function checkOmsi2(folder) {
  const ok = /omsi ?2/i.test(folder || '');
  return { ok, missing: ok ? [] : ['Omsi.exe'], text: ok ? '' : 'This is not the OMSI 2 folder: Omsi.exe not found' };
}

// --- the drive's content: two maps, three buses, Grundorf's lines --------------------------

const MAPS = [
  { file: 'maps/Grundorf/global.cfg', name: 'Grundorf', friendly: 'Grundorf', entry_points: [{ name: 'Depot' }, { name: 'Bahnhof' }, { name: 'Marktplatz' }] },
  { file: 'maps/Ruhrau/global.cfg', name: 'Ruhrau', friendly: 'Ruhrau', entry_points: [{ name: 'Betriebshof' }] },
];

const BUSES = [
  { file: 'Vehicles/MAN_SD200/SD202.bus', name: 'MAN SD202', manufacturer: 'MAN', folder: 'MAN_SD200', type_name: 'Double-decker', description: 'Berlin double-decker, 1980s', paints: ['BVG'], default_paint: 'BVG', numbers: [['2571', 'B-V 2571']], missing_packs: [], hofs: ['Grundorf.hof'] },
  { file: 'Vehicles/MAN_NL_NG/NL202.bus', name: 'MAN NL202', manufacturer: 'MAN', folder: 'MAN_NL_NG', type_name: 'Low-floor', description: 'City bus, low floor', paints: ['Weiss', 'Grundorf'], default_paint: 'Weiss', numbers: [['17', '']], missing_packs: [], hofs: ['Grundorf.hof'] },
  { file: 'Vehicles/MAN_NG/NG272.bus', name: 'MAN NG272', manufacturer: 'MAN', folder: 'MAN_NG', type_name: 'Articulated', description: 'Articulated city bus', paints: ['Weiss'], default_paint: 'Weiss', numbers: [], missing_packs: [], hofs: [] },
];

const STOPS = ['Bahnhof', 'Marktplatz', 'Kirche', 'Schule', 'Waldweg', 'Endstation'];
const stopsFrom = (dep, back) => (back ? [...STOPS].reverse() : STOPS).map((name, k) => ({ name, id: 100 + (back ? STOPS.length - 1 - k : k), arr: dep + k * 180, dep: dep + k * 180 + (k ? 20 : 0) }));
function trip(line, index, dep, back) {
  const stops = stopsFrom(dep, back);
  return { name: `${line} ${back ? 'Endstation-Bahnhof' : 'Bahnhof-Endstation'}`, index, line, from: stops[0].name, terminus: stops[stops.length - 1].name, departure: dep, arrival: stops[stops.length - 1].arr, stops, km: 6.4 };
}
function tour(line, number, start, n) {
  const trips = Array.from({ length: n }, (_, k) => trip(line, k + 1, start + k * 1500, k % 2 === 1));
  return { number, ai_group: 'Stadtbus', first: trips[0].departure, last: trips[n - 1].arrival, days: 'Mon-Fri', runs: true, next_run: null, trips };
}
const LINES = [
  { name: '1', user_allowed: true, termini: ['Bahnhof', 'Endstation'], tours: [tour('1', '1', 5 * 3600 + 600, 12), tour('1', '2', 6 * 3600, 10)] },
  { name: '2', user_allowed: true, termini: ['Endstation', 'Bahnhof'], tours: [tour('2', '1', 7 * 3600 + 900, 8)] },
  { name: 'E', user_allowed: false, termini: ['Bahnhof', 'Schule'], tours: [tour('E', '1', 7 * 3600, 2)] },
];

// --- the editors: a timetable to edit and a depot file of the player's ---------------------

const ttLine = (l) => ({ path: `maps/Grundorf/TTData/${l.name}.ttl`, name: l.name, user_allowed: l.user_allowed, priority: 1, own: false, tours: l.tours.map((t) => ({ number: t.number, ai_group: t.ai_group, days: 31, trips: t.trips.map((x) => ({ trip: x.name, profile: 0, departure: x.departure / 60 })) })) });
const TT = { dir: 'maps/Grundorf', folder: 'Grundorf', lines: LINES.map(ttLine), trips: LINES.flatMap((l) => [0, 1].map((k) => ({ name: l.tours[0].trips[k].name, line: l.name, terminus: l.tours[0].trips[k].terminus, profiles: ['0', '1'] }))) };

const DEPOT = {
  key: 'grundorf-eigen', file: 'grundorf-eigen.hof', name: 'Grundorf (own)', map: 'maps/Grundorf/global.cfg', dirty: false,
  termini: [
    { code: 1, name: 'Bahnhof', all_exit: false, strings: ['Bahnhof', 'Bahnhof', '', 'Bahnhof', '1 Bahnhof', '', '', '', '', '', '', ''], special: false, marked: false },
    { code: 2, name: 'Endstation', all_exit: false, strings: ['Endstation', 'Endstation', 'über Marktplatz', 'Endstation', '1 Endstation', '', '', '', '', '', '', ''], special: false, marked: false },
    { code: 999, name: 'Betriebsfahrt', all_exit: true, strings: ['Betriebsfahrt', 'Betriebsfahrt', '', '', '', '', '', '', '', '', '', ''], special: true, marked: true },
  ],
  stops: STOPS.map((s, k) => ({ ident: `GD_${k}`, strings: [s, s.toUpperCase()] })),
  trips: [{ code: 101, line: '1', name: 'Bahnhof - Endstation', route: 1, stops: STOPS.map((_, k) => `GD_${k}`) }],
  service_trip: 'Betriebsfahrt',
  notes: [], stop_notes: [], global_strings: ['Grundorf'], string_count: 12,
  labels: ['ibis', 'front_top', 'front_bottom', 'side', 'roller_blind', 'clear_name', 'extra_sign', 'bitmap', 'special', 'rear', 'other', 'other'].map((role) => ({ role, note: '', limit: role === 'ibis' ? 16 : 24 })),
  blocks: [], issues: [],
};
const DEPOTS = [{ key: DEPOT.key, name: DEPOT.name, map: DEPOT.map, termini: DEPOT.termini.length }];

const answers = {
  hub_version: () => ({ app: '0.1.0', min_openomsi: '0.2.22', plugin: '1.7.0', bridge: 1 }),
  hub_boot: () => ({ version: answers.hub_version(), data_dir: '(mock)', settings, setup_needed: !settings.setup_done, languages: ['en', 'nl', 'de', 'fr', 'pl'], intro: params.get('intro') || '', page: params.get('page') || '' }),
  hub_texts: () => ({}),
  // (`?migrate` offers the old saves once, as `hub_setup_state` does when ~/.openomsi has some)
  hub_setup_state: () => ({ settings, openomsi: null, omsi2: null, migrate: oldSaves() }),
  hub_setup_suggest: () => ({ openomsi: [], omsi2: [] }),
  hub_setup_openomsi: (a) => checkOpenomsi(a.folder),
  hub_setup_omsi2: (a) => checkOmsi2(a.folder),
  hub_setup_save: (a) => {
    const o = checkOpenomsi(a.openomsi);
    if (!o.ok) throw o.text;
    const r = checkOmsi2(a.omsi2);
    if (!r.ok) throw r.text;
    Object.assign(settings, { openomsi_folder: a.openomsi, omsi2_folder: a.omsi2, setup_done: true });
    if (a.language) settings.language = a.language;
    keep();
    return { ok: true, settings, openomsi: o };
  },
  hub_settings_patch: (a) => {
    Object.assign(settings, a.patch || {});
    keep();
    return settings;
  },
  hub_migrate_skip: () => ((settings.migrated = true), keep(), { ok: true }),
  hub_migrate_scan: () => ({ folder: OLD.folder, items: [], companies: OLD.companies.map((c) => c[0]), careers: OLD.careers, preview: OLD }),
  hub_migrate_import: () => ((settings.migrated = true), keep(), { imported: OLD.files, preview: OLD, settings }),
  hub_game_state: () => ({ running: false }),
  hub_log: () => null,
  // (the online account and the cloud saves, signed out: everything works offline)
  ol_state: () => ({ mode: 'out', friends: [], invites: [], requests: [], badge: 0, online_count: 0, offline: false, together: false, hosting: false, privacy: { version: 2, needed: false } }),
  ol_refresh: () => ({ ok: true }),
  ol_seen: () => ({ ok: true }),
  ol_presence: () => ({ ok: true }),
  cs_state: () => ({ companies: {}, slots: 2, used: 0, local_label: 'Local save', cloud_label: 'Cloud save' }),
  cs_open: () => ({}),
  addon_state: () => ({ folder: settings.openomsi_folder, game: checkOpenomsi(settings.openomsi_folder), plugin: { state: addonPlugin }, plugin_version: '1.7.0', hello: null, checked: false, assigned: null }),
  addon_enable: (a) => ((addonPlugin = a.on ? 'on' : 'off'), { state: addonPlugin, version: '1.7.0' }),
  addon_remove: () => ((addonPlugin = 'missing'), { state: 'missing' }),
  addon_set_folder: (a) => {
    const c = checkOpenomsi(a.folder);
    if (c.ok) (settings.openomsi_folder = a.folder), keep();
    return { folder: settings.openomsi_folder, game: checkOpenomsi(settings.openomsi_folder), plugin: { state: addonPlugin }, plugin_version: '1.7.0', checked: false, ...(c.ok ? {} : { refused: { folder: a.folder, problem: c.text } }) };
  },
  addon_version: () => ({ version: checkOpenomsi(settings.openomsi_folder).version, min: '0.2.22' }),
  addon_texts: () => ({ language: settings.language || 'en' }),
  addon_reports: () => ({ reports: 0, trips: 0, orders: 0, live: 0, drivers: [], companies: [] }),
  addon_poll: () => ({ reports: 0, trips: 0, orders: 0, live: 0, drivers: [], companies: [] }),

  // the drive
  hub_maps: () => MAPS,
  hub_buses: () => BUSES,
  hub_weather: () => [{ file: 'weather/sunny.owt', name: 'Sunny' }, { file: 'weather/rain.owt', name: 'Rain' }],
  hub_lines: (a) => (/grundorf/i.test(a.map || '') ? LINES : []),
  hub_default_hof: () => 'Grundorf.hof',
  // (the line's routes in the depot file, as `content::ibis` answers)
  hub_ibis: (a) => ({ hof: a.hof || 'Grundorf.hof', line: a.line, line_code: String(a.line || '').replace(/\D/g, '') || '1', routes: [{ code: '101', name: 'Endstation', terminus: 'Endstation' }, { code: '102', name: 'Bahnhof', terminus: 'Bahnhof' }], destinations: 3 }),
  hub_situations: () => [],
  hub_map_facts: () => ({}),
  hub_own_lines: () => [],
  hub_shifts: (a) => {
    // (two shifts of line 1's tours: from the tour's first trip, as long as asked)
    const want = (a.minutes || 120) * 60;
    return LINES[0].tours.map((t) => {
      const trips = [];
      for (const x of t.trips) if (!trips.length || x.arrival - t.trips[0].departure <= want) trips.push(x);
      const first = trips[0];
      return { key: [`1|${t.number}|${first.index}`], start: first.departure, end: trips[trips.length - 1].arrival, seconds: trips[trips.length - 1].arrival - first.departure, trips: trips.map((x) => ['1', t.number, { ...x, stops: x.stops.length }]), first: { line: '1', tour: t.number, index: first.index, departure: first.departure } };
    });
  },
  hub_weather_extras: () => ({ airport: 'EDDB', airports: [['EDDB', 'Berlin Brandenburg'], ['EDDH', 'Hamburg']], cycle: false }),
  hub_duty_extras: (a) => {
    const m = Math.max(0, (a.minutes || 0) - 10);
    return { time: `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`, date: a.date, lead: 10, display_font: null };
  },
  hub_more_setvars: () => [],
  hub_more_bus_options: () => ({ options: [], reading: false }),
  hub_more_font_view: () => ({ failed: true, options: [] }),
  // (a text on the bus's own display font, drawn by the game program: not in a browser)
  hub_display_preview: () => ({ failed: true }),
  hub_more_weather_read: (a) => ({ custom: { raw: a.weather, vis: 1200, br: 0.8, wd: 270, ws: 6.5, temp_c: -3, rh: 88, p: 1013, c: 0.75, cb: 50, precip: 2, pi: 120, wet: 0.4, snow_cover: 1, snow_on_road: 0 } }),
  hub_more_weather_write: (a) => ({ weather: a.custom?.raw || 'custom:' }),
  hub_more_bus_best: () => null,
  hub_depots: (a) => ({
    key: `${a.bus}|${a.map?.file || ''}`, bus: a.bus, map: a.map?.friendly || a.map?.name || '', want: 'Grundorf.hof', lacks: false, total: 3,
    rows: [{ key: 'Grundorf.hof', name: 'Grundorf.hof', file: 'Grundorf.hof', map: 'Grundorf', maps: true, destinations: 3, known: 3, fit: { kind: 'maps' } }],
    used: { row: 0 }, own: [], own_in_use: null, refusal: null, source: null, source_name: '', source_destinations: 0, recommended: 0,
  }),
  hub_route_map: () => ({ roads: [{ p: [0, 0, 400, 0, 400, 300, 900, 300], w: 8, main: true }, { p: [400, 0, 400, -200], w: 6 }], route: [[0, 0, 400, 0, 400, 300, 900, 300]] }),
  hub_drive: (a) => {
    setTimeout(() => backend.emit('game-ended', { code: 0, seconds: 4 }), 4000);
    return { started: true, args: ['--map', a.duty.map] };
  },
  hub_update: () => ({ kind: 'none', shown: '0.1.0', line: 'Omsi-Hub 0.1.0 is the newest.', repo: 'github.com/Luc-nbr/Omsi-Hub-Addon', auto: false }),
  hub_drive_args: (a) => ({ args: ['--root', settings.openomsi_folder, '--map', a.duty.map, '--bus', a.duty.bus, ...(a.duty.line ? ['--line', a.duty.line, '--tour', a.duty.tour || ''] : []), '--time', a.duty.time] }),
  hub_update_act: (a) => ({ kind: 'none', line: a.what === 'check' ? 'Omsi-Hub 0.1.0 is the newest.' : '', repo: 'github.com/Luc-nbr/Omsi-Hub-Addon', auto: false }),

  // the timetable editor
  tt_load: (a) => (/grundorf/i.test(a.map || '') ? TT : { dir: '', folder: '', lines: [], trips: [] }),
  tt_save: (a) => ({ saved: (a.lines || []).length, error: null, lines: (a.lines || []).map((l) => ({ name: l.name, path: l.path, line: l })) }),
  tt_edit: (a) => ({ line: a.line, tour: a.tour || 0, made: 0 }),
  tt_reset: () => ({ done: "The timetable of Grundorf is the map's own again (the edited copy was removed)", own: 0, own_error: null }),

  // the depot editor
  de_show: () => ({ entries: DEPOTS, view: DEPOT, changed_elsewhere: null, error: null }),
  de_open: () => DEPOT,
  de_edit: () => ({ view: DEPOT, sel: null }),
  de_save: () => ({ view: { ...DEPOT, dirty: false }, entries: DEPOTS, copies: 0, name: DEPOT.name }),
  de_delete: () => ({ entries: [], name: DEPOT.name }),
  de_sources: () => [
    { source: { kind: 'empty', path: null }, text: "Empty, laid out as OMSI's own", args: {} },
    { source: { kind: 'map', path: 'Vehicles/MAN_SD200/Grundorf.hof' }, text: "A copy of the map's: %{hof}", args: { hof: 'Grundorf.hof' } },
    { source: { kind: 'own', path: 'depots/grundorf-eigen.hof' }, text: 'A copy of mine: %{name}', args: { name: DEPOT.name } },
  ],
  de_make: () => ({ entries: DEPOTS, view: DEPOT }),

  ...companyAnswers,
  ...rosterflowAnswers,
};

export const backend = {
  async call(cmd, args) {
    const a = answers[cmd];
    if (!a) {
      // (kept for a look at what a page asked that the mock lacks: `__OMSIHUB_MOCK_MISSING__`)
      (globalThis.__OMSIHUB_MOCK_MISSING__ ||= {})[cmd] = (globalThis.__OMSIHUB_MOCK_MISSING__[cmd] || 0) + 1;
      throw `not ported yet: ${cmd}`;
    }
    return a(args || {});
  },
  async callBytes(cmd) {
    (globalThis.__OMSIHUB_MOCK_MISSING__ ||= {})[`bytes:${cmd}`] = 1;
    throw `not ported yet: ${cmd}`;
  },
  on(event, fn) {
    (listeners[event] ||= []).push(fn);
    return () => { listeners[event] = listeners[event].filter((f) => f !== fn); };
  },
  /** For a page's work: pretend Rust sent `event`. */
  emit(event, payload) {
    for (const f of listeners[event] || []) f(payload);
  },
};
