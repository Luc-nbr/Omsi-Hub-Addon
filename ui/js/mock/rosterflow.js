// RosterFlow's mock answers (`rf_*`): the bus company's planning with the RosterFlow add-on, as
// `omsi-launcher-api`'s `omsihub/rosterflow.rs` (and `rosterflow/{roster,timeplan,days,breaks,
// relief,tutorial}.rs`) answer it, faked so that the ported pages (`js/rosterflow/`) open in a
// plain browser. One company, "Grundorf Bus" on the map Grundorf: four buses, five drivers, the
// lines 101, 102 and 205, a service day from about 05:30 to 23:00. The day's board (blocks,
// duties, issues, figures) is worked out here from a small model - its trips by bus, where the
// duties are cut, which pieces are joined - so that the Gantt's edits (move, unassign, cut, join,
// lock, custom work, undo and redo, SAVE and DISCARD) change what is drawn. Optimise finds nothing
// better, the PDF is not made, and the Timeplan's saved timetable does not make new blocks: what
// the Rust core does there is not faked. Every text comes ready in English, as Rust says it.

// (the bus company's mock: which company is open, its day, and whether the add-on is on)
import * as companyMock from './company.js';

// --- the company ----------------------------------------------------------------------------------

const COMPANY = { id: 'grundorf-bus', name: 'Grundorf Bus', map: 'Grundorf', date: '2026-10-19' };

/** The company the bus company's mock has open (`co_company`): its id, name and day taken over.
 * Null when none is open; undefined when that mock does not say (RosterFlow then stands alone). */
function syncCompany() {
  const answers = companyMock.companyAnswers;
  if (typeof answers?.co_company !== 'function') return undefined;
  let c = null;
  try {
    c = answers.co_company() || null;
  } catch {
    c = null;
  }
  if (c) Object.assign(COMPANY, { id: c.id || COMPANY.id, name: c.name || COMPANY.name, date: /^\d{4}-\d\d-\d\d$/.test(c.date || '') ? c.date : COMPANY.date });
  return c;
}

/** Whether the company has the RosterFlow add-on on (`co::autopilot::rosterflow`): its Planning
 * page says so (`co_more_view` "planning" answers `rosterflow: true`); not in Casual. */
function addonOn(c) {
  if (c === undefined) return true;
  if (!c || c.mode === 'casual') return false;
  try {
    return !!companyMock.companyAnswers.co_more_view?.({ view: 'planning', day: 0 })?.rosterflow;
  } catch {
    return false;
  }
}
const DEPOT = 'Betriebshof Grundorf';

/** The company's lines: their stops (name, code), the minutes between them, a trip's km. */
const LINES = [
  { name: '101', number: '101', caption: 'Bahnhof – Waldfriedhof', colour: '#e03c31', a: 'Grundorf Bahnhof', b: 'Waldfriedhof', lay: 5, km: 9.8, depotKm: 2.6,
    stops: [['Grundorf Bahnhof', 1001], ['Rathaus', 1002], ['Schulzentrum', 1003], ['Kirchweg', 1004], ['Waldfriedhof', 1005]], run: [6, 7, 6, 6] },
  { name: '102', number: '102', caption: 'Marktplatz – Gewerbegebiet Nord', colour: '#1e88e5', a: 'Marktplatz', b: 'Gewerbegebiet Nord', lay: 3, km: 7.2, depotKm: 2.2,
    stops: [['Marktplatz', 1020], ['Hauptstraße', 1010], ['Industriering', 1011], ['Gewerbegebiet Nord', 1012]], run: [5, 6, 6] },
  { name: '205', number: '205', caption: 'Grundorf – Ruhrau (Schnellbus)', colour: '#43a047', a: 'Grundorf Bahnhof', b: 'Ruhrau Markt', lay: 8, km: 18.4, depotKm: 2.6,
    stops: [['Grundorf Bahnhof', 1001], ['Grundorf Nord', 2002], ['Ruhrau Gewerbepark', 2003], ['Ruhrau Kirche', 2004], ['Ruhrau Markt', 2005]], run: [6, 10, 9, 7] },
];
const lineOf = (name) => LINES.find((l) => eq(l.name, name));

// (as the bus company's mock has them: `mock/company.js`; bus 2571 is in the workshop)
const FLEET = [
  { id: 1, number: '17', name: 'MAN NL202' },
  { id: 2, number: '18', name: 'MAN NL202' },
  { id: 3, number: '31', name: 'MAN NG272' },
  { id: 4, number: '2571', name: 'MAN SD202', workshop: true },
];

const STAFF = [
  { id: 1, name: 'Jan de Vries' },
  { id: 2, name: 'Anke Müller' },
  { id: 3, name: 'Pieter Bakker' },
  { id: 4, name: 'Sabine Krüger' },
  { id: 5, name: 'Henk Janssen' },
];

/** The buses' days (`TourOfDay`): each part a shuttle from the line's first stop, its long
 * layovers (a meal break) once it is back there after the minutes in `breaks`, the duty cut at
 * the first departure from `cutAt` on. `days`: weekdays, Monday to Saturday, every day. */
const BLOCKS = [
  { line: '101', tour: '01', days: 'all', parts: [{ start: 345, end: 1305, breaks: [540, 960, 1200], cutAt: 840 }] },
  { line: '101', tour: '02', days: 'weekday', parts: [{ start: 435, end: 495, breaks: [] }, { start: 975, end: 1095, breaks: [] }] },
  { line: '102', tour: '01', days: 'sat', parts: [{ start: 360, end: 1180, breaks: [570, 1020], cutAt: 780, slow: { 420: 1 } }] },
  { line: '205', tour: '01', days: 'all', parts: [{ start: 340, end: 1380, breaks: [560, 1000, 1200], cutAt: 840 }] },
];

// --- words and numbers ----------------------------------------------------------------------------

const pad = (n) => String(n).padStart(2, '0');
const eq = (a, b) => String(a ?? '').trim().toLowerCase() === String(b ?? '').trim().toLowerCase();
const clone = (x) => JSON.parse(JSON.stringify(x));
const clock = (m) => {
  const v = ((Math.round(m) % 1440) + 1440) % 1440;
  return `${pad(Math.floor(v / 60))}:${pad(v % 60)}`;
};
const hm = (m) => {
  const v = Math.max(0, Math.round(m));
  return `${Math.floor(v / 60)}:${pad(v % 60)}`;
};
const hhmm36 = (m) => {
  const v = Math.max(0, Math.round(m));
  return `${pad(Math.floor(v / 60))}:${pad(v % 60)}`;
};
const length = (m) => `${Math.floor(m / 60)} h ${pad(Math.round(m) % 60)}`;
const minText = (m) => String(Math.round(m * 10) / 10);
const eur = (cents) => `${cents < 0 ? '-' : ''}€${Math.round(Math.abs(cents) / 100).toLocaleString('en-US')}`;
const said = (text, error = false) => ({ say: { text, error } });

const WD = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
const WEEKDAYS = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const DAY_GROUPS = ['Mon - Fri', 'Saturday', 'Sunday'];
const GROUP_WD = [0, 5, 6];

const parseDate = (d) => Date.UTC(+d.slice(0, 4), +d.slice(5, 7) - 1, +d.slice(8, 10));
const addDays = (d, k) => new Date(parseDate(d) + k * 86400000).toISOString().slice(0, 10);
const weekdayOf = (d) => (new Date(parseDate(d)).getUTCDay() + 6) % 7;
const groupOf = (wd) => (wd < 5 ? 0 : wd === 5 ? 1 : 2);
const dayLabel = (d) => `${WD[weekdayOf(d)]} ${+d.slice(8, 10)} ${MONTHS[+d.slice(5, 7) - 1]}`;
const dateOf = (day) => addDays(COMPANY.date, Math.max(0, Math.min(6, Number(day) || 0)));
const wdOf = (day) => weekdayOf(dateOf(day));

// --- the preferences (`rosterflow::Prefs::default`) -----------------------------------------------

const lim = (on, value, hard, penalty = 0) => ({ on, value, hard, penalty });
const dutyType = (name, from, to, endBy, max, split) => ({ name, start_from: from, start_to: to, end_by: endBy, max_length: max, min_length: 0, split, allowed: true, penalty: 0 });

function defaultPrefs() {
  return {
    layover: lim(true, 3, false, 2000),
    pre_trip: 5,
    post_trip: 5,
    deadhead_min: 5,
    deadhead_km: 3,
    deadheads: [],
    depot_capacity: lim(false, 20, false),
    sign_on: 10,
    sign_off: 5,
    sign_on_split: 5,
    sign_off_split: 5,
    sign_on_at: null,
    sign_off_at: null,
    relief: 'ArrivalDeparture',
    relief_before: 5,
    travel: 'Walk',
    walk: 5,
    public: 15,
    car: 10,
    points: [],
    pairs: [],
    meal: lim(true, 30, true),
    meal_after: 360,
    continuous: lim(true, 270, true),
    break_min: 15,
    break_from: 15,
    layover_breaks: true,
    max_spread: lim(true, 780, false, 5000),
    min_duty: lim(false, 240, false),
    min_piece: lim(true, 60, false, 1000),
    changeovers: lim(true, 2, false, 1500),
    split_min: 60,
    duty_types: [
      dutyType('Early shift', 180, 479, 960, 600, 'No'),
      dutyType('Middle shift', 480, 719, 1200, 600, 'No'),
      dutyType('Late shift', 720, 1080, 1620, 600, 'No'),
      dutyType('Split shift', 180, 780, 1380, 780, 'Yes'),
      dutyType('Night shift', 1081, 1439, 1800, 600, 'Either'),
    ],
    wage: null,
    driver_fixed: null,
    vehicle_fixed: null,
    per_km: null,
    optimise_vehicles: true,
    optimise_duties: true,
  };
}

let prefs = defaultPrefs();
const RELIEF = ['ArrivalDeparture', 'OnArrival', 'OnDeparture', 'MidLayover', 'BeforeDeparture'];
const RELIEF_LABELS = ['On arrival and on departure (unattended)', 'On arrival', 'On departure', 'Mid-layover', 'Before departure'];
const TRAVEL = ['Walk', 'Public', 'Car'];
const TRAVEL_LABELS = ['Walk', 'Public transport', 'Staff car'];
const TRAVEL_LETTER = ['W', 'P', 'R'];
const TRI_LABELS = ['No', 'Yes', 'Either'];
const CUSTOM_KINDS = ['Spare', 'Office', 'Union', 'Training'];
const PAID_BANDS = ['0-5', '5-7', '7-9', '9-12', '12+'];
const PALETTE = [[79, 129, 190], [196, 96, 214], [38, 166, 154], [245, 128, 40], [130, 110, 232], [250, 196, 60], [232, 82, 140], [32, 170, 210], [96, 186, 76], [222, 78, 64]].map((c) => '#' + c.map((x) => x.toString(16).padStart(2, '0')).join(''));
const MIN_PAID = 240;

/** What the company pays (`rosterflow::rates`): a driver's hour, each duty, each bus a day, each km. */
const rates = () => ({ wage: prefs.wage ?? 1850, driver_fixed: prefs.driver_fixed ?? 5550, vehicle_fixed: prefs.vehicle_fixed ?? 9000, per_km: prefs.per_km ?? 45 });

// --- what RosterFlow keeps --------------------------------------------------------------------------

/** The lines' colours as RosterFlow has them (own, or chosen in the colour window). */
const colours = Object.fromEntries(LINES.map((l) => [l.name, l.colour]));
const colourOf = (line) => colours[lineOf(line)?.name] || PALETTE[Math.max(0, LINES.findIndex((l) => eq(l.name, line))) % PALETTE.length];
let colourBy = 'line';
/** Bumped by every change: the board's revision. */
let version = 1;
let edits = 0;
/** What each weekday's planning is: saved, the draft, undo and redo, the figures before. */
const days = {};
/** Optimise's proposal, by weekday. */
const proposals = {};
const tut = { seen: true, at: 0, done: false, lessons: [] };

function dayState(wd) {
  if (!days[wd]) days[wd] = { saved: genModel(wd), draft: null, undo: [], redo: [], before: null };
  return days[wd];
}
const modelOf = (wd) => dayState(wd).draft || dayState(wd).saved;
const hasDraft = () => Object.values(days).some((d) => d.draft && JSON.stringify(d.draft) !== JSON.stringify(d.saved));

// --- the day as the timetable runs it -----------------------------------------------------------------

function mkTrip(L, tour, dep, arr, from, to, empty, out) {
  return { dep, arr, from, to, empty, line: L.name, name: empty ? '' : `${L.number}-${out ? 'A' : 'B'}`, km: empty ? L.depotKm : L.km, origin: tour, out: empty ? true : out };
}

/** A bus's part of the day: out of the depot, to and fro on its line, back to the depot. */
function shuttle(L, tour, p, cuts) {
  const run = L.run.reduce((a, b) => a + b, 0);
  const out = [mkTrip(L, tour, p.start - 15, p.start - 3, DEPOT, L.a, true, true)];
  const used = new Set();
  let t = p.start;
  let atA = true;
  let cut = p.cutAt;
  for (let guard = 0; guard < 200; guard++) {
    const extra = p.slow?.[t] || 0;
    const arr = t + run + extra;
    if (atA && cut != null && t >= cut) {
      cuts.push(t);
      cut = null;
    }
    out.push(mkTrip(L, tour, t, arr, atA ? L.a : L.b, atA ? L.b : L.a, false, atA));
    atA = !atA;
    let next = arr - extra + L.lay;
    if (atA) {
      const br = p.breaks.find((b) => arr >= b && !used.has(b));
      if (br != null) {
        for (const b of p.breaks) if (b <= arr) used.add(b);
        next = arr + 35;
      }
      if (next > p.end) break;
    }
    t = next;
  }
  const last = out[out.length - 1];
  out.push(mkTrip(L, tour, last.arr + 3, last.arr + 15, L.a, DEPOT, true, false));
  return out;
}

/** The weekday's planning as the company saved it: the buses' trips, the duties cut and joined. */
function genModel(wd) {
  const on = (d) => d === 'all' || (d === 'weekday' && wd < 5) || (d === 'sat' && wd < 6);
  const blocks = BLOCKS.filter((s) => on(s.days)).map((s) => {
    const L = lineOf(s.line);
    const cuts = [];
    const trips = [];
    s.parts.forEach((p, k) => {
      const part = shuttle(L, s.tour, p, cuts);
      if (k > 0) cuts.push(part[0].dep);
      trips.push(...part);
    });
    return { line: s.line, tour: s.tour, trips, cuts };
  });
  const joins = [];
  const peak = blocks.find((b) => b.line === '101' && b.tour === '02');
  if (peak) joins.push(piecesOf(peak).map(([a]) => pieceKey(peak, a)));
  const stack = wd < 5 ? [mkTrip(lineOf('205'), '02', 1350, 1382, 'Grundorf Bahnhof', 'Ruhrau Markt', false, true)] : [];
  return { blocks, stack, joins, blockLocks: [], dutyLocks: [], notes: {}, spares: [], dismissed: [], linked: [] };
}

/** A block's pieces of work: [first trip, end) - a new one where the duty is cut. */
function piecesOf(blk) {
  const starts = [0];
  blk.trips.forEach((x, j) => {
    if (j > 0 && blk.cuts.includes(x.dep)) starts.push(j);
  });
  return starts.map((s, k) => [s, k + 1 < starts.length ? starts[k + 1] : blk.trips.length]);
}
const pieceKey = (blk, start) => `${blk.line}|${blk.tour}|${blk.trips[start]?.dep}`;
const blockKey = (blk) => `${blk.line}|${blk.tour}`;
const blockLabel = (blk) => `${lineOf(blk.line)?.number || blk.line}/${blk.tour}`;
const tripKey = (x) => `${x.line}|${x.dep}|${x.from}`;
const isRelief = (stop) => stop === DEPOT || LINES.some((l) => eq(l.a, stop) || eq(l.b, stop));
const pointName = (stop) => (stop === DEPOT || stop === 'Depot' ? 'Depot' : stop);

