// The page's selftests: run in the app with OMSIHUB_SELFTEST=1 (the app opens the page with
// `?selftest`), or in a browser with `?mock&selftest`. Each check is a name and an async
// function that throws on a failure; the report goes to the app, which prints it and ends
// with its exit code. A port adds its checks to CHECKS.

import { call } from './api.js';
import { t } from './i18n.js';
import { PAGES } from './pages/index.js';
import { get, set, subscribe } from './store.js';

const wait = (ms) => new Promise((r) => setTimeout(r, ms));

/** Wait until `test()` is true (the page drew), at most `ms`. */
async function until(test, ms = 3000) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    if (test()) return;
    await wait(25);
  }
  throw new Error('timed out');
}

function check(ok, what) {
  if (!ok) throw new Error(what);
}

export const CHECKS = [
  ['api: hub_version answers', async () => {
    const v = await call('hub_version');
    check(v.min_openomsi === '0.2.22', 'min_openomsi ' + v.min_openomsi);
  }],
  ['api: an unported command says so', async () => {
    let said = '';
    try { await call('co_state'); } catch (e) { said = String(e); }
    check(said.startsWith('not ported yet'), said);
  }],
  ['store: set and subscribe', async () => {
    let seen = null;
    const off = subscribe((s) => { seen = s.selftestValue; });
    set({ selftestValue: 7 });
    off();
    check(seen === 7 && get().selftestValue === 7, 'seen ' + seen);
  }],
  ['i18n: placeholders are filled', async () => {
    check(t('%{a} of %{b}', { a: 1, b: 2 }) === '1 of 2', t('%{a} of %{b}', { a: 1, b: 2 }));
  }],
  ['setup: the fixture folders are checked and kept', async () => {
    // (the app passes fixtures/ with OMSIHUB_SELFTEST_FIXTURES: the setup filled in as a
    // player would, then the shell)
    const fx = window.__OMSIHUB_FIXTURES__;
    if (!fx || !get().setup) return;
    await until(() => document.querySelector('#setup-openomsi'));
    const type = (sel, value) => {
      const el = document.querySelector(sel);
      el.value = value;
      el.dispatchEvent(new Event('change', { bubbles: true }));
    };
    const said = () => (document.querySelector('#setup-openomsi-check') || {}).textContent || '';
    type('#setup-openomsi', fx.old);
    await until(() => /0\.2\.22/.test(said()));
    check(document.querySelector('#setup-save').disabled, 'saved with an old openOMSI');
    type('#setup-openomsi', fx.openomsi);
    type('#setup-omsi2', fx.omsi2);
    await until(() => !document.querySelector('#setup-save').disabled);
    check(/0\.2\.30/.test(said()), 'the version: ' + said());
    document.querySelector('#setup-save').click();
    await until(() => document.querySelector('#nav-settings'), 5000);
    set({ page: 'settings' });
    await until(() => (document.querySelector('#set-openomsi') || {}).textContent === fx.openomsi);
  }],
  ['shell: every page draws', async () => {
    if (get().setup) {
      // (the folders are not chosen: the setup is what is shown)
      await until(() => document.querySelector('#setup-openomsi'));
      return;
    }
    for (const p of PAGES) {
      set({ page: p.id });
      await until(() => document.querySelector('#nav-' + p.id + '.on') && document.querySelector('.page h1'));
    }
    set({ page: 'home' });
  }],
];

export async function run() {
  await wait(100);
  const results = [];
  for (const [name, fn] of CHECKS) {
    try {
      await fn();
      results.push({ name, ok: true });
    } catch (e) {
      results.push({ name, ok: false, error: String((e && e.message) || e) });
    }
  }
  const report = { ok: results.every((r) => r.ok), results };
  const { selftestReport } = await import('./api.js');
  await selftestReport(report);
  return report;
}
