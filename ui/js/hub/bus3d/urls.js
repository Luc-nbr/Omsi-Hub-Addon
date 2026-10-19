// Where the 3D view's bytes come from: the app's own protocol (`bytes/<cmd>?a=…`,
// `app/src/main.rs`), which the renderer's worker can fetch itself - the window's commands are
// not there. The protocol is `hub://localhost/` on macOS and Linux and `http://hub.localhost/` on
// Windows (as `api.js` has it).
//
// Nothing here touches `window`: a worker imports it too.

const base = /windows/i.test(globalThis.navigator?.userAgent || '') ? 'http://hub.localhost/' : 'hub://localhost/';

/** The address of a byte command with its arguments. */
export function bytesUrl(cmd, args = {}) {
  return `${base}bytes/${encodeURIComponent(cmd)}?a=${encodeURIComponent(JSON.stringify(args))}`;
}

/** Whether the page is the app's (its origin `tauri://localhost`, `http://tauri.localhost` on
 * Windows): a browser on the mock has no `hub` protocol. */
const inApp = /tauri/.test(globalThis.location?.origin || '');

/** The bytes of a byte command; rejects with what the launcher said. */
export async function fetchBytes(cmd, args) {
  if (!inApp) throw new Error(`${cmd}: not in the app`);
  const r = await fetch(bytesUrl(cmd, args));
  if (!r.ok) throw new Error(`${cmd}: ${(await r.text().catch(() => '')) || r.status}`);
  return r.arrayBuffer();
}