// --- the board (`board_json`) -----------------------------------------------------------------------

/** The duty type a duty is (`Shaped::kind_name`): the first allowed that fits, else Other. */
function kindOf(start, end, split) {
  const types = prefs.duty_types;
  const k = types.findIndex((t) => t.allowed && start >= t.start_from && start <= t.start_to && end <= t.end_by && end - start <= t.max_length && end - start >= (t.min_length || 0) && (t.split === 'Either' || (t.split === 'Yes') === split));
  return k;
}

/** A duty's breaks, driving periods and meal break (`breaks::duty_json`), from its trips. */
function restOf(trips, splits, start) {
  const p = prefs;
  const breaks = [];
  const periods = [];
  let from = trips[0]?.dep ?? start;
  const rest = Math.max(p.break_min, p.break_from);
  for (let k = 1; k < trips.length; k++) {
    const a = trips[k - 1].arr;
    const z = trips[k].dep;
    const gap = z - a;
    if (gap < p.break_from) continue;
    const split = splits.some(([s0, s1]) => s0 < z && a < s1);
    const meal = gap >= p.meal.value;
    const resets = gap >= rest;
    const kind = split ? 'split' : meal ? 'meal' : 'break';
    const word = split ? 'Split' : meal ? 'Meal' : 'Break';
    const lines = [`${word} ${hm(gap)}  ·  ${clock(a)}–${clock(z)}`];
    if (split) lines.push('A split: unpaid, the driver signs off and on again.');
    if (p.meal.on) lines.push(meal ? `Long enough for the meal break (at least ${hm(p.meal.value)}).` : `Too short for the meal break (at least ${hm(p.meal.value)}).`);
    if (p.continuous.on) lines.push(resets ? `Ends a driving period (a break of at least ${hm(rest)}).` : `Too short to end a driving period (at least ${hm(rest)}).`);
    breaks.push({ from: a, to: z, kind, label: `${word} ${hm(gap)}`, short: hm(gap), counts: resets || meal, tip: lines.join('\n'), meal });
    if (resets) {
      periods.push([from, a]);
      from = z;
    }
  }
  if (trips.length) periods.push([from, trips[trips.length - 1].arr]);
  const work = trips.length ? trips[trips.length - 1].arr - start : 0;
  const mealOk = !p.meal.on || work <= p.meal_after || breaks.some((b) => b.meal && b.from - start <= p.meal_after);
  const meal = mealOk ? null : { from: start + p.meal_after, to: trips[trips.length - 1]?.arr ?? start, late: false, tip: `No meal break by ${clock(start + p.meal_after)}: a duty working over ${hm(p.meal_after)} needs one of ${hm(p.meal.value)} before then.` };
  const per = periods.map(([a, z]) => {
    const over = p.continuous.on && z - a > p.continuous.value;
    const lines = [`Driving period ${hm(z - a)}  ·  ${clock(a)}–${clock(z)}`];
    lines.push(p.continuous.on ? `${over ? 'Over' : 'Within'} the Continuous Break limit: at most ${hm(p.continuous.value)} before a break of ${hm(rest)}.` : 'The Continuous Break rule is off.');
    return { from: a, to: z, text: hm(z - a), over, tip: lines.join('\n') };
  });
  return {
    json: { breaks: breaks.map(({ meal: _m, ...b }) => b), periods: per, meal, period_hard: p.continuous.hard, meal_hard: p.meal.hard },
    panel: {
      breaks: breaks.map((b) => ({ words: `${b.label}  ·  ${clock(b.from)}–${clock(b.to)}`, kind: b.kind, counts: b.counts, tip: b.tip })),
      periods: per.map((x) => ({ words: `${x.text}  ·  ${clock(x.from)}–${clock(x.to)}`, over: x.over, tip: x.tip })),
      meal: meal ? meal.tip : null,
      meal_hard: p.meal.hard,
      period_hard: p.continuous.hard,
      none: `No break: every gap is shorter than ${hm(p.break_from)}.`,
    },
  };
}

/** The day made from `model`: the board as the page draws it, and what the answers need besides. */
function derive(model, wd) {
  const p = prefs;
  const blocks = model.blocks.map((b) => ({ ...b, pieces: piecesOf(b) }));
  const issues = [];
  const issue = (scope, row, group, title, text, hard, key, penalty = 0, words = '', timeplan = null) => issues.push({ scope, row, text, group, title, hard, key, dismissed: model.dismissed.includes(key), row_words: words, timeplan, penalty });
  // the vehicles' rules: the layover
  blocks.forEach((b, i) => {
    b.bad = [];
    for (let j = 1; j < b.trips.length; j++) {
      const a = b.trips[j - 1];
      const x = b.trips[j];
      if (a.empty || x.empty || !p.layover.on) continue;
      const lay = x.dep - a.arr;
      if (lay < p.layover.value && eq(a.to, x.from)) {
        b.bad.push(x.dep);
        issue('vehicle', i, 'layover', 'Layover too short', `Layover of ${lay} min at ${x.from} before the ${clock(x.dep)} trip: at least ${hm(p.layover.value)}`, p.layover.hard, `layover/${b.line}/${b.tour}/${x.dep}`, p.layover.penalty, `Vehicle ${blockLabel(b)}`);
      }
    }
  });
  // the duties: the pieces joined, the others one each, the spare lines
  const pieces = [];
  blocks.forEach((b, i) => b.pieces.forEach(([s, e], k) => pieces.push({ tour: i, duty: k, key: pieceKey(b, s), s, e })));
  const taken = new Set();
  const groups = [];
  for (const j of model.joins) {
    const members = pieces.filter((x) => j.includes(x.key) && !taken.has(x.key));
    if (!members.length) continue;
    members.forEach((x) => taken.add(x.key));
    groups.push(members);
  }
  for (const x of pieces) if (!taken.has(x.key)) groups.push([x]);
  const spanOf = (x) => {
    const b = blocks[x.tour];
    const n = b.trips.length;
    const from = x.s === 0 ? b.trips[0].dep - p.pre_trip : b.trips[x.s].dep;
    const to = x.e === n ? b.trips[n - 1].arr + p.post_trip : b.trips[x.e - 1].arr;
    return { ...x, from, to, trips: b.trips.slice(x.s, x.e) };
  };
  const travelMin = () => p[['walk', 'public', 'car'][Math.max(0, TRAVEL.indexOf(p.travel))]];
  const travelMode = Math.max(0, TRAVEL.indexOf(p.travel));
  const shaped = groups.map((g) => {
    const spans = g.map(spanOf).sort((a, z) => a.from - z.from);
    const first = spans[0];
    const firstTrip = first.trips.find((x) => !x.empty) || first.trips[0];
    const fb = blocks[first.tour];
    const id = `D|${fb.line}/${fb.tour}/${firstTrip.name}/${firstTrip.dep}`;
    const note = model.notes[id] || {};
    const signOn = note.sign_on ?? p.sign_on;
    const signOff = note.sign_off ?? p.sign_off;
    const events = [];
    const splits = [];
    const mode = note.travelEv ?? travelMode;
    const tm = travelMin();
    const travel = (from, to) => events.push({ kind: 'travel', letter: TRAVEL_LETTER[mode], mode, label: TRAVEL_LABELS[mode], from, to });
    spans.forEach((sp, k) => {
      const prev = spans[k - 1];
      const startsAfterSplit = prev && sp.from - prev.to >= p.split_min;
      if (!prev || startsAfterSplit) {
        const on = !prev ? signOn : p.sign_on_split;
        const at = note.sign_on_at || pointName(blocks[sp.tour].trips[sp.s].from);
        if (sp.s === 0) events.push({ kind: 'on', from: sp.from - on, to: sp.from, ...(prev ? {} : { at }) });
        else {
          events.push({ kind: 'on', from: sp.from - on - tm, to: sp.from - tm, ...(prev ? {} : { at: note.sign_on_at || 'Depot' }) });
          travel(sp.from - tm, sp.from);
        }
      } else if (!eq(blocks[prev.tour].trips[prev.e - 1].to, blocks[sp.tour].trips[sp.s].from)) travel(prev.to, Math.min(sp.from, prev.to + tm));
      const next = spans[k + 1];
      const endsBeforeSplit = next && next.from - sp.to >= p.split_min;
      if (!next || endsBeforeSplit) {
        const off = !next ? signOff : p.sign_off_split;
        const at = note.sign_off_at || pointName(blocks[sp.tour].trips[sp.e - 1].to);
        if (sp.e === blocks[sp.tour].trips.length) events.push({ kind: 'off', from: sp.to, to: sp.to + off, ...(next ? {} : { at }) });
        else {
          travel(sp.to, sp.to + tm);
          events.push({ kind: 'off', from: sp.to + tm, to: sp.to + tm + off, ...(next ? {} : { at: note.sign_off_at || 'Depot' }) });
        }
      }
      if (endsBeforeSplit) splits.push([sp.to, next.from]);
    });
    (note.custom || []).forEach((cw, ci) => events.push({ kind: 'custom', ci, label: CUSTOM_KINDS[cw.kind] || 'Spare', from: cw.from, to: cw.to }));
    const start = Math.min(...events.map((e) => e.from));
    const end = Math.max(...events.map((e) => e.to));
    const allTrips = spans.flatMap((sp) => sp.trips).sort((a, z) => a.dep - z.dep);
    return { id, spans, events, splits, start, end, note, trips: allTrips, spare: null, name: '', locked: model.dutyLocks.includes(id) };
  });
  for (const sp of model.spares) {
    const id = `D|spare/${sp.id}`;
    shaped.push({
      id, spans: [], splits: [], start: sp.from - p.sign_on, end: sp.to + p.sign_off, note: model.notes[id] || {}, trips: [], spare: sp.id, name: sp.name || '', locked: !!sp.locked,
      events: [{ kind: 'on', from: sp.from - p.sign_on, to: sp.from, at: 'Depot' }, { kind: 'custom', ci: 0, label: CUSTOM_KINDS[sp.kind || 0], from: sp.from, to: sp.to }, { kind: 'off', from: sp.to, to: sp.to + p.sign_off, at: 'Depot' }],
    });
  }
  shaped.sort((a, z) => (a.spare != null) - (z.spare != null) || a.start - z.start);
  // (their figures and rules)
  const typeNames = prefs.duty_types.map((t) => t.name);
  shaped.forEach((s, row) => {
    s.row = row;
    s.label = s.spare != null ? `90${pad(s.spare)}` : String(1001 + row);
    const splitTime = s.splits.reduce((a, [x, y]) => a + (y - x), 0);
    s.spread = s.end - s.start;
    s.work = s.spread - splitTime;
    s.paid = s.note.paid ?? Math.max(MIN_PAID, s.work);
    s.driving = s.trips.reduce((a, x) => a + (x.arr - x.dep), 0);
    s.platform = s.spans.reduce((a, x) => a + (x.to - x.from), 0);
    s.changeovers = Math.max(0, s.spans.length - 1 - s.splits.length);
    s.km = Math.round(s.trips.reduce((a, x) => a + x.km, 0) * 10) / 10;
    s.nTrips = s.trips.filter((x) => !x.empty).length;
    const auto = kindOf(s.start, s.end, s.splits.length > 0);
    s.kindSet = !!s.note.kind;
    s.kindIdx = s.kindSet ? (s.note.kind <= typeNames.length ? s.note.kind - 1 : -1) : auto;
    s.kindName = s.kindIdx >= 0 ? typeNames[s.kindIdx] : 'Other';
    s.rest = restOf(s.trips, s.splits, s.start);
    const w = `Duty ${s.label}`;
    if (!s.spare && s.kindIdx < 0) issue('duty', row, 'type', 'Duty type unsatisfied', `No allowed duty type fits it (${clock(s.start)}–${clock(s.end)}, ${hm(s.spread)})`, false, `type/${s.id}`, 2500, w);
    s.rest.json.periods.forEach((x) => x.over && issue('duty', row, 'continuous', 'Continuous Break', `${hm(x.to - x.from)} of driving without a break of ${hm(Math.max(p.break_min, p.break_from))} (at most ${hm(p.continuous.value)})`, p.continuous.hard, `continuous/${s.id}/${x.from}`, p.continuous.penalty, w));
    if (s.rest.json.meal) issue('duty', row, 'meal', 'No meal break', s.rest.json.meal.tip, p.meal.hard, `meal/${s.id}`, p.meal.penalty, w);
    if (p.max_spread.on && s.spread > p.max_spread.value) issue('duty', row, 'spread', 'Spread too long', `Spread ${hm(s.spread)}: at most ${hm(p.max_spread.value)}`, p.max_spread.hard, `spread/${s.id}`, p.max_spread.penalty, w);
    if (p.min_piece.on) s.spans.forEach((sp) => sp.to - sp.from < p.min_piece.value && issue('duty', row, 'piece', 'Piece too short', `A piece of ${hm(sp.to - sp.from)}: at least ${hm(p.min_piece.value)}`, p.min_piece.hard, `piece/${s.id}/${sp.from}`, p.min_piece.penalty, w));
    if (p.changeovers.on && s.changeovers > p.changeovers.value) issue('duty', row, 'changeover', 'Too many changeovers', `${s.changeovers} changes of bus: at most ${p.changeovers.value}`, p.changeovers.hard, `changeover/${s.id}`, p.changeovers.penalty, w);
  });
  model.stack.forEach((x) => issue('global', null, 'stack', 'Trip not assigned', `Line ${lineOf(x.line)?.number} ${clock(x.dep)} ${x.from} → ${x.to} runs on no vehicle`, true, `stack/${x.line}/${x.dep}`));
  const open = (i) => !i.dismissed;
  const tipOf = (list) => ({ any: list.length > 0, hard: list.some((i) => i.hard), tip: list.map((i) => i.text).join('\n') });
  // the blocks as the page draws them
  const rowOfPiece = (i, k) => shaped.findIndex((s) => s.spans.some((sp) => sp.tour === i && sp.duty === k));
  const blocksJson = blocks.map((b, i) => {
    const L = lineOf(b.line);
    const trips = b.trips.map((x, j) => {
      const v = {
        dep: x.dep, arr: x.arr, from: x.from, to: x.to, empty: x.empty, counts: !x.empty, origin: x.origin, name: x.name, num: L.number,
        km: Math.round(x.km * 10) / 10, out: x.out, linked: !x.empty && model.linked.includes(tripKey(x)), moved: !!x.origin && x.origin !== b.tour, bad: b.bad.includes(x.dep),
        sl: `${letters(x.from)} > ${letters(x.to)}`,
      };
      return v;
    });
    const starts = b.pieces.map(([s]) => s);
    const gaps = [];
    for (let at = 1; at < b.trips.length; at++) {
      if (starts.includes(at)) continue;
      const a = b.trips[at - 1];
      const x = b.trips[at];
      if (a.empty || x.empty) gaps.push([at, false, 'A duty is cut where the bus stands at a relief point: not on its way to or from the depot.']);
      else if (eq(a.to, x.from) && isRelief(x.from)) gaps.push([at, true, `Click to cut the duty here: a driver takes over at ${x.from}.`]);
      else gaps.push([at, false, `${x.from} is no relief point: drivers do not change here (Preferences › Relief Points).`]);
    }
    const mine = issues.filter((x) => x.scope === 'vehicle' && x.row === i && open(x));
    return {
      id: `B|${b.line}|${b.tour}`, line: b.line, number: L.number, tour: b.tour, label: blockLabel(b), colour: colourOf(b.line),
      from: b.trips[0].dep, to: b.trips[b.trips.length - 1].arr, km: Math.round(b.trips.reduce((a, x) => a + x.km, 0) * 10) / 10,
      locked: model.blockLocks.includes(blockKey(b)), trips, pieces: b.pieces.map(([s, e]) => [s, e]),
      duty_rows: b.pieces.map((_, k) => {
        const r = rowOfPiece(i, k);
        return r < 0 ? null : r;
      }),
      gaps, relief_tips: b.pieces.slice(1).map(([s]) => `A driver takes over here, at ${b.trips[s].from}. Click to join the two duties.`),
      issues: tipOf(mine),
    };
  });
  const dutiesJson = shaped.map((s, row) => {
    const mine = issues.filter((x) => x.scope === 'duty' && x.row === row && open(x));
    return {
      id: s.id, label: s.name ? `${s.label} ${s.name}` : s.label, name: s.name, spare: s.spare, locked: s.locked,
      kind: s.kindName, kind_rank: s.kindIdx >= 0 ? s.kindIdx : 99, kind_set: s.kindSet,
      start: s.start, end: s.end, spread: s.spread, work: s.work, paid: s.paid, driving: s.driving, platform: s.platform,
      changeovers: s.changeovers, km: s.km, trips: s.nTrips, splits: s.splits.map((x) => [...x]),
      spans: s.spans.map((sp) => ({ tour: sp.tour, duty: sp.duty, from: sp.from, to: sp.to })),
      events: s.events, issues: tipOf(mine), rest: s.rest.json,
    };
  });
  // the figures (`Figures`)
  const counted = blocks.flatMap((b) => b.trips.filter((x) => !x.empty));
  const serviceKm = counted.reduce((a, x) => a + x.km, 0);
  const deadKm = blocks.flatMap((b) => b.trips.filter((x) => x.empty)).reduce((a, x) => a + x.km, 0);
  const sum = (f) => shaped.reduce((a, s) => a + f(s), 0);
  const rt = rates();
  const paid = sum((s) => s.paid);
  const f = {
    duties: shaped.length, blocks: blocks.length, pvr: blocks.length, service_km: serviceKm, dead_km: deadKm,
    driving: sum((s) => s.driving), paid, spread: sum((s) => s.spread), work: sum((s) => s.work), platform: sum((s) => s.platform),
    splits: shaped.filter((s) => s.splits.length).length, split_time: sum((s) => s.splits.reduce((a, [x, y]) => a + y - x, 0)),
    changeovers: sum((s) => s.changeovers), split_changeovers: 0,
    crew_cost: Math.round((paid / 60) * rt.wage) + shaped.length * rt.driver_fixed,
    vehicle_cost: blocks.length * rt.vehicle_fixed + Math.round((serviceKm + deadKm) * rt.per_km),
    penalties: issues.filter((i) => open(i) && !i.hard).reduce((a, i) => a + i.penalty, 0),
    types: [...prefs.duty_types.map((t, k) => [t.name, shaped.filter((s) => s.kindIdx === k).length]), ['Other', shaped.filter((s) => s.kindIdx < 0).length]],
    paid_bands: [0, 1, 2, 3, 4].map((k) => shaped.filter((s) => {
      const h = s.paid / 60;
      return k === 0 ? h < 5 : k === 1 ? h >= 5 && h < 7 : k === 2 ? h >= 7 && h < 9 : k === 3 ? h >= 9 && h < 12 : h >= 12;
    }).length),
    assigned: counted.length, trips: counted.length + model.stack.length,
  };
  const scopeOpen = (scope) => {
    const list = issues.filter((i) => i.scope === scope && open(i));
    return { n: list.length, hard: list.some((i) => i.hard) };
  };
  const t0 = Math.floor((Math.min(...blocks.map((b) => b.trips[0].dep), ...shaped.map((s) => s.start), 300) - 20) / 60) * 60;
  const t1 = Math.ceil((Math.max(...blocks.map((b) => b.trips[b.trips.length - 1].arr), ...shaped.map((s) => s.end), 1380) + 20) / 60) * 60;
  const stack = model.stack.map((x) => ({ line: x.line, number: lineOf(x.line)?.number || '', colour: colourOf(x.line), dep: x.dep, arr: x.arr, from: x.from, to: x.to, num: lineOf(x.line)?.number || '', name: x.name, out: x.out }));
  const json = {
    weekday: wd,
    span: [t0, Math.max(t1, t0 + 180)],
    blocks: blocksJson,
    duties: dutiesJson,
    issues: issues.map(({ penalty: _p, ...i }) => i),
    stack,
    status: { assigned: f.assigned, trips: f.trips, vehicle: scopeOpen('vehicle'), duty: scopeOpen('duty'), global: scopeOpen('global') },
    probe: { blocks: f.blocks, duties: f.duties, locked: shaped.filter((s) => s.locked).length, issues: issues.filter(open).length },
  };
  return { json, figures: f, blocks, shaped, issues };
}

