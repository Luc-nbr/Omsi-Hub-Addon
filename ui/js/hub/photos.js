// The buses' photos on the fleet's, the dealer's and Casual's tiles and on the bus step. A photo
// kept is asked for once (`co_photo`); one not there yet is taken here, one after the other, from
// the model the player's openOMSI writes for the 3D view (`hub_bus3d`, `bus3d.js`'s
// `photograph`), and handed over to be kept (`co_photo_save`). A tile asks for its photo
// (`wantPhoto`) while it is shown: one no longer shown leaves the queue.

import { call, callBytes } from '../api.js';
import { photograph } from './bus3d.js';

const id = (file, paint) => `${file}|${paint || ''}`;

/** The photos asked for: id → the promise of its URL (null: none yet). */
const have = new Map();
/** Who waits for a photo being taken: id → the `onLater` callbacks. */
const waiting = new Map();
/** The photos to take, in turn: `{ k, file, paint, want }` (`want`: the tiles that want it). */
const queue = [];
/** The photos taken or tried this time (a bus openOMSI cannot write is not tried again). */
const tried = new Set();
let working = false;

const urlOf = (blob) => URL.createObjectURL(blob);

/** The photo of `file` in `paint`: the promise of its URL, null while there is none.
 * `onLater(url)` is called when one is taken later. */
export function photo(file, paint, onLater) {
  if (!file) return Promise.resolve(null);
  const k = id(file, paint);
  if (onLater) {
    if (!waiting.has(k)) waiting.set(k, new Set());
    waiting.get(k).add(onLater);
  }
  if (!have.has(k))
    have.set(
      k,
      callBytes('co_photo', { bus: file, livery: paint || '' }).then(
        (bytes) => urlOf(new Blob([bytes], { type: 'image/png' })),
        () => null,
      ),
    );
  return have.get(k);
}

/** Ask for the photo of `file` in `paint` to be taken when there is none. Answers the function
 * that says it is no longer wanted (a photo already being taken is still finished). */
export function wantPhoto(file, paint) {
  const k = id(file, paint);
  if (!file || tried.has(k)) return () => {};
  let job = queue.find((j) => j.k === k);
  if (!job) queue.push((job = { k, file, paint: paint || '', want: 0 }));
  job.want++;
  work();
  let left = false;
  return () => {
    if (left) return;
    left = true;
    job.want--;
    const at = queue.indexOf(job);
    if (job.want <= 0 && at >= 0) queue.splice(at, 1);
  };
}

/** The model's key once openOMSI has written it (asked again while it writes), null when it
 * cannot. */
async function model(file, paint) {
  for (;;) {
    const r = await call('hub_bus3d', { bus: file, paint });
    if (r.state === 'ready') return r.key;
    if (r.state !== 'making') return null;
    await new Promise((done) => setTimeout(done, 1500));
  }
}

const base64 = (blob) =>
  new Promise((done, fail) => {
    const read = new FileReader();
    read.onload = () => done(String(read.result).slice(String(read.result).indexOf(',') + 1));
    read.onerror = () => fail(read.error);
    read.readAsDataURL(blob);
  });

/** Take the photos in the queue, one at a time. */
async function work() {
  if (working) return;
  working = true;
  try {
    while (queue.length) {
      const job = queue.shift();
      tried.add(job.k);
      try {
        if (await photo(job.file, job.paint)) continue;
        const key = await model(job.file, job.paint);
        if (!key) continue;
        const blob = await photograph(key);
        await call('co_photo_save', { bus: job.file, livery: job.paint, png: await base64(blob) });
        const url = urlOf(blob);
        have.set(job.k, Promise.resolve(url));
        for (const f of waiting.get(job.k) || []) f(url);
        waiting.delete(job.k);
      } catch (e) {
        console.warn(`photo of ${job.file} (${job.paint}) not taken:`, e);
      }
    }
  } finally {
    working = false;
  }
}
