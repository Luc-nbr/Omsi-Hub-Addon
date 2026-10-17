// The editors: the timetable and depot editors first (the old launcher/ui/js/omsihub/editors/*,
// on `tt_` and `de_`); the line editor and the livery studio later.

import { html } from '../../vendor/preact-htm.js';
import { t, useLang } from '../i18n.js';
import { Placeholder } from '../ui.js';

export function Editors() {
  useLang();
  return html`<${Placeholder} title=${t('Editors')} sub=${t('Timetables and depots.')} what="tt_ · de_" />`;
}
