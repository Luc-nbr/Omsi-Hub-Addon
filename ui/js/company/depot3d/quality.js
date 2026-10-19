// How finely the 3D depot is drawn, for computers that cannot draw it all: chosen on the depot
// page and kept on this computer (it is about this computer's graphics, not the company), or
// "auto": a guess from the computer to start with, a level lower when the pictures come too
// slowly while the yard is turned.
//
// * high: the sun's shadows at 2048², smoothed edges, the screen's full sharpness (up to 2x),
//   the surfaces' textures, the town round the depot and the staff at work;
// * medium: shadows at 1024², smoothed edges, at most 1.5x, the textures, a smaller town and
//   the staff;
// * low: no shadows, no smoothing, 1x, no trees, no textures, the town's nearest blocks only,
//   no staff, the buses' pictures at half their size.

const KEY = 'omsihub.depot3d.quality';

export const LEVELS = {
  high: { shadow: 2048, msaa: true, dpr: 2, trees: true, atlas: 1024, detail: true, town: 3, staff: true },
  medium: { shadow: 1024, msaa: true, dpr: 1.5, trees: true, atlas: 1024, detail: true, town: 2, staff: true },
  low: { shadow: 0, msaa: false, dpr: 1, trees: false, atlas: 512, detail: false, town: 1, staff: false },
};
const DOWN = { high: 'medium', medium: 'low', low: 'low' };

/** What is chosen: 'auto', 'high', 'medium' or 'low'. */
export function chosenQuality() {
  try {
    const q = localStorage.getItem(KEY);
    return q && (q === 'auto' || LEVELS[q]) ? q : 'auto';
  } catch {
    return 'auto';
  }
}

export function chooseQuality(q) {
  try {
    localStorage.setItem(KEY, q);
  } catch {
    // (a private window: for now only)
  }
  stepped = null;
}

/** The level "auto" stepped down to this session, when it had to. */
let stepped = null;

/** The level to draw at for what is chosen. */
export function levelOf(q) {
  if (q !== 'auto') return q;
  if (stepped) return stepped;
  const cores = navigator.hardwareConcurrency || 4;
  const memory = navigator.deviceMemory || 8;
  return cores <= 2 || memory <= 2 ? 'low' : cores <= 4 || memory <= 4 ? 'medium' : 'high';
}

/** "auto" was too slow at `level`: the next one down (the same when it is already the lowest). */
export function stepDown(level) {
  stepped = DOWN[level];
  return stepped;
}
