// Small building blocks every page uses: a page's frame, a card, a button, a folder field,
// a note; and the fields of the ported pages (a text, a search, a dropdown, a picture). A port
// brings its own larger ones beside its page.

import { html } from '../vendor/preact-htm.js';
import { t } from './i18n.js';
import { Icon } from './hub/bits.js';

/** A page: its title, a line under it, and its content. */
export function Page({ title, sub, children }) {
  return html`<section class="page">
    <header class="page-head"><h1>${title}</h1>${sub && html`<p class="sub">${sub}</p>`}</header>
    ${children}
  </section>`;
}

export function Card({ title, children, tone }) {
  return html`<div class=${'card' + (tone ? ' ' + tone : '')}>${title && html`<h2>${title}</h2>`}${children}</div>`;
}

export function Button({ onClick, children, primary, disabled, id }) {
  return html`<button id=${id} class=${primary ? 'btn primary' : 'btn'} disabled=${disabled} onClick=${onClick}>${children}</button>`;
}

/** A note: `tone` ok, warn or bad. */
export function Note({ tone = 'info', children }) {
  return html`<p class=${'note ' + tone}>${children}</p>`;
}

/** What a page that is not ported yet shows. */
export function Placeholder({ title, sub, what }) {
  return html`<${Page} title=${title} sub=${sub}>
    <${Card}><p class="dim">${t('This part of Omsi-Hub is being moved to the new app.')}</p>${what && html`<p class="faint">${what}</p>`}</${Card}>
  </${Page}>`;
}

// --- the fields the ported pages use (the drive's steps, the company's dialogs) -------------

/** A text field: `onInput(text)` on every key. */
export function Input({ value, onInput, onKeyDown, placeholder = '', raw, id, style }) {
  return html`<input id=${id} class="input" style=${style} value=${value ?? ''} placeholder=${raw ? placeholder : t(placeholder)} spellcheck="false" onInput=${(e) => onInput?.(e.currentTarget.value)} onKeyDown=${onKeyDown} />`;
}

/** A search field: its magnifier on the left. */
export function Search({ value, onInput, placeholder = 'Search…', raw }) {
  return html`<label class="search"><${Icon} name="search" size=${16} /><input class="input" type="search" value=${value ?? ''} placeholder=${raw ? placeholder : t(placeholder)} spellcheck="false" onInput=${(e) => onInput?.(e.currentTarget.value)} /></label>`;
}

/** A dropdown. `options` are `[value, label]` pairs, or plain labels (then the value is the
 * index); `raw` keeps the labels as they are (else they are translated). `onChange` gets the
 * value as it was given (a number stays a number). */
export function Select({ value, options = [], onChange, raw, placeholder, style, h, disabled }) {
  const pairs = options.map((o, k) => (Array.isArray(o) ? o : [k, o]));
  const at = pairs.findIndex(([v]) => String(v) === String(value));
  const label = (l) => (raw ? l : t(String(l ?? '')));
  return html`<select class="select" style=${(h ? `height:${h}px;` : '') + (style || '')} disabled=${disabled} value=${at >= 0 ? String(at) : ''}
    onChange=${(e) => { const k = Number(e.currentTarget.value); if (pairs[k]) onChange?.(pairs[k][0]); }}>
    ${at < 0 && html`<option value="" disabled>${placeholder || ''}</option>`}
    ${pairs.map(([, l], k) => html`<option value=${String(k)}>${label(l)}</option>`)}
  </select>`;
}

/** A picture filling its box (a map's, a bus's), a quiet box while there is none. */
export function Picture({ src, alt = '' }) {
  return html`<div class="pic" role="img" aria-label=${alt} style=${src ? `background-image:url("${src}")` : ''}></div>`;
}

/** The turning ring while something loads. */
export const Spinner = () => html`<div class="spinner" role="progressbar"></div>`;
