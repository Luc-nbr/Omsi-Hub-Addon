// The online account's places in the shell (the bar's button, the invites over every page, the
// company's friends), from the online part (`../online/index.js`), under the names the ported
// pages use.

import { useOnline, openPanel } from '../online/index.js';

export { OnlineButton, OnlineLayer, inviteFriend } from '../online/index.js';
/** The account as the company's pages ask for it (`ol_state`: `mode`, `friends`, `offline` …). */
export const useOnlineState = useOnline;
/** The account's panel opened at `tab` (`account`, `friends`). */
export const openOnline = (tab) => openPanel(tab);
