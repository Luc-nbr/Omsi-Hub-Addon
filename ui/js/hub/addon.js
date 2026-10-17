// Omsi-Hub beside the player's own openOMSI: the Settings page's "openOMSI and OMSI 2" tab -
// the two folders the first-run setup chose (chosen again through it), the version read from
// openOMSI's folder, how well Omsi-Hub works with it (supported, with limits, not supported, or
// checked at the first drive), and the Omsi-Hub plugin switched on or off there (`addon_state`,
// `addon_enable`; the folders' checks are `hub_setup_openomsi` and `hub_setup_omsi2`).

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { call, openPath } from '../api.js';
import { S, set } from '../store.js';
import { Icon, Button, Switch, tr } from './bits.js';

const LEVELS = {
  supported: ['check_circle', 'Supported', 'ok'],
  limited: ['info', 'Supported with limits', 'warn'],
  unsupported: ['block', 'Not supported', 'bad'],
  unknown: ['help', 'Checked at the first drive', 'warn'],
};

const FROM = {
  executable: 'read from the program',
  bundle: 'read from the app bundle',
  folder_name: 'read from the folder name',
  program: 'asked of the program',
};

/** The tab's one row: the folder, the game in it, the compatibility, the plugin. */
export function AddonPanel() {
  const [s, setS] = useState(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const run = (cmd, args) => {
    setBusy(true);
    setErr('');
    return call(cmd, args)
      .then((v) => setS(v))
      .catch((e) => setErr(String(e)))
      .finally(() => setBusy(false));
  };
  useEffect(() => {
    run('addon_state');
  }, []);
  const [omsi2, setOmsi2] = useState(null);
  useEffect(() => {
    if (S.settings?.omsi2_folder) call('hub_setup_omsi2', { folder: S.settings.omsi2_folder }).then(setOmsi2, () => {});
  }, []);
  if (!s) return html`<div class="setting-row full"><span class="muted small">${err ? tr(err) : ''}</span></div>`;
  const inst = s.game;
  const level = inst?.compat && LEVELS[inst.compat.level];
  const plugin = s.plugin;
  // (a path is shown as the system writes it: `\` on Windows, `/` on Linux and macOS)
  const folder = (icon, name, path, check, hint) => html`<div class="row wrap hub-folder" style="gap:12px;align-items:center;margin-top:8px">
      <${Icon} name=${icon} />
      <span style="min-width:150px"><b>${name}</b>${hint && html`<br /><span class="muted small">${tr(hint)}</span>`}</span>
      <a href="#" class="grow hub-path" id=${name === 'openOMSI' ? 'set-openomsi' : 'set-omsi2'} style="word-break:break-all" onClick=${(e) => (e.preventDefault(), path && openPath(path))}>${path || tr('Not chosen')}</a>
      ${check && !check.ok && html`<span class="small bad">${tr(check.text || '')}</span>`}
    </div>`;
  return html`<div class="setting-row full hub-addon">
    <p class="muted small" style="margin:0 0 6px">${tr('Omsi-Hub works with the openOMSI you have installed and your original OMSI 2. It keeps its own data and never changes their files.')}</p>
    ${folder('folder', 'openOMSI', s.folder, inst, '0.2.22 or newer')}
    ${folder('folder_open', 'OMSI 2', S.settings?.omsi2_folder, omsi2, 'Steam, Steam with Proton, or retail')}
    <div class="row wrap" style="gap:12px;margin-top:12px">
      <${Button} icon="folder_open" label="Choose the folders again" disabled=${busy} onClick=${() => set({ setup: true })} />
      ${S.boot?.data_dir && html`<${Button} kind="ghost" icon="open_in_new" label="Omsi-Hub's data folder" onClick=${() => openPath(S.boot.data_dir)} />`}
    </div>
    ${err && html`<p class="small bad" style="margin:8px 0 0">${tr(err)}</p>`}
    ${inst &&
    html`<div style="margin-top:12px">
      <div class="row wrap" style="gap:8px;align-items:center">
        <b>${inst.version ? tr('openOMSI %{version}', { version: inst.version }) : tr('Version not found')}</b>
        ${inst.version_from && html`<span class="muted small">(${tr(FROM[inst.version_from])})</span>`}
      </div>
      ${level &&
      html`<div class=${'row wrap ' + level[2]} style="gap:8px;align-items:center;margin-top:8px">
        <${Icon} name=${level[0]} /><b>${tr(level[1])}</b>
        <span class="muted small">${inst.compat.confirmed ? tr('Checked with this openOMSI') : tr('Estimated from the version')}</span>
      </div>`}
      ${(inst.compat?.notes || []).map((n) => html`<p class="small" style="margin:4px 0 0">${tr(n)}</p>`)}
      <ul class="small" style="margin:8px 0 0;padding-left:18px">
        ${(inst.compat?.features || []).map(
          (f) => html`<li class=${f.on ? '' : 'muted'}>${tr(f.label)}${!f.on && f.fallback ? html` <span class="muted">(${tr(f.fallback)})</span>` : ''}</li>`,
        )}
      </ul>
      <div class="row wrap" style="gap:12px;align-items:center;margin-top:12px">
        <span class="grow"><${Switch}
          checked=${plugin && (plugin.state === 'on' || plugin.state === 'outdated')}
          label="Omsi-Hub plugin in this openOMSI"
          onChange=${(on) => run('addon_enable', { on })}
        /></span>
        ${plugin?.state === 'outdated' && html`<${Button} icon="download" label="Update the plugin" disabled=${busy} onClick=${() => run('addon_enable', { on: true })} />`}
      </div>
      <p class="muted small" style="margin:4px 0 0">${plugin ? pluginLine(plugin, s.plugin_version) : ''}</p>
      <p class="muted small" style="margin:4px 0 0;word-break:break-all">${tr('Plugins folder: %{dir}', { dir: inst.plugins_dir || '-' })}</p>
    </div>`}
  </div>`;
}

function pluginLine(p, ours) {
  if (p.state === 'missing') return tr('Not installed');
  if (p.state === 'off') return tr('Switched off in this openOMSI');
  if (p.state === 'outdated') return tr('Version %{version} installed, this Omsi-Hub has %{ours}', { version: p.version || '?', ours });
  return tr('Version %{version} installed', { version: p.version || ours });
}
