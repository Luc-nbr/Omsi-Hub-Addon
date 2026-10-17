// What the new app does not have yet, in the places the pages open it: the line editor, the
// livery studio and the privacy statement's page (the 3D bus preview is `bus3d.js`). Each says
// so, with the way back.

import { html } from '../../vendor/preact-htm.js';
import { set } from '../store.js';
import { Icon, tr } from './bits.js';

/** The card in place of a part that comes later. */
export function ComingLater({ icon = 'construction', title, text }) {
  return html`<div class="hub-later">
    <span class="badge"><${Icon} name=${icon} size=${26} /></span>
    <b>${tr(title)}</b>
    <p>${tr(text || 'This part of Omsi-Hub comes to the new app later.')}</p>
  </div>`;
}

/** The line editor's page. */
export const LinesPage = () => html`<${ComingLater} icon="route" title="Line editor" text="The line editor comes to the new app later: it needs our own reader of the map's roads." />`;
export const linesBack = () => 'editor';

/** The livery studio's page (over the whole window in the old app): here in the sheet. */
export function LiveryEditor() {
  return html`<section class="hub-sheet page ed-page">
    <${ComingLater} icon="palette" title="Livery editor" text="The livery studio comes to the new app later." />
    <div class="row end" style="padding:0 24px 24px"><button type="button" class="hub-btn" onClick=${() => set({ page: 'editor' })}><${Icon} name="chevron_left" size=${18} />${tr('Back')}</button></div>
  </section>`;
}

/** The bus seen in 3D on the bus step. */
/** The bus in 3D: drawn from the model the player's openOMSI writes (`bus3d.js`). */
export { Bus3D } from './bus3d.js';

/** The livery studio's own state, which the editors' checks ask for. */
export const studio = null;
