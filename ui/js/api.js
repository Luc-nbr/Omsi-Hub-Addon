// The page's line to Rust: `call(cmd, args)` answers a command of hub-api as JSON,
// `callBytes` as bytes, `bytesUrl` gives a URL of the app's bytes route for an <img>.
//
// In the app the calls go through Tauri (`window.__TAURI__`, the app's `withGlobalTauri`).
// In a plain browser (opening ui/index.html for a look, or `?mock` in the app) they go to
// the mock backend (`mock.js`), so the pages can be worked on without the app.

const tauri = window.__TAURI__;
const params = new URLSearchParams(location.search);

/** Running in the app (else: the browser mock). */
export const inApp = !!(tauri && tauri.core) && !params.has('mock');
/** (the old launcher's names, for the ported pages: the app is never the phone's) */
export const inTauri = inApp;
export const onPhone = false;

/** A command's error: its text, also as `String(e)` (the frame's pages show that) and as
 * `e.message` (the ported pages read that). */
export class CallError extends Error {
  toString() {
    return this.message;
  }
}
const failed = (e) => {
  throw e instanceof CallError ? e : new CallError(typeof e === 'string' ? e : (e && e.message) || String(e));
};

// (the window's own commands the ported pages call through `call`, as the old launcher let
// them: they go to the helpers below)
const OWN = {
  pick_folder: (a) => pickFolder(a.title),
  pick_file: (a) => pickFile(a.title, a.filters),
  open_url: (a) => openUrl(a.url),
  open_path: (a) => openPath(a.path),
};

let mock = null;

async function mockBackend() {
  if (!mock) mock = (await import('./mock.js')).backend;
  return mock;
}

/** A command of hub-api (`hub_boot`, `co_state` …); `args` is one object. Rejects with the
 *  command's error text. */
export async function call(cmd, args = {}) {
  if (OWN[cmd]) return OWN[cmd](args || {});
  if (inApp) return tauri.core.invoke('call', { cmd, args }).catch(failed);
  const b = await mockBackend();
  return b.call(cmd, args).catch(failed);
}

/** A command that answers bytes: an ArrayBuffer. */
export async function callBytes(cmd, args = {}) {
  if (inApp) return tauri.core.invoke('call_bytes', { cmd, args }).catch(failed);
  const b = await mockBackend();
  return b.callBytes(cmd, args).catch(failed);
}

/** The URL of a bytes command, for an <img src>: the app's `hub` protocol
 *  (`hub://localhost/bytes/<cmd>`, `http://hub.localhost/…` on Windows). */
export function bytesUrl(cmd, args = {}) {
  const q = '?a=' + encodeURIComponent(JSON.stringify(args));
  if (!inApp) return 'data:,';
  const windows = /Windows/.test(navigator.userAgent);
  return (windows ? 'http://hub.localhost/bytes/' : 'hub://localhost/bytes/') + encodeURIComponent(cmd) + q;
}

/** Listen to an event Rust sends (`game-ended` …): `fn(payload)`. Returns the unlisten. */
export async function on(event, fn) {
  if (inApp && tauri.event) return tauri.event.listen(event, (e) => fn(e.payload));
  const b = await mockBackend();
  return b.on(event, fn);
}

/** The system's folder picker: the path, or null. */
export async function pickFolder(title = '') {
  if (inApp) return tauri.core.invoke('pick_folder', { title });
  return window.prompt(title || 'Folder', '') || null;
}

/** The system's file picker: the path, or null. `filters`: [{name, extensions: ['cfg']}]. */
export async function pickFile(title = '', filters = []) {
  if (inApp) return tauri.core.invoke('pick_file', { title, filters });
  return window.prompt(title || 'File', '') || null;
}

/** A web page in the system's browser. */
export async function openUrl(url) {
  if (inApp) return tauri.core.invoke('open_url', { url });
  window.open(url, '_blank', 'noopener');
}

/** A file or folder in the system's file manager. */
export async function openPath(path) {
  if (inApp) return tauri.core.invoke('open_path', { path });
  console.info('open', path);
}

/** The selftest's report to the app (which prints it and ends). */
export async function selftestReport(report) {
  if (inApp) return tauri.core.invoke('selftest_report', { report });
  console.info('selftest', report);
}
