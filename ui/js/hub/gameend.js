// The word after a drive that ended on an error: hub-api's `game-ended` (`{ code, seconds }`:
// openOMSI's exit code, none when it was stopped) with a code other than 0, and the end of
// openOMSI's log when hub-api sends it (`log`), to copy.

import { html } from '../../vendor/preact-htm.js';
import { set, useStore } from '../store.js';
import { Icon, tr } from './bits.js';

export function GameEndDialogs() {
  const s = useStore();
  const e = s.game?.ended;
  if (!e || e.code == null || e.code === 0) return null;
  const close = () => set({ game: { ...s.game, ended: null } });
  const log = e.log || e.text || '';
  return html`<div class="overlay" onMouseDown=${(ev) => ev.target === ev.currentTarget && close()}>
    <div class="dialog hub-end" role="dialog">
      <div class="dialog-head"><h2><${Icon} name="warning" size=${18} /> ${tr('The game closed on an error')}</h2></div>
      <div class="dialog-body">
        ${e.code != null && html`<p class="dim">${tr('Exit code %{code}', { code: e.code })}</p>`}
        ${log && html`<pre class="log selectable">${log}</pre>`}
      </div>
      <div class="dialog-foot row end">
        ${log && html`<button type="button" class="hub-btn" onClick=${() => navigator.clipboard?.writeText(log).catch(() => {})}><${Icon} name="content_copy" size=${16} />${tr('Copy the log')}</button>`}
        <button type="button" class="hub-btn hub-go" onClick=${close}>${tr('Close')}</button>
      </div>
    </div>
  </div>`;
}