/** A stop's letters for the chips' "Start and end stops" (`rosterpdf::stop_letters`). */
function letters(stop) {
  return String(stop).split(/\s+/).map((w) => w[0] || '').join('').toUpperCase().slice(0, 3);
}

/** The KPI tiles (`kpis_json`), against the figures before the last edit. */
function kpisJson(f, was) {
  const pct = (x) => `${Math.round(x * 100)}%`;
  const hours = (m) => String(Math.round(m / 60));
  const n = (x) => String(Math.round(x));
  const diff = (now, w, up, fmt) => {
    if (w == null) return null;
    const d = now - w;
    if (Math.abs(d) < 1e-6) return null;
    return [`${d > 0 ? '+' : '-'}${fmt(Math.abs(d))}`, d > 0 === up];
  };
  const veh = (x) => (x.service_km + x.dead_km ? x.service_km / (x.service_km + x.dead_km) : 0);
  const crew = (x) => (x.paid ? x.driving / x.paid : 0);
  const total = (x) => x.crew_cost + x.vehicle_cost;
  const avg = (m, x = f) => (x.duties ? m / x.duties : 0);
  const tile = (icon, value, delta, caption, formula = '') => ({ icon, value, delta, caption: caption.toUpperCase(), formula });
  const top = [
    tile('person', String(f.duties), diff(f.duties, was?.duties, false, n), 'Duties'),
    tile('directions_bus', String(f.blocks), diff(f.blocks, was?.blocks, false, n), `Blocks (PVR: ${f.pvr})`),
    tile('route', pct(veh(f)), diff(veh(f), was && veh(was), true, (d) => `${Math.round(d * 100)}%`), 'Vehicle efficiency', `(Services / Deadheads + Services): ${Math.round(f.service_km)} km / (${Math.round(f.dead_km)} + ${Math.round(f.service_km)}) km`),
    tile('schedule', pct(crew(f)), diff(crew(f), was && crew(was), true, (d) => `${Math.round(d * 100)}%`), 'Crew efficiency', `(Driving time / Paid time): ${hours(f.driving)} / ${hours(f.paid)}`),
    tile('payments', eur(total(f)), diff(total(f), was && total(was), false, eur), 'Total cost', `(Crew + Vehicles): ${eur(f.crew_cost)} + ${eur(f.vehicle_cost)}`),
  ];
  const more = [
    [
      tile('sync_alt', String(f.splits), diff(f.splits, was?.splits, false, n), 'Split count', `Time: ${Math.floor(f.split_time / 60)} h | ${f.duties ? Math.round((f.splits * 100) / f.duties) : 0}% of duties`),
      tile('commute', String(f.changeovers), diff(f.changeovers, was?.changeovers, false, n), 'Changeover', `During split: ${f.split_changeovers}`),
      tile('history', hm(avg(f.paid)), diff(avg(f.paid), was && avg(was.paid, was), false, hm), 'Average paid time'),
      tile('timer', hours(f.paid), diff(f.paid / 60, was && was.paid / 60, false, n), 'Paid time (hours)'),
      tile('schedule', hours(f.spread), diff(f.spread / 60, was && was.spread / 60, false, n), 'Spread time (hours)'),
    ],
    [
      tile('event', hours(f.work), null, 'Work time (hours)', `Average: ${hm(avg(f.work))}`),
      tile('speed', hours(f.driving), null, 'Driving time (hours)'),
      tile('directions_bus', hours(f.platform), null, 'Platform time (hours)'),
      tile('warning', eur(f.penalties), diff(f.penalties, was?.penalties, false, eur), 'Penalties', 'Soft rules broken'),
      tile('tune', eur(f.crew_cost + f.penalties), diff(f.crew_cost + f.penalties, was && was.crew_cost + was.penalties, false, eur), 'Crew algorithmic cost', `(Duty cost + Penalties): ${eur(f.crew_cost)} + ${eur(f.penalties)}`),
    ],
  ];
  return {
    top,
    more,
    types: f.types.filter((t) => t[1] > 0 || t[0] !== 'Other'),
    all_types: f.types,
    bands: PAID_BANDS.map((b, k) => [b, f.paid_bands[k]]),
  };
}

// --- the views of the board ---------------------------------------------------------------------------

function onJson() {
  const c = syncCompany();
  if (c === null) return { open: false, on: false };
  const dayList = [0, 1, 2, 3, 4, 5, 6].map((k) => {
    const date = addDays(COMPANY.date, k);
    return { date, label: k === 0 ? `${dayLabel(date)}  ·  Company day` : dayLabel(date) };
  });
  return { open: true, on: addonOn(c), company: COMPANY.id, name: COMPANY.name, date: COMPANY.date, days: dayList, casual: c?.mode === 'casual', trips_to_duties: true };
}

function headOf(day) {
  const date = dateOf(day);
  const wd = weekdayOf(date);
  const st = dayState(wd);
  const unsynced = tp.unsynced.filter((u) => u[1] === groupOf(wd)).map((u) => ({ line: u[0], number: lineOf(u[0])?.number || u[0] }));
  return {
    date,
    name: `${COMPANY.name}  ·  ${dayLabel(date)}`,
    company: COMPANY.name,
    day_text: dayLabel(date),
    day_words: dayLabel(date).toUpperCase(),
    service: WD[wd],
    draft: hasDraft(),
    undo: st.undo.map((u) => u.label),
    redo: st.redo.length,
    edits,
    gen: version,
    days: [0, 1, 2, 3, 4, 5, 6].map(() => []),
    day: { tags: [], reset: false, refresh: false, pattern: null, matched: 0, total: 0, missing: [] },
    unsynced,
    colour_by: colourBy,
    outcomes: null,
  };
}

function boardAnswer(args) {
  const day = Number(args.day) || 0;
  const wd = wdOf(day);
  const head = headOf(day);
  const rev = `${wd}|${version}`;
  if (args.have === rev) return { same: true, head };
  const st = dayState(wd);
  const d = derive(modelOf(wd), wd);
  const prop = proposals[wd];
  return {
    head,
    rev,
    board: d.json,
    kpis: kpisJson(d.figures, st.before),
    proposal: !!prop,
    proposal_better: prop ? prop.better : null,
    proposal_note: prop ? prop.note : null,
    lines: LINES.map((l) => ({ name: l.name, number: l.number, caption: l.caption, colour: colourOf(l.name), own_colour: true })),
    prefs: { relief: Math.max(0, RELIEF.indexOf(prefs.relief)), pre_trip: prefs.pre_trip, post_trip: prefs.post_trip, relief_before: prefs.relief_before, optimise_vehicles: prefs.optimise_vehicles, optimise_duties: prefs.optimise_duties },
    palette: PALETTE,
    saved_same: true,
    colours: LINES.map((l) => colourOf(l.name)),
  };
}

/** The relief points (`rosterflow::relief_points`): the depot and the lines' termini. */
function reliefRows() {
  const rows = [{ stop: 'Depot', name: 'Depot', kind: 0, lines: LINES.map((l) => l.number).join(', ') }];
  for (const l of LINES) {
    for (const s of [l.a, l.b]) {
      const r = rows.find((x) => eq(x.stop, s));
      if (r) r.lines = `${r.lines}, ${l.number}`;
      else rows.push({ stop: s, name: s, kind: 1, lines: l.number });
    }
  }
  return rows.map((r) => ({ ...r, point: { stop: r.stop, added: false, line: '', sign: true, relief: true, breaks: true, travel: null, walk: null, public: null, car: null, deadhead_min: null, deadhead_km: null, ...(prefs.points.find((x) => eq(x.stop, r.stop)) || {}) } }));
}

function dutyView(d, row) {
  const s = d.shaped[row];
  if (!s) return { gone: true };
  const note = s.note;
  const names = ['Automatic', ...prefs.duty_types.map((t) => t.name), 'Other'];
  const typeAt = s.kindSet ? (s.kindIdx >= 0 ? s.kindIdx + 1 : names.length - 1) : 0;
  const points = reliefRows().map((r) => ({ key: r.stop, name: r.name }));
  const times = [
    ['Sign-on', clock(s.start)], ['Sign-off', clock(s.end)], ['Spread', hm(s.spread)], ['Work time', hm(s.work)], ['Driving time', hm(s.driving)],
    ['Platform time', hm(s.platform)], ['Splits', s.splits.length ? s.splits.map(([a, z]) => `${clock(a)}–${clock(z)}`).join(', ') : '-'], ['Changeovers', String(s.changeovers)],
  ];
  const first = s.spans[0];
  const last = s.spans[s.spans.length - 1];
  const pieces = s.spans.map((sp, n) => {
    const b = d.blocks[sp.tour];
    const a = sp.trips[0];
    const z = sp.trips[sp.trips.length - 1];
    const v = { tour: sp.tour, duty: sp.duty, number: lineOf(b.line).number, colour: colourOf(b.line), words: `${blockLabel(b)}  ${clock(sp.from)}–${clock(sp.to)}  ${a.from} → ${z.to}` };
    const next = s.spans[n + 1];
    if (next) {
      v.break_at = note.break_at?.[n] ?? null;
      const here = z.to;
      const there = next.trips[0].from;
      if (!eq(here, there)) v.travel = note.travel?.[n] ?? Math.max(0, TRAVEL.indexOf(prefs.travel));
      else {
        const gap = next.from - sp.to;
        v.between = gap >= prefs.split_min ? `Split of ${hm(gap)}` : `Break of ${hm(gap)}`;
      }
    }
    return v;
  });
  const spareOf = s.spare != null ? modelOf(d.wd).spares.find((x) => x.id === s.spare) : null;
  return {
    label: s.label, id: s.id, kind: s.kindName, locked: s.locked, spare: s.spare != null,
    name: spareOf?.name || '', spare_at: spareOf?.sign_on_at ?? null, spare_company: pointName(prefs.sign_on_at || 'Depot'),
    types: names, type_at: typeAt, times,
    paid: s.paid, paid_text: hm(s.paid), paid_set: note.paid != null,
    signing: s.spare == null,
    sign_on: note.sign_on ?? prefs.sign_on, sign_off: note.sign_off ?? prefs.sign_off,
    sign_on_at: note.sign_on_at ?? null, sign_off_at: note.sign_off_at ?? null,
    sign_company: { on: prefs.sign_on_at ? pointName(prefs.sign_on_at) : null, off: prefs.sign_off_at ? pointName(prefs.sign_off_at) : null },
    sign_places: { on: first ? pointName(first.trips[0].from) : 'Depot', off: last ? pointName(last.trips[last.trips.length - 1].to) : 'Depot' },
    sign_points: points, break_points: points, pieces, travel_labels: TRAVEL_LABELS,
    rest: s.rest.panel,
    custom: (note.custom || []).map((cw) => `${CUSTOM_KINDS[cw.kind] || 'Spare'}  ${clock(cw.from)}–${clock(cw.to)}`),
    kinds: CUSTOM_KINDS,
    issues: d.issues.filter((i) => i.scope === 'duty' && i.row === row).map((i) => ({ text: i.text, hard: i.hard })),
  };
}

