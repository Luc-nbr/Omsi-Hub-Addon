// The bus stop signs the maps mark the stops with, in the style the player chose: the old app's
// setting `stop_style` (its navigator's and its companion's, `assets/companion/map.js`'s
// `stopSign`, drawn here the same), asked in the welcome; German by default. The German stop's
// green H on yellow (Zeichen 224), the British red roundel with its bar, the French blue square
// with the bus (C6), and - new in the add-on - the Dutch dark blue stop plate with the bus over
// its yellow band.

import { html, useEffect, useRef } from '../../vendor/preact-htm.js';
import { S } from '../store.js';

/** The styles in the order they are offered: the setting's value, the English name (translated
 * where shown). */
export const STOP_SIGNS = [
  ['de', 'German (H)'],
  ['uk', 'British'],
  ['fr', 'French'],
  ['nl', 'Dutch'],
];

/** The style chosen (the settings'), German unless another is. */
export const stopSign = () => (STOP_SIGNS.some(([id]) => id === S.settings?.stop_style) ? S.settings.stop_style : 'de');

function roundRect(ctx, x, y, w, h, r) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}

/** A bus seen from the side in white on a sign `s` half wide, its windows `glass`: body,
 * windows, wheels. */
function bus(ctx, s, glass, dy = 0) {
  ctx.fillStyle = '#ffffff';
  roundRect(ctx, -s * 0.66, -s * 0.46 + dy, s * 1.32, s * 0.78, s * 0.14);
  ctx.fill();
  ctx.fillStyle = glass;
  ctx.fillRect(-s * 0.54, -s * 0.34 + dy, s * 0.3, s * 0.26);
  ctx.fillRect(-s * 0.16, -s * 0.34 + dy, s * 0.3, s * 0.26);
  ctx.fillRect(s * 0.22, -s * 0.34 + dy, s * 0.32, s * 0.26);
  ctx.fillStyle = '#ffffff';
  ctx.beginPath();
  ctx.arc(-s * 0.36, s * 0.38 + dy, s * 0.13, 0, Math.PI * 2);
  ctx.fill();
  ctx.beginPath();
  ctx.arc(s * 0.36, s * 0.38 + dy, s * 0.13, 0, Math.PI * 2);
  ctx.fill();
}

/** The sign of `style`, `d` pixels across, its middle at `x`, `y` (the companion's
 * `stopSign`). */
export function drawStopSign(ctx, style, x, y, d) {
  const r = d / 2;
  const edge = () => {
    ctx.lineWidth = Math.max(1, d * 0.06);
    ctx.strokeStyle = 'rgba(8, 10, 14, 0.9)';
  };
  ctx.save();
  ctx.translate(x, y);
  if (style === 'uk') {
    ctx.fillStyle = '#ffffff';
    ctx.beginPath();
    ctx.arc(0, 0, r, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = '#da291c';
    ctx.lineWidth = r * 0.36;
    ctx.beginPath();
    ctx.arc(0, 0, r * 0.66, 0, Math.PI * 2);
    ctx.stroke();
    ctx.fillStyle = '#da291c';
    ctx.fillRect(-r * 0.98, -r * 0.2, r * 1.96, r * 0.4);
    ctx.fillStyle = '#ffffff';
    ctx.fillRect(-r * 0.62, -r * 0.07, r * 1.24, r * 0.14);
    edge();
    ctx.beginPath();
    ctx.arc(0, 0, r, 0, Math.PI * 2);
    ctx.stroke();
  } else if (style === 'fr') {
    const s = r * 0.92;
    roundRect(ctx, -s, -s, s * 2, s * 2, s * 0.32);
    ctx.fillStyle = '#1f5fbf';
    ctx.fill();
    edge();
    ctx.stroke();
    bus(ctx, s, '#1f5fbf');
  } else if (style === 'nl') {
    // (the Dutch plate: taller than wide, the bus over a yellow band)
    const s = r * 0.92;
    const w = s * 1.7;
    roundRect(ctx, -w / 2, -s, w, s * 2, s * 0.18);
    ctx.fillStyle = '#183f8f';
    ctx.fill();
    edge();
    ctx.stroke();
    bus(ctx, s * 0.82, '#183f8f', -s * 0.2);
    ctx.fillStyle = '#f7d117';
    ctx.fillRect(-w / 2 + s * 0.18, s * 0.56, w - s * 0.36, s * 0.24);
  } else {
    ctx.fillStyle = '#f7d117';
    ctx.beginPath();
    ctx.arc(0, 0, r, 0, Math.PI * 2);
    ctx.fill();
    ctx.lineWidth = r * 0.16;
    ctx.strokeStyle = '#0f7a3b';
    ctx.beginPath();
    ctx.arc(0, 0, r * 0.9, 0, Math.PI * 2);
    ctx.stroke();
    // the H, drawn: two posts and the bar
    ctx.fillStyle = '#0f7a3b';
    const w = r * 0.2;
    const h = r * 1.0;
    ctx.fillRect(-r * 0.42, -h / 2, w, h);
    ctx.fillRect(r * 0.42 - w, -h / 2, w, h);
    ctx.fillRect(-r * 0.42, -w / 2, r * 0.84, w);
    edge();
    ctx.beginPath();
    ctx.arc(0, 0, r, 0, Math.PI * 2);
    ctx.stroke();
  }
  ctx.restore();
}

/** One sign as a picture of its own, `size` CSS pixels square (the welcome's and the settings'
 * choices). */
export function StopSign({ style, size = 56 }) {
  const ref = useRef();
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const k = window.devicePixelRatio || 1;
    c.width = Math.round(size * k);
    c.height = Math.round(size * k);
    const g = c.getContext('2d');
    g.setTransform(k, 0, 0, k, 0, 0);
    g.clearRect(0, 0, size, size);
    drawStopSign(g, style, size / 2, size / 2, size * 0.9);
  }, [style, size]);
  return html`<canvas ref=${ref} class="hub-stopsign" style=${`width:${size}px;height:${size}px`}></canvas>`;
}
