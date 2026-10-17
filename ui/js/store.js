// The page's global state: one object, changed with `set`, read in a component with
// `useStore(select)` (the component draws again when what it selected changed).
//
// What lives here: `boot` (hub_boot's answer), `lang`, `page` (the page shown), `game`
// (whether openOMSI runs). A feature's own state goes under its own key (`company`, …).

import { useState, useEffect } from '../vendor/preact-htm.js';

const state = { boot: null, lang: 'en', page: 'home', game: { running: false } };
const listeners = new Set();

/** The state as it is now (do not change it: use `set`). */
export function get() {
  return state;
}

/** Change the state: `patch` is merged in (an object), or a function of the state that
 *  returns the patch. */
export function set(patch) {
  const p = typeof patch === 'function' ? patch(state) : patch;
  Object.assign(state, p);
  for (const l of [...listeners]) l(state);
}

/** Listen to every change: `fn(state)`. Returns the unsubscribe. */
export function subscribe(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

/** In a component: `select(state)`, drawn again when it changes (compared with ===). */
export function useStore(select = (s) => s) {
  const [value, setValue] = useState(() => select(state));
  useEffect(() => {
    const update = (s) => {
      const v = select(s);
      setValue((old) => (old === v ? old : v));
    };
    const off = subscribe(update);
    // (a change between the first draw and this effect is not missed)
    update(state);
    return off;
  }, []);
  return value;
}
