// The website's bus company: the app's own company screen (`company/company.js`), alone, over
// hub-api in the browser (`backend.js`). The website's page starts the backend with the cloud
// save, sets it as `window.__OMSIHUB_WEB__` and mounts this; "Back" in the company leaves it
// (`onLeave`), where the website saves it to the cloud again.

import { html, render } from '../../vendor/preact-htm.js';
import { call } from '../api.js';
import { setLanguage } from '../i18n.js';
import { set, useStore } from '../store.js';
import { CompanyScreen, enterCompany } from '../company/company.js';
import { Icon } from '../hub/bits.js';

function Toasts() {
  const toasts = useStore((s) => s.toasts);
  return html`<div class="toasts">${toasts.map((x) => html`<div class=${'toast ' + x.kind}><${Icon} name=${x.kind === 'error' ? 'warning' : x.kind === 'ok' ? 'check' : 'info'} size=${18} /><div class="selectable">${x.text}</div></div>`)}</div>`;
}

function Web({ onLeave }) {
  const page = useStore((s) => s.page);
  if (page !== 'company') {
    queueMicrotask(() => onLeave?.());
    return null;
  }
  return html`<${CompanyScreen} /><${Toasts} />`;
}

/** The company `id` (laid out by `backend.install`) opened in `el`. */
export async function mount(el, { id, onLeave } = {}) {
  document.documentElement.classList.add('hub');
  const b = await call('hub_boot');
  set({ boot: b, setup: false });
  await setLanguage(b.settings.language);
  enterCompany({ chooser: null, want: id || null });
  render(html`<${Web} onLeave=${onLeave} />`, el);
}

/** The language of the pages, changed (the website's language buttons). */
export async function language(l) {
  await call('hub_settings_patch', { patch: { language: l } }).catch(() => {});
  await setLanguage(l);
}
