// The editors' part of our self-check (`../selftest.js`, OMSIHUB_SELFTEST): every editor page
// drawn without a script error, their commands answering - the Timetable page's lines of a map,
// the bus's depot files, what a new depot file starts from, the depot editor's list. Nothing
// is saved.

import { call } from '../api.js';
import { S, set } from '../store.js';
import { mapLabel } from './kit.js';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const $ = (q) => document.querySelector(q);

export async function editorsSelftest(check, errors) {
  const c = S.choice;
  // the pages
  for (const id of ['editor', 'timetable', 'depots', 'lines']) {
    const before = errors.length;
    set({ page: id, hub: { ...S.hub, palette: false } });
    await sleep(900);
    const e = errors.length === before ? null : errors.slice(before).join('; ');
    check(`editors: page ${id}`, !e && !!$('.ed-page'), e || $('.ed-page .hub-head h2')?.textContent || 'drawn');
    globalThis.hubProbe?.(`editor ${id}`);
  }
  set({ page: 'drive', hub: { ...S.hub, step: 'mode' } });
  await sleep(300);

  // their commands
  try {
    const d = await call('tt_load', { map: c.map });
    check('editors: the timetable of a map', Array.isArray(d?.lines), `${d?.lines?.length ?? 0} lines of ${c.map}`);
  } catch (x) {
    check('editors: the timetable of a map', false, x.message);
  }
  const map = (S.maps || []).find((m) => m.file === c.map) || S.maps?.[0];
  const bus = (S.vehicles || []).find((v) => v.file === c.bus) || S.vehicles?.[0];
  if (map && bus) {
    try {
      const m = await call('hub_depots', { bus: bus.file, map: { file: map.file, name: map.name, friendly: map.friendly, hof: map.hof || '' }, date: c.date, chosen: '', line: '', maps: (S.maps || []).map((x) => [mapLabel(x), x.hof || '']) });
      check("editors: the bus's depot files", m?.bus === bus.file, `${bus.file}: the map wants ${m?.want || '-'}`);
    } catch (x) {
      check("editors: the bus's depot files", false, x.message);
    }
  }
  if (map) {
    try {
      const list = await call('de_sources', { hof: map.hof || '', map: map.file, bus: bus?.file || '', bus_name: bus?.name || '' });
      check('editors: what a new depot file starts from', list?.[0]?.source?.kind === 'empty', `${list?.length ?? 0} sources`);
    } catch (x) {
      check('editors: what a new depot file starts from', false, x.message);
    }
  }
  try {
    const r = await call('de_show', { key: null, maps: [] });
    check('editors: the depot files of the player', Array.isArray(r?.entries), `${r?.entries?.length ?? 0} files`);
  } catch (x) {
    check('editors: the depot files of the player', false, x.message);
  }
  // (the line editor and the livery studio come later: their pages say so)
}
