// The shell: the boot (`hub_boot`), the first-run setup when the folders are not chosen yet,
// else the navigation and the page shown. Pages come from ./pages/index.js.

import { html, render, useEffect, useState } from '../vendor/preact-htm.js';
import { call, inApp, on } from './api.js';
import { setLanguage, t, useLang } from './i18n.js';
import { PAGES } from './pages/index.js';
import { MigrateOffer, Setup } from './setup.js';
import { set, useStore } from './store.js';

/** hub_boot again (after the setup, or a change of the settings). */
export async function boot() {
  const b = await call('hub_boot');
  set({ boot: b, setup: b.setup_needed });
  return b;
}

function Nav() {
  useLang();
  const page = useStore((s) => s.page);
  return html`<nav class="nav">
    <div class="brand"><img src="img/mark.svg" alt="" /><span>Omsi-Hub</span></div>
    ${PAGES.map((p) => html`<a href="#" id=${'nav-' + p.id} class=${page === p.id ? 'on' : ''} onClick=${(e) => { e.preventDefault(); set({ page: p.id }); }}>${t(p.label)}</a>`)}
    <div class="grow"></div>
    ${!inApp && html`<span class="faint mock">${t('Browser preview')}</span>`}
  </nav>`;
}

function Shell() {
  const page = useStore((s) => s.page);
  const [state, setState] = useState(null);
  useEffect(() => { call('hub_setup_state').then(setState, () => {}); }, []);
  const P = (PAGES.find((p) => p.id === page) || PAGES[0]).component;
  return html`<div class="shell">
    <${Nav} />
    <main class="main">
      <${MigrateOffer} state=${state} onDone=${() => call('hub_setup_state').then(setState)} />
      <${P} />
    </main>
  </div>`;
}

function App() {
  const b = useStore((s) => s.boot);
  const setup = useStore((s) => s.setup);
  if (!b) return html`<div class="splash"><img src="img/mark.svg" alt="" /></div>`;
  if (setup) return html`<${Setup} onDone=${() => boot()} />`;
  return html`<${Shell} />`;
}

async function start() {
  render(html`<${App} />`, document.getElementById('app'));
  const b = await boot().catch((e) => {
    document.getElementById('app').textContent = String(e);
    throw e;
  });
  await setLanguage(b.settings.language);
  on('game-ended', () => set({ game: { running: false } }));
  if (window.__OMSIHUB_SELFTEST__ || new URLSearchParams(location.search).has('selftest')) {
    (await import('./selftest.js')).run();
  }
}

start();
