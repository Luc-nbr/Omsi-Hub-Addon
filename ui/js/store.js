// The page's global state: one object, changed with `set`, read in a component with
// `useStore(select)` (the component draws again when what it selected changed), or with
// `useStore()` (it draws again on every change, as the ported pages expect).
//
// What lives here: `boot` (hub_boot's answer), `lang`, `page` (the page shown: `drive` is the
// start and the drive's steps), `game` (whether openOMSI runs). A feature's own state goes under its own key (`company`, …). The
// drive's choice and the content lists (`settings`, `choice`, `maps`, `vehicles`, `lines` …)
// are kept by `hub/drive.js`.

import { useState, useEffect } from '../vendor/preact-htm.js';

const state = {
  boot: null,
  lang: 'en',
  page: 'drive',
  game: { running: false },
  // (the drive's: hub/drive.js fills them)
  settings: null,
  config: null,
  choice: null,
  maps: null,
  vehicles: null,
  weather: null,
  lines: { key: '', list: null, loading: false, error: null },
  favourites: [],
  hof: '',
  ibis: null,
  situations: [],
  profiles: null,
  profile: null,
  poll: { stamp: '', jobs: [], instances: [] },
  toasts: [],
  errors: {},
  cache: {},
};
const listeners = new Set();
/** How many changes there were (a component that missed one before it listened draws again). */
let version = 0;

/** The state itself, for the ported pages that read it directly (`S.choice` …). Change it
 * with `set`, so that whoever draws it hears. */
export const S = state;

/** The state as it is now (do not change it: use `set`). */
export function get() {
  return state;
}

/** Change the state: `patch` is merged in (an object), or a function of the state that
 * returns the patch. */
export function set(patch) {
  const p = typeof patch === 'function' ? patch(state) : patch;
  Object.assign(state, p);
  version++;
  for (const l of [...listeners]) l(state);
}

/** Listen to every change: `fn(state)`. Returns the unsubscribe. */
export function subscribe(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

/** In a component: `select(state)`, drawn again when it changes (compared with ===). Without
 * `select`: the whole state, drawn again on every change. */
export function useStore(select) {
  const whole = !select;
  const pick = select || ((s) => s);
  const [value, setValue] = useState(() => pick(state));
  const [, tick] = useState(0);
  const drawnAt = version;
  useEffect(() => {
    const update = whole
      ? () => tick((n) => n + 1)
      : (s) => {
          const v = pick(s);
          setValue((old) => (old === v ? old : v));
        };
    const off = subscribe(update);
    // (a change between the first draw and this effect is not missed)
    if (!whole) update(state);
    else if (version !== drawnAt) tick((n) => n + 1);
    return off;
  }, []);
  return whole ? state : value;
}

let toastId = 0;
/** A short message in the corner (`kind`: ok, error, info). */
export function toast(text, kind = 'info', ms = 5000) {
  const id = ++toastId;
  set({ toasts: [...state.toasts, { id, text: String(text), kind }] });
  setTimeout(() => set({ toasts: state.toasts.filter((x) => x.id !== id) }), kind === 'error' ? Math.max(ms, 8000) : ms);
}
