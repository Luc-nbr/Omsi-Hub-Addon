// The page's selftests: run in the app with OMSIHUB_SELFTEST=1 (the app opens the page with
// `?selftest`), or in a browser with `?mock&selftest`. Each check is a name and an async
// function that throws on a failure; the report goes to the app, which prints it and ends
// with its exit code. A port adds its checks to CHECKS.
//
// The ported pages bring suites of their own (SUITES: the drive's start and steps, the editors,
// the bus company, RosterFlow): each is `suite(check, errors)`, where `check(name, ok, detail)`
// tells one result and `errors` holds the page's script errors while it runs. They go through
// every page and state, which takes minutes and needs the company's and the drive's commands:
// they run in the browser mock, and in the app with `?selftest=full`.

import { call, inApp } from './api.js';
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

/** What each page of the shell shows once it is drawn. */
const DRAWN = {
  drive: '.hub-start',
  company: '.co',
  editor: '.ed-page .hub-head h2',
};

export const CHECKS = [
  ['api: hub_version answers', async () => {
    const v = await call('hub_version');
    check(v.min_openomsi === '0.2.22', 'min_openomsi ' + v.min_openomsi);
  }],
  ['api: an unported command says so', async () => {
    let said = '';
    try { await call('tg_status'); } catch (e) { said = String(e); }
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
    await until(() => document.querySelector('.hub-bar'), 5000);
    set({ page: 'settings' });
    // (the folders are on the second tab: openOMSI and OMSI 2)
    await until(() => document.querySelectorAll('#settings .hub-seg button').length > 1);
    document.querySelectorAll('#settings .hub-seg button')[1].click();
    await until(() => (document.querySelector('#set-openomsi') || {}).textContent === fx.openomsi);
    set({ page: 'drive' });
  }],
  ['shell: every page draws', async () => {
    if (get().setup) {
      // (the folders are not chosen: the setup is what is shown)
      await until(() => document.querySelector('#setup-openomsi'));
      return;
    }
    for (const p of PAGES) {
      set({ page: p.id });
      // (the company's first visit loads its style sheets and reads the companies: a while)
      await until(() => document.querySelector('.hub-stage') && document.querySelector(DRAWN[p.id] || '.hub-sheet .hub-head h2'), 10000).catch(() => {
        throw new Error(`page ${p.id}: ${(document.querySelector('#app')?.innerText || '').slice(0, 120)}`);
      });
    }
    set({ page: 'drive', hub: { ...get().hub, step: 'mode' } });
  }],
];

/** The ported pages' own suites, in the order they run (the drive's suite runs the editors',
 * the company's and RosterFlow's after its own). */
export const SUITES = [['hub', () => import('./hub/selftest.js').then((m) => m.selftest)]];

/** Whether the suites run: in the browser mock, or asked for (`?selftest=full`). */
const suitesWanted = () => !inApp || new URLSearchParams(location.search).get('selftest') === 'full';

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
  if (suitesWanted() && !get().setup) {
    const errors = [];
    const onError = (e) => errors.push(String(e.message || e.reason?.message || e.reason || e));
    addEventListener('error', onError);
    addEventListener('unhandledrejection', onError);
    for (const [suite, load] of SUITES) {
      const tell = (name, ok, detail) => results.push({ name, ok: !!ok, ...(ok ? { detail: String(detail ?? '') } : { error: String(detail ?? 'failed') }) });
      try {
        await (await load())(tell, errors);
      } catch (e) {
        results.push({ name: `${suite}: the suite ran to its end`, ok: false, error: String((e && e.message) || e) });
      }
    }
    removeEventListener('error', onError);
    removeEventListener('unhandledrejection', onError);
  }
  const report = { ok: results.every((r) => r.ok), results };
  window.__OMSIHUB_SELFTEST_REPORT__ = report;
  const { selftestReport } = await import('./api.js');
  await selftestReport(report);
  return report;
}