function prefsView() {
  const rt = rates();
  const rows = reliefRows();
  return {
    relief_points: {
      points: rows.map((r) => ({ stop: r.stop, name: r.name, kind: r.kind, lines: r.lines, point: r.point })),
      line_stops: LINES.map((l) => ({ line: l.name, number: l.number, stops: l.stops.map((s) => s[0]) })),
      kinds: ['Depot', 'Terminus', 'On the way'],
    },
    prefs: clone(prefs),
    defaults: defaultPrefs(),
    rates: { wage: rt.wage, driver_fixed: rt.driver_fixed, vehicle_fixed: rt.vehicle_fixed, per_km: rt.per_km },
    relief_labels: RELIEF_LABELS,
    travel_labels: TRAVEL_LABELS,
    tri_labels: TRI_LABELS,
    type_names: prefs.duty_types.map((t) => t.name),
    min_paid: MIN_PAID,
  };
}

/** How many duties (vehicles, the day) break each rule (`breaks::rule_counts`). */
function ruleCounts(d) {
  const by = { breaks: ['meal'], continuous: ['continuous'], limits: ['spread'], length: [], pieces: ['piece'], changeover: ['changeover'], types: ['type'], travel: [], layover: ['layover'], depot: [] };
  const out = {};
  for (const [item, groups] of Object.entries(by)) out[item] = new Set(d.issues.filter((i) => groups.includes(i.group)).map((i) => `${i.scope}|${i.row}`)).size;
  out.split = d.shaped.filter((s) => s.splits.length).length;
  return out;
}

function prefsCheck(day, edited) {
  const wd = wdOf(day);
  const now = ruleCounts(derive(modelOf(wd), wd));
  const was = prefs;
  prefs = { ...defaultPrefs(), ...edited };
  let then;
  try {
    then = ruleCounts(derive(modelOf(wd), wd));
  } finally {
    prefs = was;
  }
  const of = { layover: 'vehicles', depot: 'day' };
  const counts = {};
  for (const [item, n] of Object.entries(then)) {
    let text;
    if (item === 'split') text = n ? `${n} duties have a split now.` : 'No duty has a split now.';
    else if (!n) text = 'Nothing breaks this rule now.';
    else if (of[item] === 'vehicles') text = `${n} vehicles break this rule now.`;
    else if (of[item] === 'day') text = 'The day breaks this rule now.';
    else text = `${n} duties break this rule now.`;
    if (now[item] !== n) text += ` (${now[item]} with the saved rules)`;
    counts[item] = { n, was: now[item], text };
  }
  return { counts };
}

function applyView(day) {
  const date = dateOf(day);
  const wd = weekdayOf(date);
  const k = Number(day) || 0;
  const daysOut = [0, 1, 2, 3, 4, 5, 6].filter((x) => x !== k).map((x) => {
    const d = addDays(COMPANY.date, x);
    const w = weekdayOf(d);
    const same = groupOf(w) === groupOf(wd);
    const total = derive(dayState(w).saved, w).figures.assigned;
    return { k: x, date: d, label: dayLabel(d), weekday: WD[w], in_type: same, same, tags: [], own: !!dayState(w).draft, preview: same ? { matched: total, total, exact: true, missing: 0, gone: 0 } : { matched: Math.round(total * 0.6), total, exact: false, missing: total - Math.round(total * 0.6), gone: 2 }, reading: false };
  });
  return { date, label: dayLabel(date), k, weekday: WD[wd], type: DAY_GROUPS[groupOf(wd)], date_only: false, dirty: hasDraft(), reading: false, days: daysOut, others: [] };
}

/** The stars (`rate`): the cheapest five, the dearest one; a zone that breaks a rule two at most. */
function rate(zones) {
  const deltas = [...new Set(zones.map((z) => z.delta))].sort((a, z) => a - z);
  const n = Math.max(1, deltas.length);
  return zones.map((z) => {
    const k = deltas.indexOf(z.delta);
    const s = n <= 1 ? 5 : Math.round(5 - (k * 4) / (n - 1));
    return z.clean ? s : Math.min(2, s);
  });
}

/** Whether `trips` fit into a bus's day beside `have` (no two at once). */
function fits(have, trips) {
  const all = [...have, ...trips].sort((a, z) => a.dep - z.dep);
  for (let k = 1; k < all.length; k++) if (all[k].dep < all[k - 1].arr) return false;
  return true;
}

/** Whether a duty's time takes [a, z] (with its signing) besides its own. */
const dutyFree = (s, a, z) => s.spans.every((sp) => z + 15 <= sp.from || a >= sp.to + 15) && s.events.filter((e) => e.kind === 'custom').every((e) => z + 15 <= e.from || a >= e.to + 15);

/** Where what is in hand may go (`zones_for`), each rated by what it costs. */
function zonesFor(day, hand) {
  const wd = wdOf(day);
  const model = modelOf(wd);
  const d = derive(model, wd);
  const base = d.figures.crew_cost + d.figures.vehicle_cost + d.figures.penalties;
  const hardNow = d.issues.filter((i) => i.hard && !i.dismissed).length;
  const out = [];
  const probe = (drop, f) => {
    const m = clone(model);
    if (f(m) !== true) return;
    const nd = derive(m, wd);
    const hard = nd.issues.filter((i) => i.hard && !i.dismissed).length;
    out.push({ drop, delta: nd.figures.crew_cost + nd.figures.vehicle_cost + nd.figures.penalties - base, clean: hard <= hardNow });
  };
  if (hand?.kind === 'trips') {
    const src = d.blocks[hand.tour];
    if (!src) return [];
    d.blocks.forEach((b, i) => {
      if (i === hand.tour || !eq(b.line, src.line)) return;
      probe({ kind: 'block', i }, (m) => moveTrips(m, hand.tour, hand.trips || [], i) === true);
    });
    probe({ kind: 'new' }, (m) => moveTrips(m, hand.tour, hand.trips || [], null) === true);
    probe({ kind: 'stack' }, (m) => unassign(m, hand.tour, hand.trips || []) === true);
  } else if (hand?.kind === 'stack') {
    const x = model.stack[hand.k];
    if (!x) return [];
    d.blocks.forEach((b, i) => eq(b.line, x.line) && probe({ kind: 'block', i }, (m) => assign(m, hand.k, i) === true));
    probe({ kind: 'new' }, (m) => assign(m, hand.k, null) === true);
  } else if (hand?.kind === 'piece') {
    const row = d.shaped.findIndex((s) => s.spans.some((sp) => sp.tour === hand.tour && sp.duty === hand.duty));
    const sp = d.shaped[row]?.spans.find((x) => x.tour === hand.tour && x.duty === hand.duty);
    if (!sp) return [];
    d.shaped.forEach((s, r) => {
      if (r === row || s.spare != null || !dutyFree(s, sp.from, sp.to)) return;
      probe({ kind: 'duty', d: r }, (m) => joinPiece(m, d, hand.tour, hand.duty, r) === true);
    });
    if (d.shaped[row].spans.length > 1) probe({ kind: 'new_duty' }, (m) => detach(m, d, hand.tour, hand.duty) === true);
  } else if (hand?.kind === 'duty_trips') {
    const plan = dutyTripsPlan(d, hand);
    if (typeof plan === 'string') return [];
    d.shaped.forEach((s, r) => {
      if (r === plan.row || s.spare != null || !dutyFree(s, plan.from, plan.to)) return;
      probe({ kind: 'duty', d: r }, (m) => moveToDuty(m, d, hand, r) === true);
    });
    probe({ kind: 'new_duty' }, (m) => moveToDuty(m, d, hand, null) === true);
  }
  const stars = rate(out);
  return out.map((z, k) => ({ drop: z.drop, delta: z.delta, delta_text: `${z.delta > 0 ? '+ ' : z.delta < 0 ? '- ' : ''}${eur(Math.abs(z.delta))}`, clean: z.clean, stars: stars[k] }));
}

function view(args) {
  const what = String(args.view || '');
  const day = Number(args.day) || 0;
  if (what === 'prefs') return prefsView();
  if (what === 'apply') return applyView(day);
  const wd = wdOf(day);
  const d = derive(modelOf(wd), wd);
  d.wd = wd;
  switch (what) {
    case 'candidates': {
      const t = d.blocks[Number(args.tour)];
      const x = t?.trips[Number(args.trip)];
      if (!x) return [];
      return d.blocks.map((b, i) => [b, i]).filter(([b, i]) => i !== Number(args.tour) && eq(b.line, t.line) && fits(b.trips, [x])).map(([b, i]) => ({ tour: i, label: blockLabel(b) }));
    }
    case 'zones':
      return zonesFor(day, args.hand);
    case 'why': {
      const plan = args.hand?.kind === 'duty_trips' ? dutyTripsPlan(d, args.hand) : null;
      if (typeof plan === 'string') return { text: plan };
      return args.hand?.kind === 'duty_trips' ? { text: 'No other duty is free while these trips run: drop them on New duty.' } : {};
    }
    case 'duty':
      return dutyView(d, Number(args.row) || 0);
    case 'prefs_check':
      return prefsCheck(day, args.prefs || {});
    default:
      throw `no view ${what}`;
  }
}

// --- the edits (`rf_act`) -------------------------------------------------------------------------------

/** The block a trip lands on: its trips in order, its pull-out and pull-in kept at the ends. */
function insertTrips(blk, trips) {
  blk.trips = [...blk.trips, ...trips].sort((a, z) => a.dep - z.dep);
}

/** A block left without passengers goes, its cuts to trips it no longer has dropped. */
function tidy(m) {
  m.blocks = m.blocks.filter((b) => b.trips.some((x) => !x.empty));
  for (const b of m.blocks) b.cuts = b.cuts.filter((c) => b.trips.some((x, j) => j > 0 && x.dep === c));
}

function lockedBlock(m, i) {
  const b = m.blocks[i];
  return b && m.blockLocks.includes(blockKey(b)) ? `Vehicle ${blockLabel(b)} is locked: unlock it first.` : null;
}

/** Trips `js` of block `i` to block `to` (null: a new bus of the line). */
function moveTrips(m, i, js, to) {
  const src = m.blocks[i];
  if (!src) return 'That vehicle is gone.';
  const why = lockedBlock(m, i) || (to != null && lockedBlock(m, to));
  if (why) return why;
  const moving = js.map((j) => src.trips[j]).filter((x) => x && !x.empty);
  if (!moving.length) return 'Pick a trip with passengers.';
  let dst;
  if (to == null) {
    const nums = m.blocks.filter((b) => eq(b.line, src.line)).map((b) => Number(b.tour) || 0);
    dst = { line: src.line, tour: pad(Math.max(0, ...nums) + 1), trips: [], cuts: [] };
    m.blocks.push(dst);
  } else dst = m.blocks[to];
  if (!dst || !eq(dst.line, src.line)) return 'Trips go only to a bus of their line.';
  if (!fits(dst.trips, moving)) return `Vehicle ${blockLabel(dst)} runs another trip then.`;
  src.trips = src.trips.filter((x) => !moving.includes(x));
  insertTrips(dst, moving);
  tidy(m);
  return true;
}

function unassign(m, i, js) {
  const src = m.blocks[i];
  if (!src) return 'That vehicle is gone.';
  const why = lockedBlock(m, i);
  if (why) return why;
  const moving = js.map((j) => src.trips[j]).filter((x) => x && !x.empty);
  if (!moving.length) return 'Pick a trip with passengers.';
  src.trips = src.trips.filter((x) => !moving.includes(x));
  m.stack.push(...moving);
  m.stack.sort((a, z) => a.dep - z.dep);
  tidy(m);
  return true;
}

function assign(m, k, to) {
  const x = m.stack[k];
  if (!x) return 'That trip is gone.';
  let dst;
  if (to == null) {
    const nums = m.blocks.filter((b) => eq(b.line, x.line)).map((b) => Number(b.tour) || 0);
    dst = { line: x.line, tour: pad(Math.max(0, ...nums) + 1), trips: [], cuts: [] };
    m.blocks.push(dst);
  } else {
    dst = m.blocks[to];
    const why = lockedBlock(m, to);
    if (why) return why;
  }
  if (!dst || !fits(dst.trips, [x])) return `Vehicle ${dst ? blockLabel(dst) : ''} runs another trip then.`;
  m.stack.splice(k, 1);
  insertTrips(dst, [x]);
  return true;
}

/** The piece keys of duty row `r` of the derived day `d`. */
const keysOf = (d, r) => d.shaped[r].spans.map((sp) => pieceKey(d.blocks[sp.tour], sp.s));

function lockedDuty(m, d, r) {
  return d.shaped[r]?.locked ? `Duty ${d.shaped[r].label} is locked: unlock it first.` : null;
}

function joinPiece(m, d, tour, duty, r) {
  const blk = d.blocks[tour];
  const piece = blk?.pieces[duty];
  if (!piece || !d.shaped[r]) return 'That duty is gone.';
  const mine = d.shaped.findIndex((s) => s.spans.some((sp) => sp.tour === tour && sp.duty === duty));
  const why = lockedDuty(m, d, r) || lockedDuty(m, d, mine);
  if (why) return why;
  const key = pieceKey(blk, piece[0]);
  m.joins = m.joins.map((j) => j.filter((k) => k !== key)).filter((j) => j.length > 1);
  const target = keysOf(d, r);
  const g = m.joins.find((j) => j.some((k) => target.includes(k)));
  if (g) g.push(key);
  else m.joins.push([...target, key]);
  return true;
}

function detach(m, d, tour, duty) {
  const blk = d.blocks[tour];
  const piece = blk?.pieces[duty];
  if (!piece) return 'That piece is gone.';
  const mine = d.shaped.findIndex((s) => s.spans.some((sp) => sp.tour === tour && sp.duty === duty));
  const why = lockedDuty(m, d, mine);
  if (why) return why;
  if (d.shaped[mine].spans.length < 2) return 'The piece is a duty of its own already.';
  const key = pieceKey(blk, piece[0]);
  m.joins = m.joins.map((j) => j.filter((k) => k !== key)).filter((j) => j.length > 1);
  return true;
}

/** Luc's driver drag (`driver_drag`): the trips of one duty on one bus, as a range of it. */
function dutyTripsPlan(d, hand) {
  const pairs = (hand.trips || []).filter(([i, j]) => d.blocks[i]?.trips[j]);
  if (!pairs.length) return 'Pick a trip first.';
  const buses = new Set(pairs.map(([i]) => i));
  if (buses.size > 1) return 'The browser mock moves the trips of one bus at a time.';
  const [i] = pairs[0];
  const js = pairs.map(([, j]) => j).sort((a, z) => a - z);
  const blk = d.blocks[i];
  const k = blk.pieces.findIndex(([a, z]) => js[0] >= a && js[0] < z);
  if (k < 0 || js.some((j) => j < blk.pieces[k][0] || j >= blk.pieces[k][1])) return 'The trips chosen are of more than one piece of work.';
  const row = d.shaped.findIndex((s) => s.spans.some((sp) => sp.tour === i && sp.duty === k));
  return { i, k, row, j0: js[0], j1: js[js.length - 1], from: blk.trips[js[0]].dep, to: blk.trips[js[js.length - 1]].arr };
}

