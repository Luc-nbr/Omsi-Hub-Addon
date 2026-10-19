// The livery editor's page: the studio on the page's own GPU (`studio.js`) where the page can
// draw with WebGL2; else a word that it cannot (the old app streamed the game program's studio
// as pictures there, `editors/livery.js`: the add-on has no game program to run it).

import { html, useEffect, useState } from '../../../../vendor/preact-htm.js';
import { set } from '../../../store.js';
import { Icon, tr } from '../../../editors/kit.js';
import { Link } from '../link.js';
import { LakStudio } from './studio.js';

/** Whether the page draws (known once the renderer answered). */
let can;

export function LiveryEditor() {
  const [ok, setOk] = useState(can);
  useEffect(() => {
    if (can !== undefined) return;
    Link.get().ready.then((r) => {
      can = !!r.webgl;
      setOk(can);
    });
  }, []);
  if (ok === undefined) return html`<div class="ed-studio"><div class="ed-studio-note">${tr('Loading...')}</div></div>`;
  if (ok) return html`<${LakStudio} />`;
  return html`<section class="hub-sheet page ed-page">
    <div class="hub-later">
      <span class="badge"><${Icon} name="palette" size=${26} /></span>
      <b>${tr('Livery editor')}</b>
      <p>${tr('The livery studio paints the bus with the graphics card: this computer cannot draw with WebGL2 here.')}</p>
    </div>
    <div class="row end" style="padding:0 24px 24px"><button type="button" class="hub-btn" onClick=${() => set({ page: 'editor' })}><${Icon} name="chevron_left" size=${18} />${tr('Back')}</button></div>
  </section>`;
}
