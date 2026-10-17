// The app's start: the boot (`hub_boot`), the first-run setup when the folders are not chosen
// yet, else Omsi-Hub's shell (`hub/hub.js`: the glass bar, the drive's start and steps, the bus
// company, the editors, the pages of ./pages/index.js) with the messages in the corner.

import { html, render, useEffect, useState } from '../vendor/preact-htm.js';
import { call } from './api.js';
import { setLanguage } from './i18n.js';
import { PAGES } from './pages/index.js';
import { MigrateOffer, Setup } from './setup.js';
import { set, useStore } from './store.js';
import { startDrive, watchGame } from './hub/drive.js';
import { HubShell } from './hub/hub.js';
import { Icon } from './hub/bits.js';

/** The selftest's run (the app opens the page with `?selftest`). */
const SELFTEST = !!window.__OMSIHUB_SELFTEST__ || new URLSearchParams(location.search).has('selftest');

/** hub_boot again (after the setup, or a change of the settings). */
export async function boot() {
  const b = await call('hub_boot');
  // (the pages leave out what a check would wait for: the opening, the updates)
  if (SELFTEST) b.selftest = true;
  set({ boot: b, setup: b.setup_needed });
  if (!b.setup_needed) startDrive(b);
  return b;
}

/** The pages as the shell takes them: id, label, icon, component; and their lines. */
const SHELL_PAGES = PAGES.map((p) => [p.id, p.label, p.icon, p.component]);

function Toasts() {
  const toasts = useStore((s) => s.toasts);
  return html`<div class="toasts">${toasts.map((x) => html`<div class=${'toast ' + x.kind}><${Icon} name=${x.kind === 'error' ? 'warning' : x.kind === 'ok' ? 'check' : 'info'} size=${18} /><div class="selectable">${x.text}</div></div>`)}</div>`;
}

/** The old Omsi-Hub's saves, offered once over the start. */
function Migrate() {
  const [state, setState] = useState(null);
  useEffect(() => {
    call('hub_setup_state').then(setState, () => {});
  }, []);
  if (!state?.migrate?.offer) return null;
  return html`<div class="hub-migrate"><${MigrateOffer} state=${state} onDone=${() => call('hub_setup_state').then(setState)} /></div>`;
}

function App() {
  const b = useStore((s) => s.boot);
  const setup = useStore((s) => s.setup);
  if (!b) return html`<div class="splash"><img src="img/mark.svg" alt="" /></div>`;
  if (setup) return html`<${Setup} onDone=${() => boot()} />`;
  return html`<${HubShell} pages=${SHELL_PAGES} Toasts=${Toasts} /><${Migrate} />`;
}

async function start() {
  document.documentElement.classList.add('hub');
  render(html`<${App} />`, document.getElementById('app'));
  const b = await boot().catch((e) => {
    document.getElementById('app').textContent = String(e);
    throw e;
  });
  await setLanguage(b.settings.language);
  watchGame();
  if (SELFTEST) (await import('./selftest.js')).run();
}

start();