function moveToDuty(m, d, hand, r) {
  const plan = dutyTripsPlan(d, hand);
  if (typeof plan === 'string') return plan;
  const why = lockedDuty(m, d, plan.row) || (r != null && lockedDuty(m, d, r));
  if (why) return why;
  const mb = m.blocks.find((b) => blockKey(b) === blockKey(d.blocks[plan.i]));
  const trips = mb.trips;
  const [a, z] = d.blocks[plan.i].pieces[plan.k];
  const ownKeys = keysOf(d, plan.row);
  // (cut before and after the trips moved: the rest stays the duty it was)
  const cutAt = (j) => j > 0 && j < trips.length && !mb.cuts.includes(trips[j].dep) && mb.cuts.push(trips[j].dep);
  cutAt(plan.j0);
  cutAt(plan.j1 + 1);
  const moved = `${mb.line}|${mb.tour}|${trips[plan.j0].dep}`;
  const rest = [];
  if (plan.j0 > a) rest.push(`${mb.line}|${mb.tour}|${trips[a].dep}`);
  if (plan.j1 + 1 < z) rest.push(`${mb.line}|${mb.tour}|${trips[plan.j1 + 1].dep}`);
  m.joins = m.joins.map((j) => j.filter((k) => k !== moved)).filter((j) => j.length > 0);
  const own = [...new Set([...ownKeys.filter((k) => k !== moved), ...rest])];
  m.joins = m.joins.filter((j) => !j.some((k) => own.includes(k)));
  if (own.length > 1) m.joins.push(own);
  if (r != null) {
    const target = keysOf(d, r);
    const g = m.joins.find((j) => j.some((k) => target.includes(k)));
    if (g) g.push(moved);
    else m.joins.push([...target, moved]);
  }
  m.joins = m.joins.filter((j) => j.length > 1);
  return true;
}

/** Swap the trips chosen (or everything from them on) with what bus `to` runs then. */
function swap(m, i, js, to, how) {
  const a = m.blocks[i];
  const b = m.blocks[to];
  if (!a || !b) return 'That vehicle is gone.';
  const why = lockedBlock(m, i) || lockedBlock(m, to);
  if (why) return why;
  const chosen = js.map((j) => a.trips[j]).filter((x) => x && !x.empty);
  if (!chosen.length) return 'Pick a trip with passengers.';
  const lo = Math.min(...chosen.map((x) => x.dep));
  const hi = how === 'from_here' ? Infinity : Math.max(...chosen.map((x) => x.arr));
  const inA = a.trips.filter((x) => !x.empty && x.dep >= lo && x.dep < hi);
  const inB = b.trips.filter((x) => !x.empty && x.dep >= lo && x.dep < hi);
  const restA = a.trips.filter((x) => !inA.includes(x));
  const restB = b.trips.filter((x) => !inB.includes(x));
  if (!fits(restA, inB) || !fits(restB, inA)) return 'The two buses would run two trips at once: no swap.';
  a.trips = [...restA, ...inB].sort((x, y) => x.dep - y.dep);
  b.trips = [...restB, ...inA].sort((x, y) => x.dep - y.dep);
  tidy(m);
  return true;
}

/** An edit of the draft of the day shown: kept for Undo, its refusal said (`edit_rows`). */
function edit(wd, label, f) {
  const st = dayState(wd);
  const before = clone(modelOf(wd));
  const m = clone(before);
  const was = derive(before, wd).figures;
  const r = f(m);
  if (r !== true) return said(typeof r === 'string' ? r : 'That cannot be done.', true);
  st.undo.push({ model: before, label });
  if (st.undo.length > 60) st.undo.shift();
  st.redo = [];
  st.draft = m;
  st.before = was;
  edits++;
  version++;
  return { edited: true };
}

/** The duty row `row`'s id on the day as it is shown. */
const dutyAt = (wd, row) => derive(modelOf(wd), wd).shaped[row];

function noteEdit(wd, args) {
  const s = dutyAt(wd, Number(args.row));
  if (!s) return said('That duty is gone.', true);
  const field = String(args.field || '');
  return edit(wd, `Duty ${s.label}: ${field.replace(/_/g, ' ')}`, (m) => {
    if (s.spare != null) {
      const sp = m.spares.find((x) => x.id === s.spare);
      if (sp && field === 'custom_time') return Object.assign(sp, { from: args.from, to: args.to }) && true;
      if (sp && field === 'sign_on_at') return ((sp.sign_on_at = args.value ?? null), true);
    }
    const n = (m.notes[s.id] ||= {});
    switch (field) {
      case 'kind':
        if (args.value) n.kind = Number(args.value);
        else delete n.kind;
        break;
      case 'paid':
        n.paid = Number(args.value);
        break;
      case 'paid_auto':
        delete n.paid;
        break;
      case 'sign_on':
      case 'sign_off':
        n[field] = Number(args.value);
        break;
      case 'sign_on_at':
      case 'sign_off_at':
        if (args.value == null) delete n[field];
        else n[field] = args.value;
        break;
      case 'travel':
      case 'break_at':
        (n[field] ||= {})[Number(args.gap) || 0] = args.value;
        break;
      case 'custom':
        (n.custom ||= []).push({ kind: Number(args.kind) || 0, from: Number(args.from), to: Number(args.to) });
        break;
      case 'custom_time': {
        const cw = (n.custom || [])[Number(args.ci)];
        if (!cw) return 'That custom work is gone.';
        Object.assign(cw, { from: Number(args.from), to: Number(args.to) });
        break;
      }
      default:
        return true;
    }
    return true;
  });
}

function act(args) {
  const what = String(args.act || '');
  const day = Number(args.day) || 0;
  const wd = wdOf(day);
  const st = dayState(wd);
  const d = derive(modelOf(wd), wd);
  const blk = (i) => d.blocks[Number(i)];
  switch (what) {
    case 'undo':
    case 'redo': {
      const from = what === 'undo' ? st.undo : st.redo;
      const to = what === 'undo' ? st.redo : st.undo;
      const item = from.pop();
      if (!item) return {};
      to.push({ model: clone(modelOf(wd)), label: item.label });
      st.draft = item.model;
      version++;
      return said(`${what === 'undo' ? 'Undone' : 'Redone'}: ${item.label}`);
    }
    case 'back_to': {
      const k = Number(args.k);
      if (!(k >= 0 && k < st.undo.length)) return {};
      const item = st.undo[k];
      st.redo = st.undo.slice(k).reverse().map((u) => ({ model: u.model, label: u.label }));
      st.undo = st.undo.slice(0, k);
      st.draft = item.model;
      version++;
      return said(`Back to before: ${item.label}`);
    }
    case 'save': {
      const n = args.apply?.targets?.length || 0;
      for (const x of Object.values(days)) if (x.draft) [x.saved, x.draft] = [x.draft, null];
      if (n) {
        const src = clone(dayState(wd).saved);
        for (const t of args.apply.targets) {
          const w = wdOf(t.k);
          if (groupOf(w) === groupOf(wd) && w !== wd) dayState(w).saved = clone(src);
        }
      }
      st.undo = [];
      st.redo = [];
      version++;
      return said(n ? `Saved: the company runs this planning, and it is applied to ${n} more days.` : 'Saved: the company runs this planning.');
    }
    case 'discard':
      for (const x of Object.values(days)) {
        x.draft = null;
        x.undo = [];
        x.redo = [];
        x.before = null;
      }
      version++;
      return said('Discarded: the planning is as it was saved.');
    case 'day_reset':
      return said('This day follows no other day: there is nothing to reset it to.', true);
    case 'day_unfollow':
      return said('This day follows no other day.', true);
    case 'copy_rows': {
      const n = (args.targets || []).length;
      version++;
      return { ...said(n ? `Copied ${(args.ids || []).length} rows to ${n} days: every trip matched.` : 'Choose a day to copy to.', !n), edited: n > 0 };
    }
    case 'revalidate': {
      version++;
      const n = d.issues.filter((i) => !i.dismissed).length;
      return said(n ? `Validated: ${n} issues open.` : 'Validated: every rule is kept.');
    }
    case 'optimise':
      if (!prefs.optimise_vehicles && !prefs.optimise_duties) return said('Optimise makes neither the vehicles nor the duties now: switch one on in the Optimisation settings (the gear beside OPTIMIZE).', true);
      proposals[wd] = { better: false, note: 'No better plan found: your planning already costs the least by the preferences.' };
      version++;
      return { ...said(proposals[wd].note), proposal: true };
    case 'accept':
    case 'keep_mine':
      delete proposals[wd];
      version++;
      return {};
    case 'prefs':
      prefs = { ...defaultPrefs(), ...(args.prefs || {}) };
      version++;
      return { ...said('Preferences saved: the issues and Optimise follow them.'), offer: null };
    case 'opt_flags':
      if (args.restore) Object.assign(prefs, { optimise_vehicles: true, optimise_duties: true });
      if (args.vehicles != null) prefs.optimise_vehicles = !!args.vehicles;
      if (args.duties != null) prefs.optimise_duties = !!args.duties;
      version++;
      return {};
    case 'colour': {
      const l = lineOf(args.line);
      if (l) colours[l.name] = args.colour || PALETTE[LINES.indexOf(l) % PALETTE.length];
      version++;
      return {};
    }
    case 'colour_by':
      colourBy = ['line', 'trip', 'block', 'duty'].includes(args.by) ? args.by : 'line';
      version++;
      return {};
    case 'dismiss': {
      const keys = args.keys || [];
      const m = modelOf(wd);
      m.dismissed = args.on === false ? m.dismissed.filter((k) => !keys.includes(k)) : [...new Set([...m.dismissed, ...keys])];
      version++;
      return said(args.on === false ? `${keys.length} issues restored.` : `${keys.length} issues dismissed.`);
    }
    case 'pick': {
      const h = args.hand || {};
      if (h.kind === 'trips' || h.kind === 'swap') {
        const why = lockedBlock(modelOf(wd), h.tour);
        if (why) return { refused: true, ...said(why, true) };
      }
      if (h.kind === 'piece' || h.kind === 'duty_trips') {
        const row = h.kind === 'duty_trips' ? h.duty : d.shaped.findIndex((s) => s.spans.some((sp) => sp.tour === h.tour && sp.duty === h.duty));
        if (d.shaped[row]?.locked) return { refused: true, ...said(`Duty ${d.shaped[row].label} is locked: unlock it first.`, true) };
      }
      return {};
    }
    case 'drop': {
      const h = args.hand || {};
      const z = args.zone || {};
      if (h.kind === 'trips') {
        if (z.kind === 'stack') return edit(wd, `Unassigned ${h.trips.length} trips of ${blockLabel(blk(h.tour))}`, (m) => unassign(m, h.tour, h.trips));
        return edit(wd, `Moved ${h.trips.length} trips of ${blockLabel(blk(h.tour))} to ${z.kind === 'block' ? blockLabel(blk(z.i)) : 'a new vehicle'}`, (m) => moveTrips(m, h.tour, h.trips, z.kind === 'block' ? z.i : null));
      }
      if (h.kind === 'stack') return edit(wd, `Assigned a trip to ${z.kind === 'block' ? blockLabel(blk(z.i)) : 'a new vehicle'}`, (m) => assign(m, h.k, z.kind === 'block' ? z.i : null));
      if (h.kind === 'piece') {
        if (z.kind === 'duty') return edit(wd, `Joined a piece of ${blockLabel(blk(h.tour))} to duty ${d.shaped[z.d]?.label}`, (m) => joinPiece(m, d, h.tour, h.duty, z.d));
        return edit(wd, `A piece of ${blockLabel(blk(h.tour))} a duty of its own`, (m) => detach(m, d, h.tour, h.duty));
      }
      if (h.kind === 'duty_trips') return edit(wd, `Moved ${(h.trips || []).length} trips to ${z.kind === 'duty' ? `duty ${d.shaped[z.d]?.label}` : 'a new duty'}`, (m) => moveToDuty(m, d, h, z.kind === 'duty' ? z.d : null));
      return said('Nothing is in hand.', true);
    }
    case 'move':
      return edit(wd, `Moved ${(args.trips || []).length} trips of ${blockLabel(blk(args.tour) || {})} to ${args.to != null ? blockLabel(blk(args.to)) : 'a new vehicle'}`, (m) => moveTrips(m, Number(args.tour), args.trips || [], args.to == null ? null : Number(args.to)));
    case 'unassign':
    case 'unassign_key':
      return edit(wd, `Unassigned ${(args.trips || []).length} trips of ${blockLabel(blk(args.tour) || {})}`, (m) => unassign(m, Number(args.tour), args.trips || []));
    case 'unassign_rows':
    case 'unassign_duties': {
      const ids = args.ids || [];
      return edit(wd, `Unassigned the trips of ${ids.length} rows`, (m) => {
        const pairs = [];
        for (const id of ids) {
          if (String(id).startsWith('B|')) {
            const i = d.blocks.findIndex((b) => `B|${b.line}|${b.tour}` === id);
            if (i >= 0) d.blocks[i].trips.forEach((x, j) => !x.empty && pairs.push([blockKey(d.blocks[i]), x.dep]));
          } else {
            const s = d.shaped.find((x) => x.id === id);
            s?.spans.forEach((sp) => sp.trips.forEach((x) => !x.empty && pairs.push([blockKey(d.blocks[sp.tour]), x.dep])));
          }
        }
        if (!pairs.length) return 'Nothing to unassign.';
        for (const b of m.blocks) {
          if (m.blockLocks.includes(blockKey(b))) continue;
          const out = b.trips.filter((x) => !x.empty && pairs.some(([k, dep]) => k === blockKey(b) && dep === x.dep));
          b.trips = b.trips.filter((x) => !out.includes(x));
          m.stack.push(...out);
        }
        m.stack.sort((a, z) => a.dep - z.dep);
        tidy(m);
        return true;
      });
    }
    case 'cut': {
      const b = blk(args.tour);
      const x = b?.trips[Number(args.at)];
      if (!x) return said('That trip is gone.', true);
      return edit(wd, `Duty cut on ${blockLabel(b)} at ${clock(x.dep)}`, (m) => {
        const why = lockedBlock(m, Number(args.tour));
        if (why) return why;
        m.blocks[Number(args.tour)].cuts.push(x.dep);
        return true;
      });
    }
    case 'uncut': {
      const b = blk(args.tour);
      const x = b?.trips[Number(args.at)];
      if (!x) return said('That trip is gone.', true);
      return edit(wd, `Duties joined on ${blockLabel(b)} at ${clock(x.dep)}`, (m) => {
        const why = lockedBlock(m, Number(args.tour));
        if (why) return why;
        const mb = m.blocks[Number(args.tour)];
        mb.cuts = mb.cuts.filter((c) => c !== x.dep);
        return true;
      });
    }
    case 'cut_mid':
      return said('A relief on the way is cut by the app: the browser mock has none.', true);
    case 'detach':
      return edit(wd, `A piece of ${blockLabel(blk(args.tour) || {})} a duty of its own`, (m) => detach(m, d, Number(args.tour), Number(args.duty)));
    case 'link':
    case 'unlink': {
      const b = blk(args.tour);
      if (!b) return said('That vehicle is gone.', true);
      if (what === 'link') {
        const xs = (args.trips || []).map((j) => b.trips[j]).filter((x) => x && !x.empty);
        if (xs.length < 2) return said('Pick two trips or more to link.', true);
        return edit(wd, `Linked ${xs.length} trips of ${blockLabel(b)}`, (m) => ((m.linked = [...new Set([...m.linked, ...xs.map(tripKey)])]), true));
      }
      return edit(wd, `Unlinked the trips of ${blockLabel(b)}`, (m) => ((m.linked = m.linked.filter((k) => !b.trips.some((x) => tripKey(x) === k))), true));
    }
    case 'lock_block': {
      const b = blk(args.tour);
      if (!b) return said('That vehicle is gone.', true);
      const k = blockKey(b);
      const on = args.on ?? !modelOf(wd).blockLocks.includes(k);
      return edit(wd, `Vehicle ${blockLabel(b)} ${on ? 'locked' : 'unlocked'}`, (m) => ((m.blockLocks = on ? [...new Set([...m.blockLocks, k])] : m.blockLocks.filter((x) => x !== k)), true));
    }
    case 'lock_duty': {
      const s = d.shaped[Number(args.row)];
      if (!s) return said('That duty is gone.', true);
      const on = args.on ?? !s.locked;
      return edit(wd, `Duty ${s.label} ${on ? 'locked' : 'unlocked'}`, (m) => {
        if (s.spare != null) {
          const sp = m.spares.find((x) => x.id === s.spare);
          if (sp) sp.locked = on;
        } else m.dutyLocks = on ? [...new Set([...m.dutyLocks, s.id])] : m.dutyLocks.filter((x) => x !== s.id);
        return true;
      });
    }
    case 'lock_rows': {
      const ids = args.ids || [];
      const on = !!args.on;
      return edit(wd, `${ids.length} rows ${on ? 'locked' : 'unlocked'}`, (m) => {
        for (const id of ids) {
          if (String(id).startsWith('B|')) {
            const k = String(id).slice(2);
            m.blockLocks = on ? [...new Set([...m.blockLocks, k])] : m.blockLocks.filter((x) => x !== k);
          } else m.dutyLocks = on ? [...new Set([...m.dutyLocks, id])] : m.dutyLocks.filter((x) => x !== id);
        }
        return true;
      });
    }
    case 'swap': {
      const a = blk(args.tour);
      const b = blk(args.to);
      if (!a || !b) return said('That vehicle is gone.', true);
      return edit(wd, `Swapped trips of ${blockLabel(a)} and ${blockLabel(b)}`, (m) => swap(m, Number(args.tour), args.trips || [], Number(args.to), args.how));
    }
    case 'note':
      return noteEdit(wd, args);
    case 'custom_delete': {
      const s = d.shaped[Number(args.row)];
      if (!s) return said('That duty is gone.', true);
      return edit(wd, `Duty ${s.label}: custom work deleted`, (m) => {
        if (s.spare != null) {
          m.spares = m.spares.filter((x) => x.id !== s.spare);
          return true;
        }
        const n = m.notes[s.id];
        if (!n?.custom?.[Number(args.ci)]) return 'That custom work is gone.';
        n.custom.splice(Number(args.ci), 1);
        return true;
      });
    }
    case 'travel_event': {
      const s = d.shaped[Number(args.row)];
      if (!s) return said('That duty is gone.', true);
      return edit(wd, `Duty ${s.label}: ${TRAVEL_LABELS[Number(args.mode)] || 'travel'}`, (m) => (((m.notes[s.id] ||= {}).travelEv = Number(args.mode) || 0), true));
    }
    case 'spare': {
      const from = args.from ?? 480;
      const to = args.to ?? 960;
      return edit(wd, 'A spare line added', (m) => {
        const id = Math.max(0, ...m.spares.map((x) => x.id)) + 1;
        m.spares.push({ id, name: '', from, to, kind: 0, locked: false });
        return true;
      });
    }
    case 'spare_name':
    case 'spare_remove': {
      const s = d.shaped[Number(args.row)];
      if (!s || s.spare == null) return said('That spare line is gone.', true);
      return edit(wd, what === 'spare_name' ? `Spare line ${s.label} named` : `Spare line ${s.label} deleted`, (m) => {
        if (what === 'spare_remove') m.spares = m.spares.filter((x) => x.id !== s.spare);
        else {
          const sp = m.spares.find((x) => x.id === s.spare);
          if (sp) sp.name = String(args.value || '');
        }
        return true;
      });
    }
    default:
      return { ok: true };
  }
}

