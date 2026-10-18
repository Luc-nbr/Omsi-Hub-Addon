// What the new app does not have yet, in the places the pages open it: the line editor. It says
// so, with the way back. (The 3D view and the livery studio are `bus3d/`.)

import { html } from '../../vendor/preact-htm.js';
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
