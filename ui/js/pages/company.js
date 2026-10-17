// The bus company and RosterFlow: the target of the company UI port (the old
// launcher/ui/js/omsihub/company/*, rosterflow/*), on the `co_` and `rf_` commands.

import { html } from '../../vendor/preact-htm.js';
import { t, useLang } from '../i18n.js';
import { Placeholder } from '../ui.js';

export function Company() {
  useLang();
  return html`<${Placeholder} title=${t('Bus company')} sub=${t('Your own transport company, with RosterFlow for the rosters.')} what="co_ · rf_" />`;
}
