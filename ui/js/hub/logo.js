// Omsi-Hub's logo in the page (the bar, the start, the company's side bar), from our mark
// (assets/brand, `mark.js`): the ring and the line in the accent chosen, the bus and "Omsi-" in
// the look's ink (`--hub-logo-ink`: light on the dark look, dark on the light one), "Hub" and
// the stops as the brand drawing has them.

import { MARK } from './mark.js';

export { MARK };

const stop = ([x, was, inner]) => (x === was ? inner : `<g transform="translate(${(x - was).toFixed(1)} 0)">${inner}</g>`);

/** The logo as one picture. */
export const LOGO_SVG =
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${MARK.view.join(' ')}" class="hub-logo" role="img" aria-label="Omsi-Hub">` +
  `<path d="${MARK.route}" fill="none" style="stroke:var(--hub-accent-chosen)" stroke-width="${MARK.stroke}" stroke-linecap="round"/>` +
  `<path d="${MARK.bus}" style="fill:var(--hub-logo-ink)"/>` +
  `<path d="${MARK.words[0][0][2]}" style="fill:var(--hub-logo-ink)"/>` +
  `<path d="${MARK.words[1][0][2]}" fill="#F58620" stroke="#F58620" stroke-width="7" stroke-linejoin="round"/>` +
  MARK.stops.map(stop).join('') +
  '</svg>';
