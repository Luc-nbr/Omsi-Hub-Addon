// The browser mock of hub-api: what the app's Rust answers, faked, so the page can be opened
// in a plain browser (ui/index.html, or `?mock`). Kept small: a port adds the answers its page
// needs to `answers` below. The first-run setup accepts any folder whose name contains
// "openOMSI" (a version in the name, as `openOMSI-0.2.30`) and "OMSI 2".

const listeners = {};
const settings = { openomsi_folder: '', omsi2_folder: '', language: '', setup_done: false, migrated: false };

function versionIn(folder) {
  const m = /(\d+)\.(\d+)(?:\.(\d+))?/.exec(folder);
  return m ? [Number(m[1]), Number(m[2]), Number(m[3] || 0)] : null;
}

function older(a, b) {
  for (let i = 0; i < 3; i++) if (a[i] !== b[i]) return a[i] < b[i];
  return false;
}

function checkOpenomsi(folder) {
  if (!/openomsi/i.test(folder || '')) return { ok: false, problem: 'no_game', text: 'No openOMSI was found in this folder' };
  const v = versionIn(folder);
  const tooOld = !!v && older(v, [0, 2, 22]);
  return {
    ok: !tooOld,
    too_old: tooOld,
    version: v ? v.join('.') : null,
    version_from: v ? 'folder_name' : null,
    text: tooOld ? `This openOMSI is ${v.join('.')}: Omsi-Hub needs 0.2.22 or newer. Update openOMSI.` : '',
    compat: { level: tooOld ? 'unsupported' : v ? 'supported' : 'unknown', features: [], notes: [] },
  };
}

function checkOmsi2(folder) {
  const ok = /omsi ?2/i.test(folder || '');
  return { ok, missing: ok ? [] : ['Omsi.exe'], text: ok ? '' : 'This is not the OMSI 2 folder: Omsi.exe not found' };
}

const answers = {
  hub_version: () => ({ app: '0.1.0', min_openomsi: '0.2.22', plugin: '1.6.0', bridge: 1 }),
  hub_boot: () => ({ version: answers.hub_version(), data_dir: '(mock)', settings, setup_needed: !settings.setup_done, languages: ['en', 'nl', 'de'] }),
  hub_texts: () => ({}),
  hub_setup_state: () => ({ settings, openomsi: null, omsi2: null, migrate: { offer: false, companies: [], careers: [], files: 0 } }),
  hub_setup_suggest: () => ({ openomsi: [], omsi2: [] }),
  hub_setup_openomsi: (a) => checkOpenomsi(a.folder),
  hub_setup_omsi2: (a) => checkOmsi2(a.folder),
  hub_setup_save: (a) => {
    const o = checkOpenomsi(a.openomsi);
    if (!o.ok) throw o.text;
    const r = checkOmsi2(a.omsi2);
    if (!r.ok) throw r.text;
    Object.assign(settings, { openomsi_folder: a.openomsi, omsi2_folder: a.omsi2, setup_done: true });
    if (a.language) settings.language = a.language;
    return { ok: true, settings, openomsi: o };
  },
  hub_migrate_skip: () => ({ ok: true }),
  hub_game_state: () => ({ running: false }),
  hub_log: () => null,
  addon_state: () => ({ folder: settings.openomsi_folder, game: checkOpenomsi(settings.openomsi_folder), plugin: { state: 'missing' }, plugin_version: '1.6.0' }),
};

export const backend = {
  async call(cmd, args) {
    const a = answers[cmd];
    if (!a) throw `not ported yet: ${cmd}`;
    return a(args || {});
  },
  async callBytes(cmd) {
    throw `not ported yet: ${cmd}`;
  },
  on(event, fn) {
    (listeners[event] ||= []).push(fn);
    return () => { listeners[event] = listeners[event].filter((f) => f !== fn); };
  },
  /** For a page's work: pretend Rust sent `event`. */
  emit(event, payload) {
    for (const f of listeners[event] || []) f(payload);
  },
};