// --- the Roster tab (`rosterflow/roster.rs`) ------------------------------------------------------------

const DRIVER_COLOURS = ['#6ea8e6', '#60c4aa', '#c496d6', '#e2b86e', '#8cbe78', '#e29682', '#96cef0', '#c8c878', '#b4a0e6', '#78c8d2'];
const staffColour = (id) => DRIVER_COLOURS[Math.max(0, STAFF.findIndex((e) => e.id === id)) % DRIVER_COLOURS.length];
const firstName = (id) => (STAFF.find((e) => e.id === id)?.name || '?').split(' ')[0];
const whoKey = (w) => JSON.stringify(w ?? null);
const whoName = (w) => (w == null ? 'Nobody' : w === 'player' ? 'You' : w === 'agency' ? 'Agency driver' : STAFF.find((e) => e.id === w.staff)?.name || '?');

const roster = { byWd: {}, undo: [], redo: [], folded: [], service: Object.fromEntries(LINES.map((l) => [l.name, true])) };

/** What the roster fixes for each weekday: each tour's bus and its duties' drivers. */
function rosterOf(wd) {
  if (!roster.byWd[wd]) {
    const s = (id) => ({ staff: id });
    // (bus 2571 is in the workshop: the peak tour 101/02 has no bus on weekdays)
    roster.byWd[wd] =
      wd < 5
        ? { '101|01': { bus: 1, duties: [s(1), s(2)] }, '101|02': { bus: null, duties: [s(3), s(3)] }, '102|01': { bus: 3, duties: [s(4), null] }, '205|01': { bus: 2, duties: [s(5), null] } }
        : wd === 5
          ? { '101|01': { bus: 1, duties: [s(2), s(3)] }, '102|01': { bus: 3, duties: [s(4), s(1)] }, '205|01': { bus: 2, duties: [s(5), null] } }
          : { '101|01': { bus: 1, duties: [s(1), s(2)] }, '205|01': { bus: 2, duties: [s(3), s(5)] } };
  }
  return roster.byWd[wd];
}

/** The day's plan (`DayPlan`): the saved planning's tours, their buses and duties' drivers. */
function planOf(day) {
  const date = dateOf(day);
  const wd = weekdayOf(date);
  const d = derive(dayState(wd).saved, wd);
  const fixed = rosterOf(wd);
  const tours = d.blocks.map((b, ti) => {
    const f = fixed[blockKey(b)] || { bus: null, duties: [] };
    return {
      ti, line: b.line, tour: b.tour, number: lineOf(b.line).number, bus: f.bus ?? null, from: b.trips[0].dep, to: b.trips[b.trips.length - 1].arr,
      km: b.trips.reduce((a, x) => a + x.km, 0),
      duties: b.pieces.map(([s, e], k) => ({ k, from: b.trips[s].dep, to: b.trips[e - 1].arr, who: f.duties[k] ?? null, n: e - s, a: b.trips[s].from, z: b.trips[e - 1].to, rf: d.shaped.find((x) => x.spans.some((sp) => sp.tour === ti && sp.duty === k))?.id || null })),
    };
  });
  return { date, wd, d, tours, today: (Number(day) || 0) === 0 };
}

const workOf = (pl, w) => pl.tours.flatMap((t) => t.duties.filter((x) => whoKey(x.who) === whoKey(w)).map((x) => ({ ...x, line: t.line, tour: t.tour, number: t.number, ti: t.ti })));

/** What driver `w` would make of duty `k` of tour `ti` (`fit_of`): words, tone, whether he can. */
function fitOf(pl, ti, k, w) {
  const x = pl.tours[ti].duties[k];
  const other = workOf(pl, w).filter((y) => !(y.ti === ti && y.k === k));
  const clash = other.find((y) => x.from < y.to + 30 && y.from < x.to + 30);
  if (clash) return [`Drives line ${clash.number} tour ${clash.tour} then (${clock(clash.from)}–${clock(clash.to)})`, 'err', false];
  const lo = Math.min(x.from, ...other.map((y) => y.from));
  const hi = Math.max(x.to, ...other.map((y) => y.to));
  if (hi - lo > 660) return [`Working day would be ${length(hi - lo)}: ${length(660)} at most`, 'err', false];
  return other.length ? ['Can take it', 'ok', true] : ['Can take it: free all day', 'ok', true];
}

function rosterDays() {
  return [0, 1, 2, 3, 4, 5, 6].map((k) => {
    const date = addDays(COMPANY.date, k);
    const pl = planOf(k);
    const open = pl.tours.reduce((a, t) => a + (t.bus == null ? 1 : 0) + t.duties.filter((x) => x.who == null).length, 0);
    return {
      caps: (k === 0 ? 'Company day' : WD[weekdayOf(date)]).toUpperCase(),
      big: `${WD[weekdayOf(date)]} ${+date.slice(8, 10)} ${MONTHS[+date.slice(5, 7) - 1]}`,
      status: pl.tours.length ? { tone: open ? 'warn' : 'ok', tip: open ? `${open} open` : 'Every tour planned' } : { tone: 'faint', tip: 'No tours this day' },
    };
  });
}

function tourJson(pl, t) {
  const v = FLEET.find((f) => f.id === t.bus);
  const busTip = "The tour's bus: tap a bus in the list, then here - or drag it here. Tap it twice: the line's timetable";
  const bus = v
    ? { kind: 'bus', text: `${v.number}  ${v.name}`, suggested: false, problem: null, tip: busTip, id: v.id, off: true }
    : { kind: 'none', tone: 'warn', icon: 'warning', text: 'No bus', tip: busTip, off: false };
  const duties = t.duties.map((x) => {
    const times = `${clock(x.from)} – ${clock(x.to)}`;
    const faint = t.bus == null ? 0.5 : 1;
    let v2;
    if (x.who == null) v2 = { open: true, tone: 'warn', icon: 'warning', words: 'No driver', full: `No driver  ${times}` };
    else {
      const staff = x.who.staff != null ? STAFF.find((e) => e.id === x.who.staff) : null;
      const [fill, ink, label, short] = staff
        ? [staffColour(staff.id), '#0e121c', firstName(staff.id), staff.name.split(' ').map((w) => w[0]).join('').toUpperCase()]
        : x.who === 'player' ? ['accent', '#ffffff', 'You', ''] : ['grey', '#ffffff', 'Agency', ''];
      v2 = { open: false, fill, ink, alpha: 1, label, short, full: `${label}  ${times}`, outline: false, empty: pl.d.blocks[t.ti].trips.slice(pl.d.blocks[t.ti].pieces[x.k][0], pl.d.blocks[t.ti].pieces[x.k][1]).filter((y) => y.empty).map((y) => [Math.max(y.dep, x.from), Math.min(y.arr, x.to)]) };
    }
    return { ...v2, from: x.from, to: x.to, faint, late: null, warn: false, tip: `${times}  ·  ${whoName(x.who)}`, who: x.who, off: x.who != null, rf: x.rf };
  });
  return { line: t.line, tour: t.tour, number: t.number, title: `Tour ${t.tour}`, colour: colourOf(t.line), unplanned: false, by_player: false, bus, duties };
}

function planJson(pl) {
  const t0 = Math.floor(Math.min(300, ...pl.tours.map((t) => t.from)) / 60) * 60;
  const t1 = Math.ceil(Math.max(1440, ...pl.tours.map((t) => t.to)) / 60) * 60;
  const groups = LINES.map((l) => {
    const idx = pl.tours.filter((t) => eq(t.line, l.name)).map((t) => t.ti);
    const none = !idx.length;
    const covered = idx.filter((i) => pl.tours[i].bus != null && pl.tours[i].duties.every((x) => x.who != null)).length;
    const open = idx.reduce((a, i) => a + (pl.tours[i].bus != null ? pl.tours[i].duties.filter((x) => x.who == null).length : 0), 0);
    const noBus = idx.filter((i) => pl.tours[i].bus == null).length;
    const tags = [];
    if (noBus) tags.push([`${noBus} without a bus`, 'directions_bus']);
    if (open) tags.push([`${open} without a driver`, 'person']);
    const service = none
      ? { kind: 'none', text: 'No tours this day' }
      : roster.service[l.name]
        ? { kind: 'running', text: `In service since Mon 2 Mar  ·  ${covered} of ${idx.length} tours covered`, all: covered === idx.length }
        : { kind: 'start', button: 'Start service…', note: `Not in service · ${covered} of ${idx.length} tours planned` };
    return { name: l.name, idx, none, idle: false, folded: roster.folded.includes(l.name), line: { name: l.name, number: l.number, caption: l.caption, colour: colourOf(l.name) }, tags, service };
  });
  return { t0, t1: Math.max(t1, t0 + 60), groups, tours: pl.tours.map((t) => tourJson(pl, t)), today: pl.today, empty: false };
}

function daySide(pl) {
  const tours = pl.tours.length;
  const covered = pl.tours.filter((t) => t.bus != null && t.duties.every((x) => x.who != null)).length;
  const open = pl.tours.reduce((a, t) => a + (t.bus != null ? t.duties.filter((x) => x.who == null).length : 0), 0);
  const noBus = pl.tours.filter((t) => t.bus == null).length;
  const openRows = [];
  for (const t of pl.tours) {
    if (t.bus == null) openRows.push({ tour: t.ti, duty: null, line: t.line, name: t.tour, title: `Line ${t.number}  ·  Tour ${t.tour}  ·  No bus`, fill: '→ Trips dropped', tone: 'warn', hard: false });
    t.duties.forEach((x) => x.who == null && openRows.push({ tour: t.ti, duty: x.k, line: t.line, name: t.tour, title: `Line ${t.number}  ·  Tour ${t.tour}  ·  Duty ${x.k + 1}  ·  No driver`, fill: '→ Trips dropped', tone: 'warn', hard: false }));
  }
  return {
    kind: 'day',
    title: (pl.today ? "The company's day" : 'This day').toUpperCase(),
    summary: `${covered} of ${tours} tours covered`,
    ok: open + noBus === 0,
    gaps: open + noBus === 0 ? 'Nothing open.' : `${open} duties without a driver, ${noBus} tours without a bus.`,
    morning: [],
    on_way: [],
    on_way_caps: 'ON THE WAY TODAY',
    open_rows: openRows,
    staff: STAFF.map((e) => {
      const mins = workOf(pl, { staff: e.id }).reduce((a, x) => a + x.to - x.from, 0);
      const [state, tone] = mins === 0 ? ['Free all day', 'ok'] : [length(mins), mins > 480 ? 'warn' : 'soft'];
      return { id: e.id, name: e.name, colour: staffColour(e.id), first: firstName(e.id), state, tone };
    }),
    guests: [],
    buses: FLEET.map((v) => {
      const n = pl.tours.filter((t) => t.bus === v.id).length;
      const [state, tone] = v.workshop ? ['In the workshop', 'err'] : n ? [`${n} tours`, 'soft'] : ['Free', 'dim'];
      return { id: v.id, name: `${v.number}  ${v.name}`, number: v.number, state, tone };
    }),
  };
}

