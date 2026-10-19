// The buses' photos for the tiles, taken by the page's own renderer (the old app's photo v4,
// `fotomodus.ts`): a renderer of its own in a worker of its own (the view's would lose its bus),
// one bus at a time, each loaded until it is sharp (25 s at most: then as it is), then one
// picture - 640 x 400, transparent, from the front corner of the door side, 215° round and
// 8° up, the bus 88 % of the width a little above the middle, a soft contact shadow under it.
// The renderer goes 60 s after the last photo.
//
// The same renderer bakes the buses' depot models (`bake.js`, the 3D depot): five flat views and
// a hull, kept by the launcher (`b3d_depot_put`), in the same queue as the photos.

import { call } from '../../api.js';
import { Link } from './link.js';

export const PHOTO_W = 640;
export const PHOTO_H = 400;

let link;
let idle;
let queue = Promise.resolve();

function photoLink() {
  clearTimeout(idle);
  if (!link) {
    link = new Link();
    call('b3d_env')
      .then((env) => link.env(env))
      .catch(() => link.env({ cloudSize: 2000 }));
  }
  return link;
}

function letGo() {
  clearTimeout(idle);
  idle = setTimeout(() => {
    link?.close();
    link = undefined;
  }, 60000);
}

function base64(bytes) {
  const b = new Uint8Array(bytes);
  let s = '';
  for (let i = 0; i < b.length; i += 0x8000) s += String.fromCharCode.apply(null, b.subarray(i, i + 0x8000));
  return btoa(s);
}

/** Take the photo of `bus` in `paint` and keep it (`b3d_photo_put`); resolves with its PNG's
 * bytes, rejects when the bus cannot be drawn here (no WebGL2: `{ noWebgl: true }`). */
export function takePhoto(bus, paint) {
  const job = queue.then(() => take(bus, paint));
  queue = job.catch(() => {});
  return job;
}

/** Bake the depot model of `bus` in `paint` and keep it (`b3d_depot_put`); resolves with
 * `{ png, model }`, rejects as `takePhoto` does. */
export function bakeDepot(bus, paint) {
  const job = queue.then(() => take(bus, paint, 'bake'));
  queue = job.catch(() => {});
  return job;
}

async function take(bus, paint, what = 'photo') {
  const l = photoLink();
  const ready = await l.ready;
  if (!ready.webgl) {
    letGo();
    throw Object.assign(new Error('no WebGL2'), { noWebgl: true });
  }
  const t0 = performance.now();
  const canvas = document.createElement('canvas');
  let sharp;
  const done = new Promise((ok) => (sharp = ok));
  const h = l.join(canvas, { state: (s) => s.phase === 'sharp' && sharp('sharp'), error: (e) => sharp('error:' + (e.detail || e.reason)) });
  try {
    h.size(PHOTO_W, PHOTO_H, 1);
    const r = await call('b3d_open', { bus, paint: paint || '' });
    h.load(r.manifest, r.look, r.cache ? 'cache' : 'new', performance.timeOrigin + performance.now());
    const why = await Promise.race([done, new Promise((ok) => setTimeout(() => ok('late'), 25000))]);
    if (String(why).startsWith('error:')) throw new Error(why.slice(6));
    if (what === 'bake') {
      const baked = await h.shot({ bake: true });
      if (!baked?.png || !baked.model) throw new Error(baked?.error || 'no depot model');
      await call('b3d_depot_put', { bus, paint: paint || '', png: base64(baked.png), model: baked.model });
      return baked;
    }
    const shot = await h.shot({ photo: true, w: PHOTO_W, h: PHOTO_H });
    if (!shot?.png) throw new Error(shot?.error || 'no picture');
    await call('b3d_photo_put', { bus, paint: paint || '', png: base64(shot.png) });
    (window.hubBus3dPhotos ??= []).push({ bus, paint: paint || '', ms: Math.round(performance.now() - t0), sharp: why === 'sharp', cache: r.cache });
    return shot.png;
  } finally {
    h.gone();
    letGo();
  }
}
