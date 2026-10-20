//! The next turn: where the bus is on the trip's route (the lanes the timetable's track drives,
//! as the add-on reads the map) and the first bend ahead sharp enough to be a turn, or the next
//! stop when that comes first. Without a route the next stop is all there is: straight on to
//! it, as the crow flies.
//!
//! Map coordinates as the game has them: x east, y north, metres; headings clockwise from
//! north, so a turn to the right is a positive change of heading.

/// What the driver does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Maneuver {
    Straight,
    SlightLeft,
    Left,
    SharpLeft,
    SlightRight,
    Right,
    SharpRight,
    UTurn,
    Arrive,
}

impl Maneuver {
    pub fn name(self) -> &'static str {
        match self {
            Maneuver::Straight => "straight",
            Maneuver::SlightLeft => "slight_left",
            Maneuver::Left => "left",
            Maneuver::SharpLeft => "sharp_left",
            Maneuver::SlightRight => "slight_right",
            Maneuver::Right => "right",
            Maneuver::SharpRight => "sharp_right",
            Maneuver::UTurn => "uturn",
            Maneuver::Arrive => "arrive",
        }
    }

    /// The turn a change of heading of `deg` degrees is (right positive).
    pub fn of_angle(deg: f64) -> Maneuver {
        let a = deg.abs();
        let (slight, normal, sharp) = if deg < 0.0 { (Maneuver::SlightLeft, Maneuver::Left, Maneuver::SharpLeft) } else { (Maneuver::SlightRight, Maneuver::Right, Maneuver::SharpRight) };
        if a < TURN_MIN {
            Maneuver::Straight
        } else if a < 50.0 {
            slight
        } else if a < 135.0 {
            normal
        } else if a < 165.0 {
            sharp
        } else {
            Maneuver::UTurn
        }
    }
}

/// A bend of less than this (degrees) is the road, not a turn.
pub const TURN_MIN: f64 = 32.0;
/// The heading before and after a point is measured over this far along the route (m): the
/// lanes of a junction are short pieces, a turn is all of them.
const ARM: f64 = 18.0;
/// Turns are looked for this far ahead (m).
const AHEAD: f64 = 3000.0;
/// The bus counts as on the route within this far of it (m).
pub const ON_ROUTE: f64 = 60.0;

/// The next thing to do, how far away (m) and where (the point a street name is asked for:
/// a little way into the road after the turn).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Next {
    pub maneuver: Maneuver,
    pub distance: f64,
    pub probe: Option<[f64; 2]>,
}

/// A route as a polyline, measured along.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Route {
    pub pts: Vec<[f64; 2]>,
    /// The distance along the route to each point.
    pub along: Vec<f64>,
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The heading from `a` to `b` (degrees clockwise from north).
fn bearing(a: [f64; 2], b: [f64; 2]) -> f64 {
    (b[0] - a[0]).atan2(b[1] - a[1]).to_degrees()
}

/// `b - a` between -180 and 180.
fn turn(a: f64, b: f64) -> f64 {
    let d = (b - a).rem_euclid(360.0);
    if d > 180.0 {
        d - 360.0
    } else {
        d
    }
}

impl Route {
    /// The polyline with points closer than 0.5 m to the one before left out.
    pub fn new(points: &[[f64; 2]]) -> Route {
        let mut pts: Vec<[f64; 2]> = Vec::with_capacity(points.len());
        for p in points {
            if p[0].is_finite() && p[1].is_finite() && pts.last().is_none_or(|q| dist(*q, *p) >= 0.5) {
                pts.push(*p);
            }
        }
        let mut along = Vec::with_capacity(pts.len());
        let mut s = 0.0;
        for (i, p) in pts.iter().enumerate() {
            if i > 0 {
                s += dist(pts[i - 1], *p);
            }
            along.push(s);
        }
        Route { pts, along }
    }

    pub fn len(&self) -> f64 {
        self.along.last().copied().unwrap_or(0.0)
    }

    pub fn is_empty(&self) -> bool {
        self.pts.len() < 2
    }

    /// The point `s` metres along the route.
    pub fn at(&self, s: f64) -> [f64; 2] {
        let s = s.clamp(0.0, self.len());
        let i = self.along.partition_point(|a| *a <= s).clamp(1, self.pts.len() - 1);
        let (a, b) = (self.pts[i - 1], self.pts[i]);
        let seg = self.along[i] - self.along[i - 1];
        let t = if seg > 0.0 { (s - self.along[i - 1]) / seg } else { 0.0 };
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
    }