function dutySide(pl, ti, k) {
  const t = pl.tours[ti];
  const x = t.duties[k];
  const facts = [[`${clock(x.from)} – ${clock(x.to)}  ·  ${length(x.to - x.from)}  ·  ${x.n} trips`, 'soft'], [`${x.a} → ${x.z}`, 'dim'], [`Driven by: ${whoName(x.who)}`, 'text']];
  if (x.who != null) facts.push(['fixed in the roster', 'dim']);
  else facts.push(['Open because: No driver', 'warn']);
  const cands = [
    { who: 'player', name: 'You', state: 'Drive it yourself', tone: 'dim', swatch: 'accent', on: x.who === 'player' },
    { who: 'agency', name: 'Agency driver', state: `${eur(Math.round(((x.to - x.from) / 60) * 3200))} for this duty, every ${WEEKDAYS[pl.wd]}`, tone: 'dim', swatch: 'grey', on: x.who === 'agency' },
    ...STAFF.map((e) => {
      const [state, tone] = fitOf(pl, ti, k, { staff: e.id });
      return { who: { staff: e.id }, name: e.name, state, tone, swatch: staffColour(e.id), on: whoKey(x.who) === whoKey({ staff: e.id }) };
    }),
    { who: null, name: 'Nobody', state: '', tone: 'dim', swatch: null, on: false },
  ];
  return {
    kind: 'duty', title: 'DUTY', ti, k, line: t.line, tour: t.tour, colour: colourOf(t.line), head: `Line ${t.number}  ·  Tour ${t.tour}  ·  Duty ${k + 1}`,
    facts, off: x.who != null ? `Unassign ${whoName(x.who)}` : null, options: [], cands, by_player: false, drive: null,
  };
}

function tourSide(pl, ti) {
  const t = pl.tours[ti];
  const v = FLEET.find((f) => f.id === t.bus);
  const facts = [[`${clock(t.from)} – ${clock(t.to)}  ·  ${Math.round(t.km)} km  ·  ${t.duties.length} duties`, 'soft'], [`Bus: ${v ? `${v.number} ${v.name}` : 'No bus'}`, v ? 'text' : 'warn']];
  if (v) facts.push(['fixed in the roster', 'dim']);
  const cands = FLEET.map((f) => {
    const other = pl.tours.find((o) => o.ti !== ti && o.bus === f.id && t.from < o.to + 10 && o.from < t.to + 10);
    const [state, tone, ok] = f.workshop ? ['In the workshop', 'err', false] : other ? [`On tour ${o2(other)} then`, 'warn', false] : ['Free', 'ok', true];
    return [ok, { bus: f.id, name: `${f.number}  ${f.name}`, state, tone, on: t.bus === f.id }];
  }).sort((a, b) => b[0] - a[0]).map((x) => x[1]);
  cands.push({ bus: null, name: 'No bus', state: '', tone: 'dim', on: false });
  return {
    kind: 'tour', title: 'TOUR', ti, line: t.line, tour: t.tour, colour: colourOf(t.line), head: `Line ${t.number}  ·  Tour ${t.tour}`, facts, options: [],
    off: v ? `Unassign bus ${v.number}` : null, sends: [], sends_caps: 'SEND A BUS FOR IT', cands, by_player: false,
  };
}
const o2 = (t) => `${t.number}/${t.tour}`;

function rosterView(args) {
  const day = Math.max(0, Math.min(6, Number(args.day) || 0));
  const pl = planOf(day);
  const unsaved = !!dayState(pl.wd).draft;
  const tools = unsaved
    ? { text: 'RosterFlow has changes of this day not saved: this tab and the day run the saved duties. Save them to plan with them here.', tone: 'warn', icon: 'warning' }
    : { text: `${dayLabel(pl.date)}: the roster of this weekday repeats every week.`, tone: 'soft', icon: 'event' };
  const sel = args.sel || null;
  let side = daySide(pl);
  if (sel?.duty) {
    const [line, tour, k] = sel.duty;
    const t = pl.tours.find((x) => x.line === line && x.tour === tour);
    if (t && t.duties[k]) side = dutySide(pl, t.ti, k);
  } else if (sel?.tour) {
    const t = pl.tours.find((x) => x.line === sel.tour[0] && x.tour === sel.tour[1]);
    if (t) side = tourSide(pl, t.ti);
  }
  return { date: pl.date, days: rosterDays(), tools, repeat: pl.wd < 5 ? 'Repeat Mon–Fri' : 'Repeat on the weekend', undo: roster.undo.length, redo: roster.redo.length, plan: planJson(pl), outcomes: null, side };
}

/** A change of the roster: what it was kept for Undo. */
function rosterChange(f) {
  const before = clone(roster.byWd);
  const r = f();
  if (r && r.say?.error) return r;
  roster.undo.push(before);
  roster.redo = [];
  version++;
  return r || {};
}

function rosterAct(args) {
  const what = String(args.act || '');
  const day = Math.max(0, Math.min(6, Number(args.day) || 0));
  const pl = planOf(day);
  const fixed = rosterOf(pl.wd);
  const entry = (t) => (fixed[`${t.line}|${t.tour}`] ||= { bus: null, duties: [] });
  const tourOf = (line, tour) => pl.tours.find((t) => t.line === line && t.tour === tour);
  /** The pieces joined with duty `k` of tour `ti` in the studio go to the same driver. */
  const partners = (ti, k) => {
    const s = pl.d.shaped.find((x) => x.spans.some((sp) => sp.tour === ti && sp.duty === k));
    return s ? s.spans.map((sp) => [sp.tour, sp.duty]) : [[ti, k]];
  };
  switch (what) {
    case 'undo':
    case 'redo': {
      const from = what === 'undo' ? roster.undo : roster.redo;
      const to = what === 'undo' ? roster.redo : roster.undo;
      const s = from.pop();
      if (!s) return {};
      to.push(clone(roster.byWd));
      roster.byWd = s;
      version++;
      return said(what === 'undo' ? 'Undone: the roster as it was.' : 'Redone.');
    }
    case 'give_driver':
      return rosterChange(() => {
        for (const [ti, k] of partners(Number(args.ti), Number(args.k))) entry(pl.tours[ti]).duties[k] = args.who ?? null;
      });
    case 'give_bus':
      return rosterChange(() => {
        entry(pl.tours[Number(args.ti)]).bus = args.bus ?? null;
      });
    case 'off_driver':
      return rosterChange(() => {
        const t = tourOf(args.line, args.tour);
        if (!t) return said('That tour is gone.', true);
        const who = t.duties[Number(args.k)]?.who;
        for (const [ti, k] of partners(t.ti, Number(args.k))) entry(pl.tours[ti]).duties[k] = null;
        return said(`${whoName(who)} taken off the duty: it is open.`);
      });
    case 'off_bus':
      return rosterChange(() => {
        const t = tourOf(args.line, args.tour);
        if (!t) return said('That tour is gone.', true);
        entry(t).bus = null;
        return said('The bus is taken off the tour.');
      });
    case 'off_many':
      return rosterChange(() => {
        for (const it of args.items || []) {
          if (it.duty) {
            const t = tourOf(it.duty[0], it.duty[1]);
            if (t) entry(t).duties[it.duty[2]] = null;
          } else if (it.tour) {
            const t = tourOf(it.tour[0], it.tour[1]);
            if (t) entry(t).bus = null;
          }
        }
        return said(`${(args.items || []).length} taken off.`);
      });
    case 'off_staff':
      return rosterChange(() => {
        for (const t of pl.tours) t.duties.forEach((x) => whoKey(x.who) === whoKey(args.who) && (entry(t).duties[x.k] = null));
        return said(`${whoName(args.who)} taken off every duty of this day.`);
      });
    case 'off_vehicle':
      return rosterChange(() => {
        for (const t of pl.tours) if (t.bus === args.bus) entry(t).bus = null;
        return said('The bus is taken off every tour of this day.');
      });
    case 'repeat':
      return rosterChange(() => {
        const range = pl.wd < 5 ? [0, 1, 2, 3, 4] : [5, 6];
        for (const w of range) if (w !== pl.wd) roster.byWd[w] = clone(fixed);
        return said(pl.wd < 5 ? `The roster of ${WEEKDAYS[pl.wd]} is on Monday to Friday now.` : `The roster of ${WEEKDAYS[pl.wd]} is on the weekend now.`);
      });
    case 'clear':
      return rosterChange(() => {
        roster.byWd[pl.wd] = {};
        return said(`Every bus and driver of ${WEEKDAYS[pl.wd]} is out of the roster.`);
      });
    case 'fill':
      return rosterChange(() => {
        let filled = 0;
        let left = 0;
        const used = new Set(pl.tours.map((t) => t.bus).filter((b) => b != null));
        for (const t of pl.tours) {
          if (t.bus == null) {
            const v = FLEET.find((f) => !used.has(f.id) && !f.workshop);
            if (v) {
              used.add(v.id);
              t.bus = entry(t).bus = v.id;
              filled++;
            }
          }
          for (const x of t.duties) {
            if (x.who != null) continue;
            const e = STAFF.find((s) => fitOf(pl, t.ti, x.k, { staff: s.id })[2]);
            if (e) {
              x.who = entry(t).duties[x.k] = { staff: e.id };
              filled++;
            } else left++;
          }
        }
        return said(left ? `Filled ${filled}. ${left} duties still open: nobody qualified is free.` : filled ? `Filled ${filled}: every tour is planned.` : 'Nothing to fill: every tour is planned.', false);
      });
    case 'fold': {
      const l = String(args.line);
      roster.folded = roster.folded.includes(l) ? roster.folded.filter((x) => x !== l) : [...roster.folded, l];
      version++;
      return {};
    }
    case 'fold_all':
      roster.folded = args.all ? [...(args.lines || [])] : [];
      version++;
      return {};
    case 'stop':
      roster.service[args.line] = false;
      version++;
      return said(`Line ${lineOf(args.line)?.number || args.line} is out of service: its tours do not run.`);
    case 'service_start':
      roster.service[args.line] = true;
      version++;
      return { ...said(`Line ${lineOf(args.line)?.number || args.line} is in service.`), close: true };
    case 'set_fill':
    case 'break_choose':
      return said('Covering a duty for today only is the app’s: the browser mock keeps the roster.', true);
    default:
      return { ok: true };
  }
}

function serviceView(args) {
  const l = lineOf(args.line);
  if (!l) return { gone: true };
  const rows = [0, 1, 2, 3, 4, 5, 6].map((k) => {
    const pl = planOf(k);
    const tours = pl.tours.filter((t) => eq(t.line, l.name));
    const open = tours.reduce((a, t) => a + (t.bus == null ? 1 : 0) + t.duties.filter((x) => x.who == null).length, 0);
    return { day: dayLabel(pl.date), text: !tours.length ? 'No tours' : open ? `${open} open` : 'Every tour planned', tone: !tours.length ? 'dim' : open ? 'warn' : 'ok' };
  });
  const gaps = rows.some((r) => r.tone === 'warn');
  return {
    title: `Start service: line ${l.number}`,
    intro: `Line ${l.number}'s tours run from the moment you choose, as they are planned; what is not covered then is dropped with the contract's penalty.`,
    whens: ['Now', 'From tomorrow', 'From a date'],
    date: addDays(COMPANY.date, 1),
    rows,
    short_text: gaps ? 'Some of its tours are not covered yet: their trips are dropped until they are.' : null,
    gaps_label: 'Start all the same',
  };
}

// --- the Timeplan (`rosterflow/timeplan.rs`) -------------------------------------------------------------

const tp = { line: '101', drafts: {}, saved: {}, undo: [], redo: [], unsynced: [], hint: null, templates: [] };
const tpKey = (line, group) => `${line}|${group}`;

/** A line's timetable of a day group as the map has it: its running times and its trips. */
function tpFresh(line, group) {
  const L = lineOf(line);
  const m = genModel(GROUP_WD[group]);
  const trips = [];
  for (const b of m.blocks) if (eq(b.line, line)) for (const x of b.trips) if (!x.empty) trips.push({ dir: x.out ? 0 : 1, dep: x.dep, offs: {} });
  for (const x of m.stack) if (eq(x.line, line)) trips.push({ dir: x.out ? 0 : 1, dep: x.dep, offs: {} });
  trips.sort((a, z) => a.dep - z.dep);
  const bandsOf = (run) => [{ from: 0, run: [...run] }, { from: 420, run: run.map((r, i) => (i === 1 ? r + 1 : r)) }, { from: 540, run: [...run] }];
  const n = L.stops.length;
  const tps = [0, Math.floor(n / 2), n - 1];
  return { bands: [bandsOf(L.run), bandsOf([...L.run].reverse())], timepoints: [tps, tps.map((k) => n - 1 - k).sort((a, z) => a - z)], trips };
}

function tpDraft(line, group) {
  const k = tpKey(line, group);
  if (!tp.saved[k]) tp.saved[k] = tpFresh(line, group);
  if (!tp.drafts[k]) tp.drafts[k] = clone(tp.saved[k]);
  return tp.drafts[k];
}
const tpDirty = (line) => [0, 1, 2].some((g) => tp.drafts[tpKey(line, g)] && JSON.stringify(tp.drafts[tpKey(line, g)]) !== JSON.stringify(tp.saved[tpKey(line, g)]));
const stopsOf = (L, dir) => (dir === 0 ? L.stops : [...L.stops].reverse());

/** The band a departure runs by, and a trip's times at its stops. */
function bandAt(bands, dep) {
  let b = bands[0];
  for (const x of bands) if (x.from <= dep) b = x;
  return b;
}
function baseTimes(t, x) {
  const run = bandAt(t.bands[x.dir], x.dep).run;
  const out = [x.dep];
  for (const r of run) out.push(out[out.length - 1] + r);
  return out;
}
const tripTimes = (t, x) => baseTimes(t, x).map((v, s) => v + (x.offs?.[s] || 0));

function parseTime(s) {
  s = String(s).trim();
  let h;
  let m;
  const sp = s.split(/[:.h]/);
  if (sp.length === 2) [h, m] = sp.map((x) => parseInt(x.trim(), 10));
  else if (/^\d{3,}$/.test(s)) [h, m] = [parseInt(s.slice(0, -2), 10), parseInt(s.slice(-2), 10)];
  else if (/^\d{1,2}$/.test(s)) [h, m] = [parseInt(s, 10), 0];
  else return null;
  if (!(h >= 0 && h <= 36 && m >= 0 && m < 60) || h * 60 + m > 2160) return null;
  return h * 60 + m;
}

