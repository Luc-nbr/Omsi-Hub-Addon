// openOMSI's own settings on Omsi-Hub's Settings page (the tab "openOMSI's settings"): the
// graphics, the screen and the navigator, the game's language, driving and the passengers, as
// openOMSI keeps them in `~/.openomsi/settings.cfg` (`addon_game_settings`). A change is written
// at once (`addon_game_setting`), never while the game runs; "openOMSI's default" takes the key
// out of the file, so that openOMSI's own default holds. Omsi-Hub changes only the keys listed
// here: the rest of the file stays as it is. The game controllers' groups (the wheel and the
// pedals, the gamepad, force feedback) are on the tab "Controllers" instead, beside the devices
// (`controllers.js`: `<GameSettingsPanel controllers />`).

import { html, useEffect, useRef, useState } from '../../vendor/preact-htm.js';
import { call, openPath } from '../api.js';
import { Button, Slider, tr } from './bits.js';
import { Select } from '../ui.js';

const DEFAULT = "openOMSI's default";

/** A number as the slider shows it (no float noise). */
const shown = (n) => String(Math.round(Number(n) * 100) / 100);

function Field({ f, running, save }) {
  const set = (v) => save(f.key, v);
  if (f.kind === 'number') {
    return html`<${NumberField} f=${f} running=${running} save=${save} />`;
  }
  if (f.kind === 'text') {
    return html`<${TextField} f=${f} running=${running} save=${save} />`;
  }
  const options = f.kind === 'switch' ? [['', DEFAULT], ['1', 'On'], ['0', 'Off']] : [['', DEFAULT], ...f.options];
  return html`<div class="setting-row" id=${'oo-' + f.key}><span class="name">${tr(f.label)}</span>
    <${Select} value=${f.value ?? ''} options=${options} disabled=${running} onChange=${set} /></div>`;
}

/** A number: the slider, written a moment after it stops moving; "Default" takes it out. */
function NumberField({ f, running, save }) {
  const [v, setV] = useState(null);
  const timer = useRef(0);
  useEffect(() => () => clearTimeout(timer.current), []);
  const value = v ?? (f.value != null ? Number(f.value) : (f.default ?? f.min));
  const move = (n) => {
    if (running) return;
    setV(n);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => save(f.key, String(n)).then(() => setV(null)), 350);
  };
  return html`<div class="setting-row full" id=${'oo-' + f.key}>
    <${Slider} label=${f.label} value=${value} min=${f.min} max=${f.max} step=${f.step} format=${(n) => {
      if (f.value != null || v != null) return shown(n);
      // (not in the file: openOMSI's default, its number where the guide says it)
      return f.default != null ? `${shown(n)} (${tr(DEFAULT)})` : tr(DEFAULT);
    }} onChange=${move} />
    ${f.value != null && !running && html`<div><${Button} kind="ghost" small icon="restart_alt" label=${DEFAULT} onClick=${() => save(f.key, null)} /></div>`}
  </div>`;
}

/** A value the guide names without its values: a line of text, written when it is left (or on
 * Enter); empty is openOMSI's default. */
function TextField({ f, running, save }) {
  const [v, setV] = useState(null);
  const text = v ?? f.value ?? '';
  const done = () => {
    if (v == null || v.trim() === (f.value ?? '')) return setV(null);
    save(f.key, v.trim()).then(() => setV(null));
  };
  return html`<div class="setting-row" id=${'oo-' + f.key}><span class="name">${tr(f.label)}</span>
    <input class="path" style="min-width:0;flex:none;width:min(320px,100%)" value=${text} disabled=${running} placeholder=${`${tr(DEFAULT)} · ${tr(f.hint)}`}
      onInput=${(e) => setV(e.currentTarget.value)} onBlur=${done} onKeyDown=${(e) => e.key === 'Enter' && e.currentTarget.blur()} /></div>`;
}

/** The tab's panel: every group of openOMSI's settings but the game controllers' - or, with
 * `controllers`, theirs only (the keys of `hide` left out: the tab does them its own way).
 * `onChange` hears the settings after every change. */
export function GameSettingsPanel({ controllers = false, hide = [], onChange } = {}) {
  const [s, setS] = useState(null);
  const [err, setErr] = useState('');
  const got = (v) => (setS(v), onChange?.(v));
  const load = () => call('addon_game_settings').then(got, (e) => setErr(String(e?.message || e)));
  useEffect(() => {
    load();
  }, []);
  const save = (key, value) =>
    call('addon_game_setting', { key, value })
      .then((v) => (setErr(''), got(v)))
      .catch((e) => (setErr(String(e?.message || e)), load()));
  if (!s) return html`<div class="setting-row full"><span class="muted small">${err ? tr(err) : ''}</span></div>`;
  const groups = s.groups.filter((g) => !!g.controllers === controllers).map((g) => ({ ...g, fields: g.fields.filter((f) => !hide.includes(f.key)) }));
  return html`<div class=${'setting-row full hub-game-settings' + (controllers ? ' controllers' : '')}>
    ${!controllers && html`<p class="muted small" style="margin:0 0 6px">${tr("openOMSI's own settings. Omsi-Hub changes only these; everything else in the file stays as it is. openOMSI reads them when it starts.")}</p>`}
    <div class="row wrap" style="gap:12px;align-items:center">
      <a href="#" class="hub-path small" style="word-break:break-all" onClick=${(e) => (e.preventDefault(), s.exists && openPath(s.path))}>${s.path}</a>
      ${!s.exists && html`<span class="muted small">${tr('Not there yet: it is made with the first change.')}</span>`}
    </div>
    ${s.running && html`<p class="small warn" id="oo-running" style="margin:8px 0 0">${tr('openOMSI is running: change its settings once it has ended')}</p>`}
    ${err && html`<p class="small bad" style="margin:8px 0 0">${tr(err)}</p>`}
    ${groups.map((g) => html`<section class="hub-game-group" style="margin-top:14px">
      <h4 class="hub-caps" style="margin:0 0 4px"><b>${tr(g.title)}</b></h4>
      ${g.fields.map((f) => html`<${Field} key=${f.key} f=${f} running=${s.running} save=${save} />`)}
    </section>`)}
  </div>`;
}
