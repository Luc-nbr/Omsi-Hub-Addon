// The buses' own depot models on the 3D yard: one a bus file and livery, however many buses
// wear it. Kept by the launcher (`b3d_depot_has`, the atlas `b3d_depot`); one not kept yet (or
// made of the bus before its files changed) is baked by the page's renderer (`bake.js`, in the
// photos' queue: one bus at a time) and kept. Until it is there - and where the page cannot draw
// a bus (no WebGL2), or the bus cannot be read - the yard shows the bus as the plain model in
// the company's colour (`bus.js`).

import { S } from '../../store.js';
import { call, callBytes } from '../../api.js';
import { bakeDepot } from '../../hub/bus3d/photo.js';
import { unpackModel } from '../../hub/bus3d/bake.js';

/** `file|paint` -> { state: 'loading' | 'ready' | 'failed', model, image, at, listeners }. */
const skins = new Map();
/** The page cannot draw: nothing is baked. */
let noWebgl = false;

export const skinKey = (file, paint) => `${file}|${paint || ''}`;

/**
 * The depot model of `file` in `paint` when it is there (`{ model, image }`), else null;
 * `ready` hears when it comes. One that could not be had is tried again after a few minutes.
 */
export function skin(file, paint, ready) {
  if (!file) return null;
  const key = skinKey(file, paint);
  let s = skins.get(key);
  if (s?.state === 'ready') return s;
  if (s?.state === 'loading') {
    s.listeners.add(ready);
    return null;
  }
  if (s?.state === 'failed' && Date.now() - s.at < 300000) return null;
  s = { state: 'loading', listeners: new Set([ready]) };
  skins.set(key, s);
  load(file, paint || '', s);
  return null;
}

async function kept(file, paint) {
  const has = await call('b3d_depot_has', { bus: file, paint });
  return has?.has ? unpackModel(has.model) : null;
}

async function load(file, paint, s) {
  try {
    let model = await kept(file, paint);
    if (!model) {
      // (none in the quick self-check, as the photos; none where the page cannot draw)
      if (S.boot?.selftest && S.boot.selftest !== 'full') throw new Error('no depot models in the quick self-check');
      if (noWebgl) throw new Error('no WebGL2');
      await bakeDepot(file, paint);
      model = await kept(file, paint);
      if (!model) throw new Error('not kept');
    }
    const bytes = await callBytes('b3d_depot', { bus: file, paint });
    const image = await createImageBitmap(new Blob([bytes], { type: 'image/png' }));
    Object.assign(s, { state: 'ready', model, image });
    for (const f of s.listeners) f?.();
  } catch (x) {
    if (x?.noWebgl) noWebgl = true;
    Object.assign(s, { state: 'failed', at: Date.now() });
  }
  s.listeners.clear();
}
