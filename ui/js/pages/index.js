// The pages of the shell, in the order the navigation lists them. A port plugs in here: its
// page module in ui/js/pages/ (or its own folder under ui/js/), one entry below.

import { Home } from './home.js';
import { Company } from './company.js';
import { Editors } from './editors.js';
import { Online } from './online.js';
import { Settings } from './settings.js';

/** `id` (the store's `page`), `label` (English: translated by the shell), `component`. */
export const PAGES = [
  { id: 'home', label: 'Home', component: Home },
  { id: 'company', label: 'Bus company', component: Company },
  { id: 'editors', label: 'Editors', component: Editors },
  { id: 'online', label: 'Online', component: Online },
  { id: 'settings', label: 'Settings', component: Settings },
];
