// The staff at work today about the 3D yard (`plan.staff`): each walks a way of the scene
// (`scene.js`'s routes) for their role - the mechanics beside the buses in the bays, the
// cleaners in the washing bay and between the parked buses, the drivers between the buses, the
// diesel station and the offices, the service staff before the offices' door, the inspectors on
// the pavement by the gate - stopping now and then (their idle), walking on (their walk) at the
// pace their legs go. Two on one way keep a step apart. Without the animations everybody stands
// where they would start.
//
// Not all of them: the drivers are mostly out on their duties (a quarter shows), and the yard
// shows at most as many as its graphics level allows (`quality.js`), every role in turn.

/** Floats an instance: x, z, heading, how far between the two frames; the frame's row in the
 * bones, the next one's, walking (the walk's skin) or not, and one spare. */
export const FIGURE = 8;

/** Where each role works, the ways it likes best first; the pavement where none of them is
 * built. */
const WHERE = {
  driver: ['parking', 'fuel', 'offices'],
  mechanic: ['workshop'],
  cleaner: ['wash', 'parking'],
  service: ['offices'],
  researcher: ['offices', 'workshop'],
  inspector: ['street'],
};
export const ROLES = Object.keys(WHERE);

const TURN = 4.5;
const WALK = 0.65;

/** A small seeded random, so a person starts where they started. */
function seeded(seed) {
  let s = seed >>> 0 || 1;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 4294967296;
  };
}

const wrap = (a) => Math.atan2(Math.sin(a), Math.cos(a));

/** Who of `staff` shows: a quarter of the drivers, then every role in turn up to `most`. */
function chosen(staff, most) {
  const by = new Map(ROLES.map((r) => [r, []]));
  for (const e of [...staff].sort((a, b) => a.id - b.id)) by.get(e.role)?.push(e);
  const drivers = by.get('driver');
  by.set('driver', drivers.slice(0, Math.ceil(drivers.length / 4)));
  const out = [];
  for (let k = 0; out.length < most; k++) {
    const row = ROLES.map((r) => by.get(r)[k]).filter(Boolean);
    if (!row.length) break;
    out.push(...row.slice(0, most - out.length));
  }
  return out;
}

export class Crowd {
  /** `staff` as the plan has it ({ id, role }), `routes` the scene's, at most `most` shown. */
  constructor(staff, routes, most) {
    const used = routes.map(() => 0);
    this.people = chosen(staff, most).map((e) => {
      const rnd = seeded(e.id * 7919 + 17);
      // the least walked of the ways the role likes, the one it likes best on a tie
      const kinds = WHERE[e.role];
      let best = -1;
      routes.forEach((r, k) => {
        const pref = kinds.indexOf(r.kind);
        if (pref < 0) return;
        if (best < 0 || used[k] < used[best] || (used[k] === used[best] && pref < kinds.indexOf(routes[best].kind))) best = k;
      });
      if (best < 0) best = routes.findIndex((r) => r.kind === 'street');
      const route = routes[best];
      // (a step apart from who else walks it)
      const side = used[best] % 2 ? 1 : -1;
      const off = [side * (0.35 + 0.3 * Math.floor(used[best] / 2)) * (rnd() < 0.5 ? 1 : -1), (rnd() - 0.5) * 0.6];
      used[best]++;
      const at = Math.floor(rnd() * route.pts.length);
      const p = route.pts[at];
      return {
        role: e.role,
        route,
        off,
        rnd,
        at,
        dir: rnd() < 0.5 ? 1 : -1,
        x: p.x + off[0],
        z: p.z + off[1],
        heading: p.face ?? rnd() * Math.PI * 2,
        walking: false,
        wait: rnd() * 6,
        t: rnd() * 10,
      };
    });
  }

  get count() {
    return this.people.length;
  }

  /** The roles that show. */
  roles() {
    return [...new Set(this.people.map((f) => f.role))];
  }

  /** `dt` seconds on: who stands waits, then walks to the next point of their way. */
  step(dt, speedOf = () => WALK) {
    for (const f of this.people) {
      f.t += dt;
      const pts = f.route.pts;
      if (f.wait > 0) {
        f.wait -= dt;
        const p = pts[f.at];
        if (p.face !== null) f.heading += Math.max(-TURN * dt, Math.min(TURN * dt, wrap(p.face - f.heading)));
        if (f.wait > 0 || pts.length < 2) continue;
        // on to the next point, back the other way at an end
        if (f.at + f.dir < 0 || f.at + f.dir >= pts.length) f.dir = -f.dir;
        f.at += f.dir;
      }
      const p = pts[f.at];
      const dx = p.x + f.off[0] - f.x;
      const dz = p.z + f.off[1] - f.z;
      const dist = Math.hypot(dx, dz);
      if (!f.walking) {
        f.walking = true;
        f.t = f.rnd() * 2;
      }
      const turn = wrap(Math.atan2(dx, dz) - f.heading);
      f.heading += Math.max(-TURN * dt, Math.min(TURN * dt, turn));
      // (it turns on the spot before it walks off)
      if (Math.abs(turn) > 0.9) continue;
      const go = Math.min(dist, speedOf(f.role) * dt);
      f.x += (dx / (dist || 1)) * go;
      f.z += (dz / (dist || 1)) * go;
      if (dist - go > 0.02) continue;
      if (p.stop) {
        f.walking = false;
        f.wait = 2.5 + f.rnd() * 7;
        f.t = f.rnd() * 4;
      } else {
        f.at += f.dir;
        if (f.at < 0 || f.at >= pts.length) {
          f.dir = -f.dir;
          f.at += 2 * f.dir;
        }
      }
    }
  }

  /** The instances by role (`FIGURE` floats each), for the roles whose figure `modelOf(role)`
   * has (its `clips` and `fps`). */
  instances(modelOf) {
    const groups = new Map();
    for (const f of this.people) {
      const m = modelOf(f.role);
      if (!m) continue;
      const clip = m.clips[f.walking ? 'walk' : 'idle'];
      const at = (f.t * m.fps) % clip.frames;
      const k = Math.floor(at);
      if (!groups.has(f.role)) groups.set(f.role, []);
      groups.get(f.role).push(f.x, f.z, f.heading, at - k, clip.row + k, clip.row + ((k + 1) % clip.frames), f.walking ? 1 : 0, 0);
    }
    return new Map([...groups].map(([k, v]) => [k, new Float32Array(v)]));
  }
}
