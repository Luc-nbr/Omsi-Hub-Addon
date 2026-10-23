// After the first run (the welcome, `hub/welcome.js`, which keeps the folders and the language:
// `hub_setup_save`): the old Omsi-Hub's saves, when there are any in ~/.openomsi, offered once.

import { html, useState } from '../vendor/preact-htm.js';
import { call } from './api.js';
import { setLanguage, t, useLang } from './i18n.js';
import { get, set } from './store.js';
import { Button, Note } from './ui.js';

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
    // (not asked again either way; a failure said, not an offer that stays without a word)
    try {
      await call('hub_migrate_skip');
    } catch (e) {
      setError(String(e));
      return;
    }
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
