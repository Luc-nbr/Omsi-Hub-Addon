// Settings: the folders (chosen again through the setup), the plugin in openOMSI, the
// language and the data folder. Updates come with the updater port.

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { call, openPath } from '../api.js';
import { setLanguage, t, useLang } from '../i18n.js';
import { set, useStore } from '../store.js';
import { Button, Card, Note, Page } from '../ui.js';

const LANGUAGE_NAMES = { en: 'English', nl: 'Nederlands', de: 'Deutsch', fr: 'Français', pl: 'Polski', ru: 'Русский', uk: 'Українська' };

export function Settings() {
  useLang();
  const boot = useStore((s) => s.boot);
  const lang = useStore((s) => s.lang);
  const [addon, setAddon] = useState(null);
  const [error, setError] = useState('');
  const load = () => call('addon_state').then(setAddon, (e) => setError(String(e)));
  useEffect(() => { load(); }, []);
  if (!boot) return null;
  const s = boot.settings;
  const plugin = addon && addon.plugin;
  const toggle = async (on) => {
    setError('');
    try { await call('addon_enable', { on }); } catch (e) { setError(String(e)); }
    load();
  };
  return html`<${Page} title=${t('Settings')}>
    <div class="grid">
      <${Card} title=${t('Folders')}>
        <dl class="facts">
          <dt>openOMSI</dt><dd id="set-openomsi">${s.openomsi_folder || '-'}</dd>
          <dt>OMSI 2</dt><dd>${s.omsi2_folder || '-'}</dd>
          <dt>${t('Omsi-Hub data')}</dt><dd><a href="#" onClick=${(e) => { e.preventDefault(); openPath(boot.data_dir); }}>${boot.data_dir}</a></dd>
        </dl>
        <${Button} onClick=${() => set({ setup: true })}>${t('Choose the folders again')}</${Button}>
      </${Card}>
      <${Card} title=${t('The Omsi-Hub plugin in openOMSI')}>
        <p class="dim">${t('State')}: <b id="set-plugin">${plugin ? plugin.state : '-'}</b> · ${t('Version')} ${addon && addon.plugin_version}</p>
        <div class="row">
          <${Button} primary onClick=${() => toggle(true)}>${t('Install and switch on')}</${Button}>
          <${Button} onClick=${() => toggle(false)}>${t('Switch off')}</${Button}>
        </div>
        ${error && html`<${Note} tone="bad">${error}</${Note}>`}
      </${Card}>
      <${Card} title=${t('Language')}>
        <select value=${lang} onChange=${(e) => setLanguage(e.target.value)}>
          ${(boot.languages || ['en']).map((l) => html`<option value=${l}>${LANGUAGE_NAMES[l] || l}</option>`)}
        </select>
      </${Card}>
    </div>
  </${Page}>`;
}
