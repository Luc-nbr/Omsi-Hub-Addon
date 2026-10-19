// The browser mock of a drive that runs (`hub_drive` … `hub_game_stop`) and its live dashboard
// (`hub_live`, hub-api's `live.rs`): a tour of line 23 from Riverside to Central Station and
// back, its bus moving along a made-up road at a speed that changes, a delay that comes and
// goes, passengers on and off. `?drive` opens the page with the drive running (a company's
// duty: `?drive=company`, a shift: `?drive=shift`, a free drive: `?drive=free`).

const params = new URLSearchParams(location.search);

/** The stops of the trip there; the way back is the same the other way round. */
const STOPS = [
  ['Riverside', 0, 0],
  ['City Mall', 4, 1300],
  ['City Hall', 7, 2500],
  ['Museum Quarter', 11, 3800],
  ['Market Square', 16, 6100],
  ['Opera', 19, 7400],
  ['Central Station', 24, 9400],
];
/** The road the bus drives, the map's metres (a stop lies along it by its metres). */
const ROAD = [[0, 0], [600, 420], [1100, 600], [1600, 1150], [2300, 1400], [3000, 2100], [3500, 2350], [4300, 2700], [4900, 3400], [5600, 3600]];
const TRIP_MIN = 26;
const LAYOVER_MIN = 8;
const FIRST_DEP = 7 * 3600 + 45 * 60;
const TRIPS = 6;

let drive = null;

function roadLength() {
  let m = 0;
  for (let i = 1; i < ROAD.length; i++) m += Math.hypot(ROAD[i][0] - ROAD[i - 1][0], ROAD[i][1] - ROAD[i - 1][1]);
  return m;
}
const ROAD_M = roadLength();
const TRIP_M = STOPS[STOPS.length - 1][2];

/** Where on the road `metres` along it (of the trip there) is. */
function at(metres) {
  let left = (Math.max(0, Math.min(metres, TRIP_M)) / TRIP_M) * ROAD_M;
  for (let i = 1; i < ROAD.length; i++) {
    const [a, b] = [ROAD[i - 1], ROAD[i]];
    const l = Math.hypot(b[0] - a[0], b[1] - a[1]);
    if (left <= l) return [a[0] + ((b[0] - a[0]) * left) / l, a[1] + ((b[1] - a[1]) * left) / l];
    left -= l;
  }
  return ROAD[ROAD.length - 1];
}

const trips = () =>
  Array.from({ length: TRIPS }, (_, k) => {
    const dep = FIRST_DEP + k * (TRIP_MIN + LAYOVER_MIN) * 60;
    const back = k % 2 === 1;
    return { line: '23', tour: '4', index: k + 1, name: back ? '23 Central-Riverside' : '23 Riverside-Central', number: '23', from: back ? 'Central Station' : 'Riverside', terminus: back ? 'Riverside' : 'Central Station', departure: dep, arrival: dep + TRIP_MIN * 60, km: TRIP_M / 1000 };
  });

/** The stops of trip `k` (from 0): name, due (s of the day), metres from its first stop. */
function stopsOf(k) {
  const t = trips()[k];
  const list = k % 2 === 1 ? [...STOPS].reverse().map(([n, m, d]) => [n, TRIP_MIN - 2 - m < 0 ? 0 : 24 - m, TRIP_M - d]) : STOPS;
  return list.map(([name, min, metres]) => ({ name, due: t.departure + min * 60, metres }));
}

