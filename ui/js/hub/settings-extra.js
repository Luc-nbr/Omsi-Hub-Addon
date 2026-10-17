// The Settings page's tabs and rows (`pages/settings.js` draws them): the look - light or dark,
// the accent, the launcher's size, the animations, the company's layout - and the language;
// openOMSI and OMSI 2 (the folders, the version, the plugin: `addon.js`); the updates; and the
// bus models' categories and regions the company goes by. Every change is saved at once
// (`saveSettings`).
//
// A row is `{ type, key, label, … }`: `toggle`, `slider` (`min`, `max`, `step`, `format`,
// `store`), `select` (`options` [value, label]), `button` (`patch()` returns the settings to
// change, or does something itself), or `custom` (`view()` draws it).

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { PRESETS, hexOf } from './theme.js';
import { UpdateRow } from './update.js';
import { AddonPanel } from './addon.js';

const pct = (v) => `${Math.round(v * 100)} %`;

/** The language names as each language writes its own. */
export const LANGUAGE_NAMES = { en: 'English', de: 'Deutsch', fr: 'Français', nl: 'Nederlands', pl: 'Polski', ru: 'Русский', uk: 'Українська', cs: 'Čeština', it: 'Italiano', es: 'Español' };

export const SETTINGS_TABS = [
  {
    id: 'general',
    label: 'General',
    cols: [
      {
        title: 'Look',
        rows: [
          { type: 'slider', key: 'launcher_scale', label: 'Launcher size', min: 0.6, max: 2, step: 0.05, def: 1, format: pct, store: (v) => Math.round(v * 20) / 20 },
          { type: 'toggle', key: 'animations', label: 'Animations' },
          { type: 'select', key: 'accent', label: 'Accent colour', options: PRESETS.map(([name, c]) => [hexOf(c), name]) },
          { type: 'toggle', key: 'dark_mode', label: 'Dark mode' },
          // (the bus company's pages round a sidebar, or found with the command palette: `company/shell.js`)
          { type: 'select', key: 'company_layout', label: 'Company layout', options: [['sidebar', 'Sidebar'], ['palette', 'Command palette']] },
          { type: 'button', label: 'Start the tour', icon: 'route', patch: () => (import('./hub.js').then((h) => (h.go('drive', 'mode'), import('./tour.js').then((m) => setTimeout(() => m.startTour(), 300)))), {}) },
        ],
      },
      {
        title: 'Language',
        rows: [{ type: 'language', key: 'language', label: 'Language' }],
      },
    ],
  },
  { id: 'addon', label: 'openOMSI and OMSI 2', cols: [{ title: 'Your openOMSI and OMSI 2', rows: [{ type: 'custom', id: 'hub-addon', view: () => html`<${AddonPanel} />` }] }] },
  { id: 'updates', label: 'Updates', cols: [{ title: 'Updates', rows: [{ type: 'custom', id: 'hub-update', view: () => html`<${UpdateRow} />` }] }] },
  // (the bus models' categories and regions, for every company: what the dealer, the market and
  // the shop of the bus company go by, `company/vehicles.js`)
  { id: 'vehicles', label: 'Vehicles', cols: [{ title: 'Bus categories and regions', rows: [{ type: 'custom', id: 'hub-vehicles', view: () => html`<${Vehicles} />` }] }] },
];

/** The models' categories and regions (loaded with the tab: the company's list). */
function Vehicles() {
  const [m, setM] = useState(null);
  useEffect(() => {
    import('../company/vehicles.js').then(setM).catch(() => {});
  }, []);
  return m ? html`<${m.VehicleClasses} />` : null;
}
