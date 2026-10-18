// The bus company on the website (omsihub.com): hub-api in the browser. The module is
// `crates/hub-web`, built for wasm32-wasip1; WASI's folders are kept in memory
// (`vendor/browser_wasi_shim`): `/data`, Omsi-Hub's data folder with the cloud save laid out in
// it, and `/omsi`, the OMSI 2 content the desktop app sent with the save. The pages ask it as
// they ask the app (`api.js` with `window.__OMSIHUB_WEB__`): the same commands, the same answers.

import { WASI, File, Directory, PreopenDirectory, OpenFile, ConsoleStdout } from '../../vendor/browser_wasi_shim/index.js';

const enc = new TextEncoder();
const dec = new TextDecoder();

/** A folder in memory holding `files` (`{ 'a/b.json': Uint8Array }`, `/` between the parts). */
function folder(files = {}) {
  const root = new Directory(new Map());
  for (const [path, bytes] of Object.entries(files)) {
    const parts = path.split('/').filter(Boolean);
    let dir = root;
    for (const part of parts.slice(0, -1)) {
      let next = dir.contents.get(part);
      if (!(next instanceof Directory)) {
        next = new Directory(new Map());
        next.parent = dir;
        dir.contents.set(part, next);
      }
      dir = next;
    }
    dir.contents.set(parts.at(-1), new File(bytes));
  }
  return root;
}

/** Every file under `dir`, as `folder` takes them. */
function filesOf(dir, at = '', out = {}) {
  for (const [name, node] of dir.contents) {
    const path = at ? `${at}/${name}` : name;
    if (node instanceof Directory) filesOf(node, path, out);
    else if (node instanceof File) out[path] = node.data;
  }
  return out;
}

/** The module started: `wasm` (its URL or bytes), `data` and `omsi` (the folders' files).
 * Answers the backend the pages call (`call`, `callBytes`, `on`) and the save's own steps
 * (`install`, `pack`, `files`). */
export async function start({ wasm, data = {}, omsi = {} }) {
  const dirs = { data: folder(data), omsi: folder(omsi), home: folder() };
  const fds = [
    new OpenFile(new File([])),
    ConsoleStdout.lineBuffered((l) => console.log('[company]', l)),
    ConsoleStdout.lineBuffered((l) => console.warn('[company]', l)),
    new PreopenDirectory('/data', dirs.data.contents),
    new PreopenDirectory('/omsi', dirs.omsi.contents),
    new PreopenDirectory('/home', dirs.home.contents),
  ];
  // (the preopens hold the folders themselves: what the module writes is in `dirs`)
  dirs.data = fds[3].dir;
  const wasi = new WASI([], [], fds, { debug: false });
  const source = typeof wasm === 'string' ? fetch(wasm) : null;
  const { instance } = source && WebAssembly.instantiateStreaming
    ? await WebAssembly.instantiateStreaming(source, { wasi_snapshot_preview1: wasi.wasiImport })
    : await WebAssembly.instantiate(source ? await (await source).arrayBuffer() : wasm, { wasi_snapshot_preview1: wasi.wasiImport });
  wasi.initialize(instance);
  const x = instance.exports;
  let dead = null;

  const put = (b) => {
    if (typeof b === 'string') b = enc.encode(b);
    const p = x.hub_alloc(b.length);
    new Uint8Array(x.memory.buffer, p, b.length).set(b);
    return [p, b.length];
  };
  const out = () => new Uint8Array(x.memory.buffer, x.hub_out(), x.hub_out_len()).slice();
  /** One call into the module, its memory given back; a panic ends the module for good. */
  const run = (f, ...parts) => {
    if (dead) throw new Error(dead);
    const held = parts.map(put);
    try {
      f(...held.flat());
    } catch (e) {
      dead = `The company stopped (${e.message}). Reload the page.`;
      throw new Error(dead);
    } finally {
      if (!dead) for (const [p, l] of held) x.hub_free(p, l);
    }
    return out();
  };

  const listeners = new Map();
  const tell = () => {
    const list = JSON.parse(dec.decode(run(x.hub_events)) || '[]');
    for (const { event, payload } of list) for (const fn of listeners.get(event) || []) fn(payload);
  };

  return {
    /** A command of hub-api, as `api.js` `call`: its answer, or a rejection with its text. */
    async call(cmd, args = {}) {
      const r = JSON.parse(dec.decode(run(x.hub_call, cmd, JSON.stringify(args ?? {}))));
      tell();
      if ('err' in r) throw r.err;
      return r.ok;
    },
    /** A command that answers bytes: an ArrayBuffer. */
    async callBytes(cmd, args = {}) {
      const b = run(x.hub_call_bytes, cmd, JSON.stringify(args ?? {}));
      tell();
      if (b[0] !== 1) throw dec.decode(b.slice(1));
      return b.slice(1).buffer;
    },
    /** Listen to what the commands tell the page: `fn(payload)`; answers the unlisten. */
    async on(event, fn) {
      if (!listeners.has(event)) listeners.set(event, new Set());
      listeners.get(event).add(fn);
      return () => listeners.get(event).delete(fn);
    },
    /** A cloud save (`v<n>.omsave`, its bytes) laid out in the data folder: `{ id, name, profile }`. */
    install(save, { language = '' } = {}) {
      const r = JSON.parse(dec.decode(run(x.hub_install, save instanceof Uint8Array ? save : new Uint8Array(save), JSON.stringify({ language }))));
      if ('err' in r) throw new Error(r.err);
      return r.ok;
    },
    /** Company `id` packed as a cloud save again: `{ bytes, sha256 }`. */
    pack(id) {
      const b = run(x.hub_pack, id);
      if (b[0] !== 1) throw new Error(dec.decode(b.slice(1)));
      const bytes = b.slice(1);
      return { bytes, sha256: dec.decode(run(x.hub_sha256)) };
    },
    /** The data folder's files as they are now (what the company wrote). */
    files: () => filesOf(dirs.data),
    /** The module stopped (a panic): why, or null. */
    dead: () => dead,
  };
}
