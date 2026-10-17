// Omsi-Hub online in the shell: what the page mounts, and where.
//
// * `OnlineButton` - the account button: in the top bar, beside the other bar buttons. It shows
//   the account (signed out) or the friends with a badge for new requests and invites, and opens
//   and closes the panel.
// * `OnlineLayer` - once, at the shell's root (after <main>): the panel the button opens, the
//   invites' notices over every page, and, for a signed-in account that has not accepted the
//   current Privacy Statement (an old account, or an amended statement), the question over
//   everything.
// * `OnlinePage` - the Online page itself (ui/js/pages/online.js renders it): the account and the
//   friends, inline.
// * `openPanel(tab)` - the panel opened from elsewhere (`account` or `friends`).
// * `CloudBadge`, `cloudBeforeOpen(id)` - for the company's pages (see cloud.js).
//
// The updates live beside it in ../update.js (`UpdateDialog`, `UpdatedNotice`, `useUpdates`,
// `UpdateRow`).

import { html } from '../../vendor/preact-htm.js';
import { useLang } from '../i18n.js';
import { OnlineButton, OnlineLayer, openPanel, Panel, useOnline, inviteFriend } from './online.js';
import { CloudBadge, cloudBeforeOpen, CloudSaves } from './cloud.js';
import { tr } from './bits.js';

export { OnlineButton, OnlineLayer, openPanel, useOnline, inviteFriend, CloudBadge, cloudBeforeOpen, CloudSaves };

/** The Online page: the account and the friends, inline. */
export function OnlinePage() {
  useLang();
  const v = useOnline();
  return html`<section class="page ol-page">
    <header class="page-head"><h1>${tr('Online')}</h1><p class="sub">${tr('Friends, the leaderboard and cloud saves. Optional: everything works offline.')}</p></header>
    <${Panel} v=${v} inline />
  </section>`;
}
