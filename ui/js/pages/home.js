// Home: the start of the app. For now the version and whether openOMSI is ready; the drive's
// steps (shift, tour, free drive) come with the drive port.

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { call } from '../api.js';
import { t, useLang } from '../i18n.js';
import { useStore } from '../store.js';
import { Card, Note, Page } from '../ui.js';

export function Home() {
  useLang();
  const boot = useStore((s) => s.boot);
  const [addon, setAddon] = useState(null);
  useEffect(() => {
    call('addon_state').then(setAddon, () => setAddon(null));
  }, []);
  const game = addon && addon.game;
  return html`<${Page} title=${t('Welcome to Omsi-Hub')} sub=${t('Built for realism, made for immersion.')}>
    <div class="grid">
      <${Card} title=${t('Your openOMSI')}>
        ${game && game.ok
          ? html`<p id="home-openomsi">openOMSI ${game.version || '?'}</p><${Note} tone="ok">${t('Ready to drive.')}</${Note}>`
          : html`<${Note} tone="warn">${(game && game.text) || t('Choose your openOMSI folder in the settings.')}</${Note}>`}
      </${Card}>
      <${Card} title=${t('Omsi-Hub')}>
        <p class="dim">${t('Version')} ${boot && boot.version.app}</p>
        <p class="faint">${t('Needs openOMSI %{v} or newer', { v: (boot && boot.version.min_openomsi) || '0.2.22' })}</p>
      </${Card}>
    </div>
  </${Page}>`;
}
