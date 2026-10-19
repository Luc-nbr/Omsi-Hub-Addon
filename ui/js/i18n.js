// The page's texts: the English text is the key (as in hub-locales' tables), `t(text, vars)`
// gives it in the player's language, with `%{name}` filled from `vars`. The tables come from
// Rust (`hub_texts {lang}`); a text without a translation is shown in English.

import { call } from './api.js';
import { set, useStore } from './store.js';

let table = {};
let lang = 'en';

/** The language's texts loaded (and the page drawn again). */
export async function setLanguage(next) {
  const l = (next || systemLanguage()).toLowerCase();
  try {
    table = l === 'en' ? {} : await call('hub_texts', { lang: l });
  } catch (e) {
    console.warn('texts', e);
    table = {};
  }
  lang = l;
  document.documentElement.lang = l;
  set({ lang: l });
}

/** The system's language, two letters. */
export function systemLanguage() {
  return (navigator.language || 'en').slice(0, 2).toLowerCase();
}

/** `text` in the player's language, `%{name}` filled from `vars`. */
export function t(text, vars) {
  let s = table[text] || text;
  if (vars) s = s.replace(/%\{(\w+)\}/g, (_, k) => (vars[k] == null ? '' : String(vars[k])));
  return s;
}

/** In a component: draws it again when the language changes. */
export function useLang() {
  return useStore((s) => s.lang);
}

export function language() {
  return lang;
}

// Every kind of dash and the raised dot as a plain hyphen, in the texts drawn on a canvas (the
// route map's labels: names from the content too).
const DASHES = /[\u2010-\u2015\u2212\u2E3A\u2E3B\uFE58\uFF0D\u00B7]/g;
export const plain = (text) => (typeof text === 'string' ? text.replace(DASHES, '-') : text);
