// Small building blocks every page uses: a page's frame, a card, a button, a folder field,
// a note. A port brings its own larger ones beside its page.

import { html } from '../vendor/preact-htm.js';
import { t } from './i18n.js';

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
