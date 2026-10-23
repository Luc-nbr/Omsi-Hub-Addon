// The word that the app is busy: whenever something the player set going takes long - a page
// read, a line paid, a map opened, a bus loaded - a small card over the window says so, a spinner
// and what it is doing, until it is done (`api.js` says what is waited for). Not for a moment's
// work: it comes after `SHOW_AFTER`, and once up it stays `SHOW_AT_LEAST`, so it does not flicker.
// It takes no clicks: the page under it goes on.

import { html, useEffect, useState } from '../../vendor/preact-htm.js';
import { onWaiting } from '../api.js';
import { Spinner } from '../ui.js';
import { tr } from './bits.js';

const SHOW_AFTER = 350;
const SHOW_AT_LEAST = 400;

/** What a command does, in a word: saving (paying too), else reading. */
function wordOf({ cmd, args }) {
  const what = `${cmd} ${args?.op || ''} ${args?.act || ''} ${args?.what || ''}`.toLowerCase();
  if (/save|pay|confirm|commit|withdraw|discard|keep|apply/.test(what)) return tr('Saving…');
  return tr('Loading...');
}

export function BusyCard() {
  const [shown, setShown] = useState(null);
  useEffect(() => {
    let timer = 0;
    let since = 0;
    let up = null;
    const show = (w) => {
      up = w;
      since = performance.now();
      setShown(w);
    };
    const look = (waiting) => {
      clearTimeout(timer);
      const now = performance.now();
      const oldest = [...waiting.values()].sort((a, b) => a.at - b.at)[0];
      if (oldest) {
        const word = wordOf(oldest);
        const due = oldest.at + SHOW_AFTER - now;
        if (up) {
          if (up !== word) show(word);
        } else if (due <= 0) show(word);
        else timer = setTimeout(() => look(waiting), due);
        return;
      }
      if (!up) return;
      const left = since + SHOW_AT_LEAST - now;
      const hide = () => {
        up = null;
        setShown(null);
      };
      if (left <= 0) hide();
      else timer = setTimeout(() => (waiting.size ? look(waiting) : hide()), left);
    };
    const off = onWaiting(look);
    return () => {
      off();
      clearTimeout(timer);
    };
  }, []);
  if (!shown) return null;
  return html`<div class="hub-busy" role="status" aria-live="polite"><${Spinner} /><span>${shown}</span></div>`;
}