/** The drive `seconds` after it began, as hub-api's `live::view` gives it. */
function view() {
  const mode = drive.mode;
  const secs = (Date.now() - drive.began) / 1000;
  // (the game's clock runs ten times as fast in the mock, so that something happens)
  const clock = FIRST_DEP + 4 * 60 + secs * 10;
  const plan = mode === 'free' ? [] : trips();
  const k = plan.findIndex((t) => clock < t.arrival + LAYOVER_MIN * 60);
  const t = plan[k];
  const stops = t ? stopsOf(k) : [];
  const delay = Math.round(90 + 110 * Math.sin(secs / 40));
  const within = t ? Math.max(0, clock - t.departure - Math.max(delay, 0)) : 0;
  const along = t ? Math.min(within / (TRIP_MIN * 60), 1) * TRIP_M : ((secs * 8) % TRIP_M);
  const next = t ? stops.findIndex((s) => s.metres > along + 5) : -1;
  const atStop = t && next > 0 && stops[next - 1].metres + 40 > along;
  const speed = t && (clock < t.departure || next < 0 || atStop) ? 0 : 28 + 18 * Math.abs(Math.sin(secs / 9));
  const back = k % 2 === 1;
  const pos = at(back ? TRIP_M - along : along);
  drive.trail.push(pos);
  if (drive.trail.length > 600) drive.trail.shift();
  if (t && next >= 0) drive.last = next;
  // (standing at a stop: that one is the next, as the game counts it)
  const nextIdx = next < 0 ? stops.length : atStop ? next - 1 : next;
  const since = t && nextIdx > 0 ? along - stops[nextIdx - 1].metres : 0;
  const behind = Math.max(delay, 0);
  const stopView = (s, i) => ({ index: i, name: s.name, due: s.due, eta: s.due + behind, minutes: Math.max(0, (s.due + behind - clock) / 60), metres: Math.max(0, s.metres - along), passed: i < nextIdx, stops: true });
  const status = !t && mode !== 'free' ? 'done' : mode === 'free' ? 'free' : clock < t.departure ? 'waiting' : next < 0 ? 'layover' : atStop ? 'at_stop' : 'in_service';
  const metres = Math.max(0, secs * 10 * 6.2);
  drive.series.push([clock, Math.round(speed * 10) / 10, t ? delay : null, 18 + Math.round(12 * Math.sin(secs / 30))]);
  if (drive.series.length > 240) drive.series.shift();
  const company = mode === 'company';
  const start = company ? FIRST_DEP - 15 * 60 : plan[0]?.departure ?? null;
  const end = company ? plan[plan.length - 1].arrival + 15 * 60 : plan[plan.length - 1]?.arrival ?? null;
  const served = Math.floor(secs / 4);
  const days = (n, km) => Array.from({ length: n }, (_, i) => (i % 3 === 1 ? 0 : Math.round(km * (0.6 + 0.4 * Math.sin(i)) * 10) / 10));
  const brk = plan.slice(0, -1).map((p, i) => [p, plan[i + 1]]).find(([a, b]) => clock < b.departure);
  return {
    running: true, live: true, plugin: true, began: drive.began / 1000, mode, status,
    company: company ? 'Stadtbus Riverside' : '', duty: company ? '23 4 2' : '', driver: 'Driver',
    clock, date: '2025-04-22',
    route: { number: mode === 'free' ? '' : '23', line: '23', tour: '4', from: t?.from ?? '', to: t?.terminus ?? '', trip: t ? k + 1 : null, trips: plan.length, free_route: null },
    vehicle: { number: 'A-284', name: 'Mercedes Citaro (Solo)', file: 'Vehicles/MB_O530/Citaro.bus' },
    start, end, elapsed: start != null ? Math.max(0, clock - start) : null,
    start_place: company ? 'Riverside Depot' : plan[0]?.from ?? '', end_place: company ? 'Riverside Depot' : plan[plan.length - 1]?.terminus ?? '',
    next_break: company ? { kind: 'break', from: 10.5 * 3600, to: 11 * 3600, at: 'Central Station' } : brk ? { kind: 'layover', from: brk[0].arrival, to: brk[1].departure, at: brk[0].terminus } : null,
    speed, delay: t ? delay : null, passengers: 18 + Math.round(12 * Math.sin(secs / 30)), fuel: 180,
    stops: stops.map(stopView).filter((s) => !s.passed).slice(0, 4),
    trip_stops: stops.map(stopView),
    plan,
    today: { metres, seconds: secs * 10, boarded: Math.floor(secs / 3), served, early: Math.floor(served / 12), late: Math.floor(served / 7), on_time: served ? ((served - Math.floor(served / 12) - Math.floor(served / 7)) / served) * 100 : null, avg_speed: secs > 6 ? 22.3 : null, jolts: 1, crashes: 0 },
    told: company
      ? [
          { id: 'message:1', kind: 'warning', title: 'Control', text: 'Traffic congestion near Market Square.', at: FIRST_DEP + 27 * 60 },
          { id: 'notice:n1', kind: 'info', title: 'Roadworks on Hauptstraße', text: 'Expect reduced speed (30 km/h).', at: FIRST_DEP + 11 * 60 },
        ]
      : [],
    brief: [],
    series: drive.series,
    map: { map: 'maps/Riverside/global.cfg', bus: pos, heading: null, trail: drive.trail, stood: STOPS.slice(0, Math.max(1, drive.last)).map(([name, , m]) => ({ name, at: at(back ? TRIP_M - m : m) })) },
    week: { now: { trips: 31, metres: 284_000, boarded: 3120, on_time: 89.5, avg_speed: 21.4 }, before: { trips: 27, metres: 251_000, boarded: 2710, on_time: 86.1, avg_speed: 20.2 }, days: days(7, 48) },
    month: { now: { trips: 122, metres: 1_102_000, boarded: 12_480, on_time: 87.2, avg_speed: 20.9 }, before: { trips: 109, metres: 980_000, boarded: 11_900, on_time: 88.0, avg_speed: 21.1 }, days: days(30, 45) },
  };
}

