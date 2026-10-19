// The profile photo (crates/hub-online/src/avatar.rs; the website shows the same one): a picture
// chosen here is cut to a square from its middle and drawn at 256 px as WebP (JPEG where WebP
// cannot be written), then sent to Rust (`ol_avatar`), which puts it in the account's folder of the
// bucket `avatars`. Without a photo the name's first letter stands in, as before.

import { html, useRef, useState } from '../../vendor/preact-htm.js';
import { call } from '../api.js';
import { language } from '../i18n.js';
import { Btn, Icon, toast, tr } from './bits.js';
import { poll } from './online.js';

const SIZE = 256;
/** The bucket takes 1 MB; a 256 px picture is far below it. */
const MAX = 1 << 20;

/** The picture `file` as a square `SIZE` px data URL, or null when it is no picture. */
export async function squarePicture(file) {
  if (!file || !/^image\//.test(file.type)) return null;
  const url = URL.createObjectURL(file);
  try {
    const img = await new Promise((ok, no) => {
      const i = new Image();
      i.onload = () => ok(i);
      i.onerror = no;
      i.src = url;
    });
    const side = Math.min(img.naturalWidth, img.naturalHeight);
    if (!side) return null;
    const c = document.createElement('canvas');
    c.width = SIZE;
    c.height = SIZE;
    const g = c.getContext('2d');
    g.imageSmoothingQuality = 'high';
    g.drawImage(img, (img.naturalWidth - side) / 2, (img.naturalHeight - side) / 2, side, side, 0, 0, SIZE, SIZE);
    let out = c.toDataURL('image/webp', 0.86);
    if (!out.startsWith('data:image/webp')) out = c.toDataURL('image/jpeg', 0.88);
    return out.length * 0.75 > MAX ? null : out;
  } catch {
    return null;
  } finally {
    URL.revokeObjectURL(url);
  }
}

/** The photo, or the name's first letter where there is none (or it will not load). */
export function Avatar({ url, name, size = 40, cls = '' }) {
  const [broken, setBroken] = useState('');
  const letter = (String(name || '?').trim()[0] || '?').toUpperCase();
  const style = `width:${size}px;height:${size}px;font-size:${Math.round(size * 0.42)}px`;
  if (url && broken !== url) {
    return html`<img class=${'ol-avatar ' + cls} src=${url} alt="" style=${style} draggable=${false} onError=${() => setBroken(url)} />`;
  }
  return html`<span class=${'ol-avatar letter ' + cls} style=${style} aria-hidden="true">${letter}</span>`;
}

/** The photo with its buttons: choose a picture, or take it away. `me`: `ol_state`'s `me`.
 * `compact`: the photo alone, a click on it chooses a new one (the service record's). */
export function AvatarPicker({ me, size = 72, compact = false }) {
  const input = useRef();
  const [busy, setBusy] = useState(false);
  const send = async (args) => {
    setBusy(true);
    let r;
    try {
      r = await call('ol_avatar', { ...args, lang: language() });
    } catch (e) {
      r = { error: String(e?.message || e) };
    }
    setBusy(false);
    if (r?.error) toast(r.error, r.code === 'offline' ? 'info' : 'error');
    else if (r?.message) toast(r.message, 'ok');
    poll();
  };
  const chosen = async (e) => {
    const file = e.currentTarget.files?.[0];
    e.currentTarget.value = '';
    if (!file) return;
    const data = await squarePicture(file);
    if (!data) return toast(tr('That picture cannot be used: choose a JPEG, PNG or WebP picture.'), 'error');
    send({ data });
  };
  const file = html`<input ref=${input} type="file" accept="image/png,image/jpeg,image/webp" hidden onChange=${chosen} />`;
  if (compact) {
    return html`<button type="button" class=${'ol-avatar-btn' + (busy ? ' busy' : '')} title=${me?.avatar ? tr('Change photo') : tr('Upload a photo')} disabled=${busy} onClick=${() => input.current?.click()}>
      <${Avatar} url=${me?.avatar} name=${me?.name} size=${size} /><span class="ol-avatar-over"><${Icon} name=${busy ? 'refresh' : 'upload'} size=${Math.round(size * 0.34)} class=${busy ? 'ol-spin' : ''} /></span>
    </button>${file}`;
  }
  return html`<div class="ol-avatar-pick">
    <${Avatar} url=${me?.avatar} name=${me?.name} size=${size} />
    <div class="ol-avatar-acts">
      <${Btn} icon="upload" label=${me?.avatar ? tr('Change photo') : tr('Upload a photo')} busy=${busy} onClick=${() => input.current?.click()} />
      ${me?.avatar && html`<${Btn} kind="ghost" icon="delete" label=${tr('Remove photo')} disabled=${busy} onClick=${() => send({ clear: true })} />`}
      <em class="ol-hint">${tr('Shown to your friends, on the leaderboard and on the website.')}</em>
    </div>
    ${file}
  </div>`;
}