function tpView(args) {
  if (!lineOf(tp.line)) tp.line = LINES[0].name;
  const L = lineOf(tp.line);
  const day = Math.max(0, Math.min(2, Number(args.day) || 0));
  const side = Math.max(0, Math.min(1, Number(args.side) || 0));
  const tab = Math.max(0, Math.min(1, Number(args.tab) || 0));
  const t = tpDraft(L.name, day);
  const dirty = tpDirty(L.name);
  const unsynced = tp.unsynced.some((u) => eq(u[0], L.name));
  const out = {
    line: L.name,
    lines: LINES.map((l) => ({ name: l.name, label: `${l.number} · ${l.caption}` })),
    days: DAY_GROUPS,
    ready: true,
    why: null,
    probe: { ready: true, why: null, dirty, changes: tp.undo.length, trips: t.trips.length, unsynced },
    number: L.number,
    map: false,
    dirty,
    can_undo: tp.undo.length > 0,
    can_redo: tp.redo.length > 0,
    save: dirty ? 'Save' : 'Saved',
    runs: true,
    from: { show: false, label: `From ${dayLabel(addDays(COMPANY.date, 1))}` },
    has_inbound: true,
    colour: colourOf(L.name),
    issues: [],
    issues_head: `Line ${L.number} · ${DAY_GROUPS[day]}`,
    issues_label: `Line ${L.number}: 0`,
    errors: 0,
    warnings: 0,
    unsynced,
    calendar: [0, 1, 2, 3, 4, 5, 6].map((k) => {
      const date = addDays(COMPANY.date, k);
      const g = groupOf(weekdayOf(date));
      return { k, date, label: dayLabel(date), group: g, natural: g, read: true, on: g === day, odd: false, tip: `Runs the ${DAY_GROUPS[g]} timetable` };
    }),
    hint: tp.hint,
    pending: null,
    patterns: [{ code: '-', label: `-  ·  ${side === 0 ? L.b : L.a}`, stops: stopsOf(L, side).map((s) => s[0]) }],
    copy: { days: DAY_GROUPS.map((label, k) => ({ k, label, trips: tpDraft(L.name, k).trips.length, closed: false })), templates: tp.templates.filter((x) => x.line === L.name).map((x) => ({ id: x.id, name: x.name, day: x.day, day_label: DAY_GROUPS[x.day], trips: x.trips.length })) },
  };
  const stops = stopsOf(L, side);
  const n = stops.length;
  const tps = t.timepoints[side];
  const dots = stops.map(([name, code], k) => ({ name, code: String(code), timepoint: tps.includes(k) }));
  if (tab === 0) {
    const bands = t.bands[side].map((b, k) => {
      const until = t.bands[side][k + 1]?.from ?? 2160;
      void until;
      const total = b.run.reduce((a, x) => a + x, 0);
      return { from: hhmm36(b.from), from_m: b.from, run: [...b.run], texts: b.run.map(minText), total: `= ${minText(total)} m` };
    });
    out.rt = { pattern: 0, dir: side, n, dots, bands, few: null };
  } else {
    const cols = stops.map((_, k) => k).filter((k) => !args.points_only || tps.includes(k));
    const rows = t.trips.map((x, i) => [x, i]).filter(([x]) => x.dir === side).sort((a, z) => a[0].dep - z[0].dep);
    const rowsJson = rows.map(([x, i], k) => {
      const base = baseTimes(t, x);
      const times = tripTimes(t, x);
      return {
        i, dir: x.dir, code: '-', start: hhmm36(x.dep), headway: k > 0 ? minText(x.dep - rows[k - 1][0].dep) : null, n: times.length,
        cells: cols.map((s) => {
          const o = times[s] - base[s];
          return Math.abs(o) >= 0.5 ? { s, t: hhmm36(times[s]), off: o > 0 ? 1 : -1, base: `From the running times: ${hhmm36(base[s])}` } : { s, t: hhmm36(times[s]), late: times[s] > 2160 };
        }),
      };
    });
    const byHour = Array.from({ length: 24 }, (_, h) => rows.filter(([x]) => Math.floor(x.dep / 60) % 24 === h).length);
    out.tt = { dots: cols.map((k) => dots[k]), vt: 'SD', rows: rowsJson, kpis: { count: rows.length, by_pattern: [rows.length], by_hour: byHour, codes: ['-'], label: `${L.number} · ${DAY_GROUPS[day]}` } };
  }
  return out;
}

function tpEdit(line, group, f) {
  const k = tpKey(line, group);
  const t = tpDraft(line, group);
  const before = clone(t);
  const r = f(t);
  if (r !== true && r !== undefined && typeof r !== 'object') return said(typeof r === 'string' ? r : 'That cannot be done.', true);
  tp.undo.push({ k, t: before });
  tp.redo = [];
  version++;
  return typeof r === 'object' ? r : {};
}

function tpAct(args) {
  const what = String(args.act || '');
  const L = lineOf(tp.line) || LINES[0];
  const group = Math.max(0, Math.min(2, Number(args.day) || 0));
  const side = Math.max(0, Math.min(1, Number(args.side) || 0));
  const set = (t, cell, value) => {
    switch (cell?.kind) {
      case 'run': {
        const v = parseFloat(String(value).replace(',', '.'));
        if (!(v >= 0 && v <= 600)) return 'Type the minutes, such as 7';
        t.bands[side][cell.band].run[cell.seg] = Math.round(v * 10) / 10;
        return true;
      }
      case 'band_from': {
        const v = typeof value === 'number' ? value : parseTime(value);
        if (v == null) return 'Type a time, such as 06:30';
        t.bands[side][cell.band].from = v;
        t.bands[side].sort((a, z) => a.from - z.from);
        return true;
      }
      case 'start': {
        const v = typeof value === 'number' ? value : parseTime(value);
        if (v == null || !t.trips[cell.i]) return 'Type a time, such as 06:30';
        t.trips[cell.i].dep = v;
        return true;
      }
      case 'headway': {
        const v = parseFloat(String(value).replace(',', '.'));
        const x = t.trips[cell.i];
        if (!x || !(v >= 1 && v <= 600)) return 'Type the minutes, such as 15';
        const prev = t.trips.filter((y) => y.dir === x.dir && y.dep < x.dep).sort((a, z) => z.dep - a.dep)[0];
        if (prev) x.dep = prev.dep + v;
        return true;
      }
      case 'time': {
        const x = t.trips[cell.i];
        const v = typeof value === 'number' ? value : parseTime(value);
        if (!x || v == null) return 'Type a time, such as 06:30';
        if (cell.s === 0) x.dep = v;
        else x.offs = { ...x.offs, [cell.s]: v - baseTimes(t, x)[cell.s] };
        return true;
      }
      default:
        return 'Choose a cell first.';
    }
  };
  const current = (t, cell) => {
    switch (cell?.kind) {
      case 'run':
        return t.bands[side][cell.band]?.run[cell.seg] ?? 0;
      case 'band_from':
        return t.bands[side][cell.band]?.from ?? 0;
      case 'start':
        return t.trips[cell.i]?.dep ?? 0;
      case 'time':
        return t.trips[cell.i] ? tripTimes(t, t.trips[cell.i])[cell.s] : 0;
      default:
        return 0;
    }
  };
  switch (what) {
    case 'line':
      if (lineOf(args.line)) tp.line = lineOf(args.line).name;
      tp.undo = [];
      tp.redo = [];
      version++;
      return {};
    case 'apply':
      return tpEdit(L.name, group, (t) => set(t, args.cell, args.text));
    case 'nudge':
      return tpEdit(L.name, group, (t) => {
        const c = args.cell;
        if (c?.kind === 'headway') return 'Type the headway instead.';
        return set(t, c, current(t, c) + (Number(args.by) || 0));
      });
    case 'add_one':
      return tpEdit(L.name, group, (t) => {
        const x = t.trips[args.sel];
        const rows = t.trips.filter((y) => y.dir === (x ? x.dir : side)).sort((a, z) => a.dep - z.dep);
        const at = x ? rows.indexOf(x) : rows.length - 1;
        const head = at > 0 ? rows[at].dep - rows[at - 1].dep : 30;
        const dep = x ? x.dep + head : rows.length ? rows[rows.length - 1].dep + 30 : 360;
        t.trips.push({ dir: x ? x.dir : side, dep, offs: {} });
        return { sel: t.trips.length - 1 };
      });
    case 'add_many':
      return tpEdit(L.name, group, (t) => {
        const first = parseTime(args.first);
        let last = parseTime(args.last);
        const heads = String(args.headway || '').split(/[,;/ ]/).filter((x) => x.trim()).map(Number);
        if (first == null || last == null || !heads.length || heads.some((h) => !(h >= 1))) return 'Type the times as 06:00 and the headway in minutes';
        if (last < first && last + 1440 <= 2160) last += 1440;
        const ctl = Number(args.control) || 0;
        const lead = t.bands[side][0].run.slice(0, ctl).reduce((a, x) => a + x, 0);
        let n = 0;
        for (let v = first, k = 0; v <= last + 1e-3 && n < 1000; k++, n++) {
          t.trips.push({ dir: side, dep: v - lead, offs: {} });
          v += Math.max(1, heads[k % heads.length]);
        }
        return { added: n, ...said(`${n} trips added.`) };
      });
    case 'delete':
      return tpEdit(L.name, group, (t) => (t.trips[args.i] ? (t.trips.splice(args.i, 1), true) : 'That trip is gone.'));
    case 'bulk_delete':
      return tpEdit(L.name, group, (t) => {
        const drop = new Set(args.trips || []);
        t.trips = t.trips.filter((_, i) => !drop.has(i));
        return true;
      });
    case 'bulk_shift':
      return tpEdit(L.name, group, (t) => {
        for (const i of args.trips || []) if (t.trips[i]) t.trips[i].dep += Number(args.by) || 0;
        return true;
      });
    case 'bulk_revert':
      return tpEdit(L.name, group, (t) => {
        for (const i of args.trips || []) if (t.trips[i]) t.trips[i].offs = {};
        return true;
      });
    case 'bulk_pattern':
      return {};
    case 'band_add':
      return tpEdit(L.name, group, (t) => {
        const bands = t.bands[side];
        const at = args.at != null ? Number(args.at) : Math.min(2100, bands[bands.length - 1].from + 120);
        bands.push({ from: at, run: [...bandAt(bands, at).run] });
        bands.sort((a, z) => a.from - z.from);
        return { band: bands.findIndex((b) => b.from === at) };
      });
    case 'band_remove':
      return tpEdit(L.name, group, (t) => {
        const bands = t.bands[side];
        if (bands.length < 2) return 'A direction keeps one time band at least.';
        bands.splice(Number(args.k), 1);
        bands[0].from = 0;
        return true;
      });
    case 'timepoint':
      return tpEdit(L.name, group, (t) => {
        const k = Number(args.k);
        const n = L.stops.length;
        if (k === 0 || k === n - 1) return 'The first and the last stop are always timepoints.';
        const list = t.timepoints[side];
        t.timepoints[side] = list.includes(k) ? list.filter((x) => x !== k) : [...list, k].sort((a, z) => a - z);
        return true;
      });
    case 'undo':
    case 'redo': {
      const from = what === 'undo' ? tp.undo : tp.redo;
      const to = what === 'undo' ? tp.redo : tp.undo;
      const item = from.pop();
      if (!item) return {};
      to.push({ k: item.k, t: clone(tp.drafts[item.k]) });
      tp.drafts[item.k] = item.t;
      version++;
      return {};
    }
    case 'save': {
      if (!tpDirty(L.name)) return said('Nothing to save.');
      for (const g of [0, 1, 2]) {
        const k = tpKey(L.name, g);
        if (tp.drafts[k] && JSON.stringify(tp.drafts[k]) !== JSON.stringify(tp.saved[k])) {
          tp.saved[k] = clone(tp.drafts[k]);
          if (!tp.unsynced.some((u) => u[0] === L.name && u[1] === g)) tp.unsynced.push([L.name, g]);
        }
      }
      version++;
      return said(`Line ${L.number}'s timetable saved. Refresh makes its buses and duties anew (UNSYNCED).`);
    }
    case 'refresh': {
      const line = args.line ? lineOf(args.line)?.name : L.name;
      tp.unsynced = tp.unsynced.filter((u) => !eq(u[0], line));
      version++;
      return said(`Line ${lineOf(line)?.number || line}: its blocks and duties are made anew from the timetable (the browser mock keeps them as they were).`);
    }
    case 'copy_run':
      return tpEdit(L.name, group, () => {
        const src = tpDraft(L.name, group);
        for (const g of args.to || []) {
          const t = tpDraft(L.name, g);
          if (args.both) t.bands = clone(src.bands);
          else t.bands[side] = clone(src.bands[side]);
        }
        return said(`The running times of ${DAY_GROUPS[group]} are copied to ${(args.to || []).map((g) => DAY_GROUPS[g]).join(', ')}.`);
      });
    case 'copy_to':
      return tpEdit(L.name, group, () => {
        const src = tpDraft(L.name, group);
        for (const g of args.days || []) tpDraft(L.name, g).trips = clone(src.trips);
        return said(`Copied to ${(args.days || []).map((g) => DAY_GROUPS[g]).join(', ')}.`);
      });
    case 'template_save': {
      const id = Math.max(0, ...tp.templates.map((x) => x.id)) + 1;
      tp.templates.push({ id, line: L.name, name: String(args.name || `Template ${id}`), day: group, trips: clone(tpDraft(L.name, group).trips) });
      version++;
      return said('Template kept.');
    }
    case 'template_rename': {
      const x = tp.templates.find((y) => y.id === args.id);
      if (x) x.name = String(args.name || x.name);
      version++;
      return {};
    }
    case 'template_drop':
      tp.templates = tp.templates.filter((y) => y.id !== args.id);
      version++;
      return {};
    default:
      return { ok: true };
  }
}

function tpGuide(args) {
  const l = lineOf(args.line);
  if (l) tp.line = l.name;
  tp.hint = args.hint === 'map' ? "The line keeps the map's timetable: change it here if you like, save, then Refresh" : args.hint === 'new' ? 'Set the running times and departures, save, then Refresh to make the buses and duties' : null;
  return { screen: args.hint === 'new' ? 0 : 1, group: args.day != null ? groupOf(wdOf(args.day)) : null };
}

/** The Routes tab's strip (`rf_tp_route`): the Timeplan's running times of the route's direction. */
function tpRoute(args) {
  const L = lineOf(args.line);
  if (!L) return { linked: false, days: DAY_GROUPS, why: null };
  const dir = Math.max(0, Math.min(1, Number(args.dir) || 0));
  const group = Math.max(0, Math.min(2, Number(args.day) || 0));
  let sayNow = null;
  if (args.act === 'seg') {
    const m = Number(args.minutes);
    if (!(m >= 0 && m <= 600)) return said('Type the minutes, such as 7', true);
    tpEdit(L.name, group, (t) => ((t.bands[dir][0].run[Number(args.seg)] = m), true));
  } else if (args.act === 'road') sayNow = { text: `The roads' minutes are line ${L.number}'s running times of ${DAY_GROUPS[group]} now: save them in the Timeplan.`, error: false };
  const t = tpDraft(L.name, group);
  const run = t.bands[dir][0].run;
  return { linked: true, days: DAY_GROUPS, run: [...run], texts: run.map(minText), total: minText(run.reduce((a, x) => a + x, 0)), bands: t.bands[dir].length, dirty: tpDirty(L.name), side: dir, pattern: 0, closed: [false, false, false], say: sayNow };
}

// --- the answers ------------------------------------------------------------------------------------------

const answers = {
  rf_on: () => onJson(),
  rf_board: (a) => boardAnswer(a),
  rf_view: (a) => view(a),
  rf_act: (a) => act(a),
  rf_here: () => ({ here: [], told: [] }),
  rf_print: () => said('The PDF is made by the app: the browser mock prints no file.', true),
  rf_roster: (a) => rosterView(a),
  rf_roster_act: (a) => rosterAct(a),
  rf_roster_service: (a) => serviceView(a),
  rf_tp: (a) => tpView(a),
  rf_tp_act: (a) => tpAct(a),
  rf_tp_guide: (a) => tpGuide(a),
  rf_tp_route: (a) => tpRoute(a),
  rf_tp_route_act: (a) => tpRoute(a),
  rf_tut: () => ({ ...tut, lessons: [...tut.lessons] }),
  rf_tut_act: (a) => {
    switch (a.act) {
      case 'seen':
        tut.seen = true;
        return {};
      case 'left':
        tut.seen = true;
        if (a.completed) Object.assign(tut, { done: true, at: 0 });
        else tut.at = Number(a.at) || 0;
        return {};
      case 'lesson_done':
        if (a.id && !tut.lessons.includes(a.id)) tut.lessons.push(a.id);
        return {};
      case 'begin':
      case 'keep_tp':
        return {};
      case 'end':
        return said(a.undo ? 'The lesson is over: the browser mock keeps your changes.' : 'The lesson is over: your changes stay.');
      default:
        return {};
    }
  },
};

/** The answers, each on the company the bus company's mock has open now (its name and day). */
export const rosterflowAnswers = Object.fromEntries(
  Object.entries(answers).map(([cmd, f]) => [
    cmd,
    (args = {}) => {
      if (cmd !== 'rf_on' && !cmd.startsWith('rf_tut')) syncCompany();
      return f(args || {});
    },
  ]),
);