/** The map under the mock drive (`hub_route_map` of its map): the road it drives and a few
 * side streets, the line's route along the road and its stops, from an origin off the world's
 * corner as a real map's is. */
export function liveRouteMap() {
  const o = [1000, 2000];
  const flat = (pts) => pts.flatMap(([x, y]) => [x, y]);
  const side = [[[600, 420], [300, 1200]], [[1600, 1150], [2200, 400]], [[3000, 2100], [2600, 3000]], [[4300, 2700], [5000, 2000]], [[1100, 600], [2300, 1400]]];
  return {
    origin: o,
    roads: [{ p: flat(ROAD), w: 9, main: true }, ...side.map((r) => ({ p: flat(r), w: 6 }))].map((r) => ({ ...r, p: r.p.map((v, i) => v - o[i % 2]) })),
    route: [flat(ROAD).map((v, i) => v - o[i % 2])],
    stops: STOPS.map(([, , m], k) => ({ k, at: at(m).map((v, i) => v - o[i]) })),
    lo: [-o[0], -o[1]], hi: [5600 - o[0], 3600 - o[1]], entries: [],
  };
}

/** Start a mock drive (`mode`: company, shift, tour or free). */
export function startMockDrive(mode, emit) {
  drive = { mode, began: Date.now(), trail: [], series: [], last: 1 };
  emit('game-started', { pid: 4242, mode });
}

export const liveAnswers = (emit) => ({
  hub_live: () => (drive ? view() : { running: false }),
  hub_game_state: () => (drive ? { running: true, pid: 4242 } : { running: false }),
  hub_game_stop: () => {
    if (drive) {
      const seconds = (Date.now() - drive.began) / 1000;
      drive = null;
      setTimeout(() => emit('game-ended', { code: 0, seconds, booked: { reports: 3, trips: 1 } }), 300);
    }
    return { running: false };
  },
});

/** `?drive`: the page opens with a drive running. */
export function mockDriveAtStart(emit) {
  if (!params.has('drive')) return;
  const m = params.get('drive');
  startMockDrive(['company', 'shift', 'tour', 'free'].includes(m) ? m : 'tour', emit);
}
