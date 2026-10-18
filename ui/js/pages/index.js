// The pages of the shell. A port plugs in here: its page module in ui/js/pages/ (or its own
// folder under ui/js/), one entry below. The shell (`hub/hub.js`) draws the drive's start and
// steps (`drive`), the bus company over the whole window (`company`) and the editors itself;
// every other page goes into the wide sheet under the bar, with its icon, title and line.

import { Online } from './online.js';
import { Settings } from './settings.js';
import { ProfilePage } from './profile.js';
import { MultiplayerPage } from './multiplayer.js';

/** `id` (the store's `page`), `label` (English: translated by the shell), `icon` (a Material
 * Symbol), `sub` (the line under the title), `component` (none: the shell's own). */
export const PAGES = [
  { id: 'drive', label: 'Home', icon: 'home' },
  { id: 'company', label: 'Bus company', icon: 'garage' },
  { id: 'editor', label: 'Editors', icon: 'construction' },
  { id: 'online', label: 'Online', icon: 'public', sub: 'Friends, the leaderboard and cloud saves. Optional: everything works offline.', component: Online },
  { id: 'multiplayer', label: 'Multiplayer', icon: 'groups', sub: 'Drive with friends by a code, or on servers that are always on.', component: MultiplayerPage },
  { id: 'profile', label: 'Service record', icon: 'badge', sub: 'Everything the logbook kept, from the first duty to the last.', component: ProfilePage },
  { id: 'settings', label: 'Settings', icon: 'settings', sub: 'Every change is saved at once.', component: Settings },
];