    /// Where `p` is on the route: the distance along it and how far off it, looked for among
    /// the segments from `from` metres on to `to` (the bus does not jump about its route).
    pub fn project(&self, p: [f64; 2], from: f64, to: f64) -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64)> = None;
        for i in 1..self.pts.len() {
            if self.along[i] < from || self.along[i - 1] > to {
                continue;
            }
            let (a, b) = (self.pts[i - 1], self.pts[i]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let l2 = dx * dx + dy * dy;
            let t = if l2 > 0.0 { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
            let q = [a[0] + dx * t, a[1] + dy * t];
            let d = dist(p, q);
            if best.is_none_or(|b| d < b.1) {
                best = Some((self.along[i - 1] + t * l2.sqrt(), d));
            }
        }
        best
    }

    /// How the route bends at `s` metres along it (degrees, right positive).
    fn bend(&self, s: f64) -> f64 {
        let (a, m, b) = (self.at(s - ARM), self.at(s), self.at(s + ARM));
        if dist(a, m) < 1.0 || dist(m, b) < 1.0 {
            return 0.0;
        }
        turn(bearing(a, m), bearing(m, b))
    }

    /// The first turn after `s` metres (within [`AHEAD`]): where it is and its bend.
    pub fn next_turn(&self, s: f64) -> Option<(f64, f64)> {
        let end = (s + AHEAD).min(self.len() - ARM);
        // (each corner of the line is a candidate; a turn made of several short pieces is
        // measured at its sharpest corner within the arm)
        let mut i = self.along.partition_point(|a| *a <= s + 5.0);
        while i < self.pts.len() && self.along[i] <= end {
            let at = self.along[i];
            let b = self.bend(at);
            if b.abs() >= TURN_MIN {
                // (the sharpest corner of this turn)
                let (mut best, mut bb) = (at, b);
                let mut j = i + 1;
                while j < self.pts.len() && self.along[j] <= at + ARM {
                    let bj = self.bend(self.along[j]);
                    if bj.abs() > bb.abs() {
                        best = self.along[j];
                        bb = bj;
                    }
                    j += 1;
                }
                return Some((best, bb));
            }
            i += 1;
        }
        None
    }
}

/// Follows the bus along its route from one look to the next.
#[derive(Debug, Clone, Default)]
pub struct Tracker {
    pub route: Route,
    /// Where the bus was last seen on it (m along).
    pub s: Option<f64>,
}

impl Tracker {
    pub fn new(route: Route) -> Tracker {
        Tracker { route, s: None }
    }

    /// Where the bus at `p` is along the route now, if it is on it.
    pub fn locate(&mut self, p: [f64; 2]) -> Option<f64> {
        if self.route.is_empty() {
            return None;
        }
        // near where it was (a route that loops back past itself is not mistaken), else anywhere
        let near = self.s.and_then(|s| self.route.project(p, s - 100.0, s + 600.0)).filter(|r| r.1 <= ON_ROUTE);
        let found = near.or_else(|| self.route.project(p, f64::MIN, f64::MAX).filter(|r| r.1 <= ON_ROUTE));
        self.s = found.map(|r| r.0);
        found.map(|r| r.0)
    }

    /// What to do next from `p`, with the next stop at `stop` (a map point) when known.
    pub fn next(&mut self, p: [f64; 2], stop: Option<[f64; 2]>) -> Option<Next> {
        let Some(s) = self.locate(p) else { return straight_to(p, stop) };
        let stop_at = stop.and_then(|q| self.route.project(q, s - 20.0, f64::MAX)).filter(|r| r.1 <= ON_ROUTE).map(|r| r.0);
        let turn = self.route.next_turn(s);
        match (turn, stop_at) {
            (Some((t, _)), Some(st)) if st <= t => Some(Next { maneuver: Maneuver::Arrive, distance: (st - s).max(0.0), probe: None }),
            (Some((t, deg)), _) => Some(Next { maneuver: Maneuver::of_angle(deg), distance: t - s, probe: Some(self.route.at(t + 25.0)) }),
            (None, Some(st)) => Some(Next { maneuver: Maneuver::Arrive, distance: (st - s).max(0.0), probe: None }),
            (None, None) => Some(Next { maneuver: Maneuver::Straight, distance: (self.route.len() - s).max(0.0), probe: None }),
        }
    }
}

