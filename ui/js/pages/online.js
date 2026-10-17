// Online: the account, friends, the leaderboard and the cloud saves (the old
// launcher/ui/js/omsihub/online.js, cloud.js), on `ol_` and `cs_`: the online part's page.

import { html } from '../../vendor/preact-htm.js';
import { useLang } from '../i18n.js';
import { OnlinePage } from '../online/index.js';

export function Online() {
  useLang();
  return html`<${OnlinePage} />`;
}
