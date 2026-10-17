// The first-run setup: the openOMSI folder (found, its version read, refused below 0.2.22),
// the OMSI 2 folder, the language; kept in Omsi-Hub's own data folder (`hub_setup_save`).
// The old Omsi-Hub's saves, when there are any in ~/.openomsi, are offered after it.

import { html, useEffect, useState } from '../vendor/preact-htm.js';
import { call, pickFolder } from './api.js';
import { setLanguage, t, useLang } from './i18n.js';
import { get, set, useStore } from './store.js';
import { Button, Note } from './ui.js';
import { PrivacyLink } from './privacy.js';

function FolderStep({ id, title, hint, value, check, suggestions, onChange }) {
  const pick = async () => {
    const f = await pickFolder(title);
    if (f) onChange(f);
  };
  return html`<div class="step">
    <h2>${title}</h2>
    <p class="dim">${hint}</p>
    <div class="row">
      <input id=${id} class="path" value=${value} placeholder=${t('Folder')} onChange=${(e) => onChange(e.target.value)} />
      <${Button} onClick=${pick}>${t('Choose…')}</${Button}>
    </div>
    ${suggestions && suggestions.length > 0 && !value && html`<p class="faint">${t('Found')}: ${suggestions.map((s) => html`<a href="#" class="chip" onClick=${(e) => { e.preventDefault(); onChange(s); }}>${s}</a>`)}</p>`}
    ${check && html`<${Note} tone=${check.ok ? 'ok' : 'bad'}><span id=${id + '-check'}>${check.ok ? (check.version ? 'openOMSI ' + check.version + ' ✓' : t('Found') + ' ✓') : check.text}</span>${check.ok && check.text ? html` · ${check.text}` : ''}</${Note}>`}
  </div>`;
}

export function Setup({ onDone }) {
  useLang();
  const boot = useStore((s) => s.boot);
  const lang = useStore((s) => s.lang);
  const s = (boot && boot.settings) || {};
  const [openomsi, setOpenomsi] = useState(s.openomsi_folder || '');
  const [omsi2, setOmsi2] = useState(s.omsi2_folder || '');
  const [checkO, setCheckO] = useState(null);
  const [checkR, setCheckR] = useState(null);
  const [suggest, setSuggest] = useState({ openomsi: [], omsi2: [] });
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);

  useEffect(() => { call('hub_setup_suggest').then(setSuggest, () => {}); }, []);
  useEffect(() => { if (openomsi) call('hub_setup_openomsi', { folder: openomsi }).then(setCheckO); else setCheckO(null); }, [openomsi]);
  useEffect(() => { if (omsi2) call('hub_setup_omsi2', { folder: omsi2 }).then(setCheckR); else setCheckR(null); }, [omsi2]);

  const save = async () => {
    setBusy(true);
    setError('');
    try {
      await call('hub_setup_save', { openomsi, omsi2, language: lang });
      onDone();
    } catch (e) {
      setError(String(e));
    }
    setBusy(false);
  };
  const ready = checkO && checkO.ok && checkR && checkR.ok;
  return html`<div class="setup">
    <div class="setup-card">
      <img class="setup-logo" src="img/logo-light.svg" alt="Omsi-Hub" />
      <h1>${t('Welcome to Omsi-Hub')}</h1>
      <p class="dim">${t('Omsi-Hub works with the openOMSI you have installed and your original OMSI 2. It keeps its own data and never changes their files.')}</p>
      <${FolderStep} id="setup-openomsi" title=${t('Your openOMSI folder')} hint=${t('The folder openOMSI is in (openOMSI 0.2.22 or newer).')} value=${openomsi} check=${checkO} suggestions=${suggest.openomsi} onChange=${setOpenomsi} />
      <${FolderStep} id="setup-omsi2" title=${t('Your OMSI 2 folder')} hint=${t('The original OMSI 2 with its maps and buses: from Steam (on Linux, Steam with Proton) or the retail disc.')} value=${omsi2} check=${checkR} suggestions=${suggest.omsi2} onChange=${setOmsi2} />
      <div class="step">
        <h2>${t('Language')}</h2>
        <select value=${lang} onChange=${(e) => setLanguage(e.target.value)}>
          ${((boot && boot.languages) || ['en']).map((l) => html`<option value=${l}>${l}</option>`)}
        </select>
      </div>
      ${error && html`<${Note} tone="bad">${error}</${Note}>`}
      <div class="row end"><span class="grow"><${PrivacyLink} /></span><${Button} id="setup-save" primary disabled=${!ready || busy} onClick=${save}>${t('Start')}</${Button}></div>
    </div>
  </div>`;
}

/** After the setup: the old Omsi-Hub's saves, when there are any (once): what was found -
 * the companies, careers, own lines, depot files and settings - and the choice to copy them into
 * the new app's data folder (`hub_migrate_import`; the old folder is never moved or changed) or
 * not (`hub_migrate_skip`). Either way it is not asked again. */
export function MigrateOffer({ state, onDone }) {
  useLang();
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(null);
  const [error, setError] = useState('');
  const m = state && state.migrate;
  if (done) {
    return html`<div class="card" id="migrate">
      <h2>${t('Your saves from the old Omsi-Hub')}</h2>
      <${Note} tone="ok"><span id="migrate-done">${t('Copied: %{n} files. Your companies are ready.', { n: done.imported })}</span></${Note}>
      <div class="row end"><${Button} onClick=${onDone}>${t('Close')}</${Button}></div>
    </div>`;
  }
  if (!m || !m.offer) return null;
  const p = m.preview || {};
  const skip = async () => {
    await call('hub_migrate_skip');
    onDone();
  };
  const copy = async () => {
    setBusy(true);
    setError('');
    try {
      const r = await call('hub_migrate_import');
      setDone(r);
      // (the settings taken over: the language and the look at once)
      if (r && r.settings) {
        const b = get().boot;
        if (b) set({ boot: { ...b, settings: r.settings } });
        if (r.settings.language) await setLanguage(r.settings.language);
      }
    } catch (e) {
      setError(String(e));
    }
    setBusy(false);
  };
  const row = (label, list) => list && list.length > 0 && html`<li><b>${label}</b>: ${list.join(', ')}</li>`;
  return html`<div class="card warn" id="migrate">
    <h2>${t('Your saves from the old Omsi-Hub')}</h2>
    <p>${t('Omsi-Hub found saves of the old Omsi-Hub in %{folder}. They can be copied into the new app: the old folder is not moved or changed.', { folder: m.folder || '~/.openomsi' })}</p>
    <ul class="migrate-found" id="migrate-found">
      ${row(t('Companies'), (p.companies || []).map((c) => c[1] || c[0]))}
      ${row(t('Careers'), p.careers)}
      ${row(t('Own lines'), p.lines)}
      ${row(t('Depot files'), p.depots)}
      ${p.settings && p.settings.length > 0 && html`<li><b>${t('Settings')}</b>: ${t('%{n} settings (look, language, navigator)', { n: p.settings.length })}</li>`}
      ${p.driver && html`<li><b>${t('Driver')}</b>: ${p.driver}</li>`}
    </ul>
    <p class="faint">${t('%{n} files', { n: p.files != null ? p.files : m.files })}</p>
    ${error && html`<${Note} tone="bad">${error}</${Note}>`}
    <div class="row end">
      <${Button} onClick=${skip} disabled=${busy}>${t('Not now')}</${Button}>
      <${Button} id="migrate-copy" primary disabled=${busy} onClick=${copy}>${busy ? t('Copying…') : t('Copy them')}</${Button}>
    </div>
  </div>`;
}
