// Online: the account, friends, the leaderboard and the cloud saves (the old
// launcher/ui/js/omsihub/online.js, cloud.js), on `ol_` and `cs_`.

import { html } from '../../vendor/preact-htm.js';
import { t, useLang } from '../i18n.js';
import { Placeholder } from '../ui.js';

export function Online() {
  useLang();
  return html`<${Placeholder} title=${t('Online')} sub=${t('Friends, the leaderboard and cloud saves. Optional: everything works offline.')} what="ol_ · cs_" />`;
}