/// Without a route: straight on to the next stop (or nothing to say without one).
pub fn straight_to(p: [f64; 2], stop: Option<[f64; 2]>) -> Option<Next> {
    let q = stop?;
    let d = dist(p, q);
    Some(Next { maneuver: if d < 40.0 { Maneuver::Arrive } else { Maneuver::Straight }, distance: d, probe: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// North 300 m, then east (a right turn) 200 m, then back west-north-west (a left turn of 150°).
    fn l_route() -> Route {
        let mut pts = Vec::new();
        for i in 0..=30 {
            pts.push([0.0, i as f64 * 10.0]);
        }
        for i in 1..=20 {
            pts.push([i as f64 * 10.0, 300.0]);
        }
        for i in 1..=10 {
            pts.push([200.0 - i as f64 * 10.0 * 0.866, 300.0 + i as f64 * 10.0 * 0.5]);
        }
        Route::new(&pts)
    }

    #[test]
    fn angles_are_turns_by_how_sharp_they_are() {
        assert_eq!(Maneuver::of_angle(10.0), Maneuver::Straight);
        assert_eq!(Maneuver::of_angle(-40.0), Maneuver::SlightLeft);
        assert_eq!(Maneuver::of_angle(90.0), Maneuver::Right);
        assert_eq!(Maneuver::of_angle(-150.0), Maneuver::SharpLeft);
        assert_eq!(Maneuver::of_angle(178.0), Maneuver::UTurn);
        assert_eq!(turn(350.0, 10.0), 20.0);
        assert_eq!(turn(10.0, 350.0), -20.0);
    }

    #[test]
    fn the_route_is_measured_and_a_point_found_on_it() {
        let r = l_route();
        assert!((r.len() - 600.0).abs() < 0.5, "{}", r.len());
        assert_eq!(r.at(150.0), [0.0, 150.0]);
        let (s, off) = r.project([3.0, 100.0], f64::MIN, f64::MAX).unwrap();
        assert!((s - 100.0).abs() < 1e-6 && (off - 3.0).abs() < 1e-6);
    }

    #[test]
    fn the_next_turn_is_said_with_its_distance_and_then_the_one_after() {
        let mut t = Tracker::new(l_route());
        let n = t.next([1.0, 100.0], None).unwrap();
        assert_eq!(n.maneuver, Maneuver::Right);
        assert!((n.distance - 200.0).abs() < 1.0, "{n:?}");
        // the street is asked for in the road after the turn
        let probe = n.probe.unwrap();
        assert!(probe[0] > 20.0 && (probe[1] - 300.0).abs() < 1.0, "{probe:?}");
        // past the corner: the sharp left turn comes next
        let n = t.next([60.0, 299.0], None).unwrap();
        assert_eq!(n.maneuver, Maneuver::SharpLeft);
        assert!((n.distance - 140.0).abs() < 1.0, "{n:?}");
    }

    #[test]
    fn a_stop_before_the_turn_comes_first_and_off_the_route_the_stop_is_straight_on() {
        let mut t = Tracker::new(l_route());
        let n = t.next([0.0, 50.0], Some([4.0, 180.0])).unwrap();
        assert_eq!((n.maneuver, n.distance.round()), (Maneuver::Arrive, 130.0));
        // a stop after the turn: the turn first
        assert_eq!(t.next([0.0, 50.0], Some([100.0, 304.0])).unwrap().maneuver, Maneuver::Right);
        // far off the route: straight to the stop
        let n = t.next([500.0, -400.0], Some([500.0, -100.0])).unwrap();
        assert_eq!((n.maneuver, n.distance.round()), (Maneuver::Straight, 300.0));
        assert_eq!(straight_to([0.0, 0.0], Some([0.0, 30.0])).unwrap().maneuver, Maneuver::Arrive);
        assert_eq!(straight_to([0.0, 0.0], None), None);
    }

    #[test]
    fn a_gentle_bend_is_no_turn() {
        // a quarter circle of 400 m radius: about 1.4° a corner
        let pts: Vec<[f64; 2]> = (0..=60)
            .map(|i| {
                let a = (i as f64 * 1.5).to_radians();
                [400.0 - 400.0 * a.cos(), 400.0 * a.sin()]
            })
            .collect();
        let mut t = Tracker::new(Route::new(&pts));
        assert_eq!(t.next([0.0, 0.0], None).unwrap().maneuver, Maneuver::Straight);
    }
}
