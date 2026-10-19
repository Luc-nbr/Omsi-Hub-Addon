//! The rooms inside the depot's buildings, laid out by the player as in City Bus Manager: the
//! offices' floors (offices, a meeting room, rest rooms, lockers, toilets, a reception, an
//! archive) and the back of each workshop bay (a tool store, a parts store, the mechanics' room,
//! the foreman's office). A room is a rectangle of whole metres on a building's floor, in the
//! building's own frame as it is turned to the street (x across from its west side, y from its
//! back), within the part of it that can be laid out (`inside`), not over another room; it is
//! furnished at once, paid now (its price a square metre, `RoomKind::cost_m2`), and taken out
//! for nothing back. A building pulled down takes its rooms with it, an office floor given up
//! the rooms on it (`prune`); a building moved keeps them.
//!
//! Figures (Realistic, net, mid-2020s Germany, rounded): fitting out an office about €400 a
//! square metre with its furniture, a staff room or a locker room €300-400, sanitary rooms
//! €700-800 (their pipes); a store's racks €250-300, a tool store with its benches €450.

use super::depot::{self, Area, Site};
use super::model::{Cents, Company};
use super::yard::{self, Rect};
use serde::{Deserialize, Serialize};

/// What a room is.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RoomKind {
    /// Desks for the planners and the office's people.
    Office,
    Meeting,
    /// Sofas and a kitchenette, for the drivers between their duties.
    Rest,
    Lockers,
    Toilets,
    Reception,
    Archive,
    /// Benches and tool cabinets.
    Tools,
    /// Racks of spare parts.
    Parts,
    /// The mechanics' break room.
    Mechanics,
    /// The workshop foreman's office.
    Foreman,
}

impl RoomKind {
    pub const ALL: [RoomKind; 11] = [
        RoomKind::Office,
        RoomKind::Meeting,
        RoomKind::Rest,
        RoomKind::Lockers,
        RoomKind::Toilets,
        RoomKind::Reception,
        RoomKind::Archive,
        RoomKind::Tools,
        RoomKind::Parts,
        RoomKind::Mechanics,
        RoomKind::Foreman,
    ];

    /// The building it is laid out in.
    pub fn area(self) -> Area {
        use RoomKind::*;
        match self {
            Office | Meeting | Rest | Lockers | Toilets | Reception | Archive => Area::Offices,
            Tools | Parts | Mechanics | Foreman => Area::Workshop,
        }
    }

    pub fn key(self) -> &'static str {
        use RoomKind::*;
        match self {
            Office => "office",
            Meeting => "meeting",
            Rest => "rest",
            Lockers => "lockers",
            Toilets => "toilets",
            Reception => "reception",
            Archive => "archive",
            Tools => "tools",
            Parts => "parts",
            Mechanics => "mechanics",
            Foreman => "foreman",
        }
    }

    pub fn from_key(s: &str) -> Option<RoomKind> {
        RoomKind::ALL.into_iter().find(|k| k.key() == s)
    }

    pub fn label(self) -> &'static str {
        use RoomKind::*;
        match self {
            Office => "Office",
            Meeting => "Meeting room",
            Rest => "Rest room",
            Lockers => "Locker room",
            Toilets => "Toilets",
            Reception => "Reception",
            Archive => "Archive",
            Tools => "Tool store",
            Parts => "Parts store",
            Mechanics => "Mechanics' room",
            Foreman => "Foreman's office",
        }
    }

    /// What is in it.
    pub fn what(self) -> &'static str {
        use RoomKind::*;
        match self {
            Office => "Desks for the planners and the office's people",
            Meeting => "A table for the meetings, a screen on the wall",
            Rest => "Sofas and a kitchenette, for the drivers between their duties",
            Lockers => "Lockers and benches, where the staff change",
            Toilets => "Toilets and washbasins",
            Reception => "A counter where visitors are met",
            Archive => "Shelves for the files",
            Tools => "Benches and tool cabinets",
            Parts => "Racks of spare parts",
            Mechanics => "A table and a coffee machine for the mechanics",
            Foreman => "A desk with a window on the bays",
        }
    }

    /// The launcher's icon for it.
    pub fn icon(self) -> &'static str {
        use RoomKind::*;
        match self {
            Office => "desk",
            Meeting => "groups",
            Rest => "weekend",
            Lockers => "checkroom",
            Toilets => "wc",
            Reception => "badge",
            Archive => "inventory_2",
            Tools => "handyman",
            Parts => "category",
            Mechanics => "coffee",
            Foreman => "engineering",
        }
    }

    /// What a square metre of it costs to fit out (Realistic, at founding).
    pub fn cost_m2(self) -> Cents {
        use RoomKind::*;
        let euros = match self {
            Office => 420,
            Meeting => 480,
            Rest => 380,
            Lockers => 320,
            Toilets => 760,
            Reception => 450,
            Archive => 260,
            Tools => 480,
            Parts => 300,
            Mechanics => 380,
            Foreman => 420,
        };
        euros * 100
    }
}

/// A room laid out: on which building (`yard::Placed::id`) and floor (0 the ground floor), its
/// rectangle of metres in the building's own frame.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Room {
    pub id: u32,
    pub module: u32,
    #[serde(default)]
    pub floor: u32,
    pub kind: RoomKind,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Room {
    pub fn rect(&self) -> Rect {
        Rect { x: self.x, y: self.y, w: self.w, h: self.h }
    }
}

/// The smallest a room is across and deep, in metres.
pub const MIN_SIDE: i32 = 2;

/// The part of a building that can be laid out, in metres of its own frame (None: one that has
/// no rooms): the offices' whole building (its first 4 m are the yard at its back), the back of
/// a workshop bay behind the bus in it.
pub fn inside(area: Area) -> Option<Rect> {
    match area {
        Area::Offices => Some(Rect { x: 0, y: 4, w: 20, h: 20 }),
        Area::Workshop => Some(Rect { x: 1, y: 1, w: 6, h: 5 }),
        _ => None,
    }
}

/// How many floors a building of `area` has to lay out: the offices a floor a level (three at
/// most), a workshop bay its one.
pub fn floors(s: &Site, area: Area) -> u32 {
    match area {
        Area::Offices => s.offices.clamp(1, 3),
        Area::Workshop => 1,
        _ => 0,
    }
}

/// What a square metre of `kind` costs the company now, in whole euros.
pub fn cost_m2(c: &Company, kind: RoomKind) -> Cents {
    ((kind.cost_m2() as f64 * super::econ_uk::mode(c.game_mode).costs * c.price_index / 100.0 / 100.0).round() as Cents) * 100
}

/// What fitting out `r` as `kind` costs the company now.
pub fn cost(c: &Company, kind: RoomKind, r: &Rect) -> Cents {
    cost_m2(c, kind) * (r.w.max(0) * r.h.max(0)) as Cents
}

/// Whether a room of `kind` fits at `r` on `floor` of building `module` (`except`: a room
/// being laid out again, not in its own way); why not if it does not.
pub fn fits(s: &Site, module: u32, floor: u32, kind: RoomKind, r: &Rect, except: Option<u32>) -> Result<(), &'static str> {
    let Some(p) = s.layout.iter().find(|p| p.id == module) else { return Err("That is not there any more.") };
    if p.area != kind.area() {
        return Err("This room does not go in this building.");
    }
    if s.works.iter().any(|w| w.at.is_some_and(|at| at.id == module)) {
        return Err("Building work is under way on it.");
    }
    if floor >= floors(s, p.area) {
        return Err("The building has no such floor.");
    }
    let Some(room) = inside(p.area) else { return Err("This room does not go in this building.") };
    if r.w < MIN_SIDE || r.h < MIN_SIDE {
        return Err("A room is at least 2 m by 2 m.");
    }
    if !r.inside(&room) {
        return Err("It does not fit in the building.");
    }
    if s.rooms.iter().any(|o| o.module == module && o.floor == floor && Some(o.id) != except && o.rect().overlaps(r)) {
        return Err("Another room is there already.");
    }
    Ok(())
}

/// Lay out a room: fitted out at once, paid now (from the building credit first).
pub fn add(c: &mut Company, module: u32, floor: u32, kind: RoomKind, r: Rect) -> Result<u32, &'static str> {
    yard::reconcile(&mut c.site);
    fits(&c.site, module, floor, kind, &r, None)?;
    let price = cost(c, kind, &r);
    depot::pay_building(c, price, format!("{} ({} m²)", kind.label(), r.w * r.h))?;
    c.site.room_counter += 1;
    let id = c.site.room_counter;
    c.site.rooms.push(Room { id, module, floor, kind, x: r.x, y: r.y, w: r.w, h: r.h });
    Ok(id)
}

/// Take a room out: nothing paid back.
pub fn remove(c: &mut Company, id: u32) -> Result<(), &'static str> {
    let Some(k) = c.site.rooms.iter().position(|o| o.id == id) else { return Err("That is not there any more.") };
    c.site.rooms.remove(k);
    Ok(())
}

/// The rooms of buildings no longer there, or of floors given up, gone (`yard::reconcile`).
pub fn prune(s: &mut Site) {
    let floors_of = |s: &Site, module: u32| s.layout.iter().find(|p| p.id == module).map(|p| floors(s, p.area)).unwrap_or(0);
    let keep: Vec<bool> = s.rooms.iter().map(|o| o.floor < floors_of(s, o.module)).collect();
    let mut k = keep.into_iter();
    s.rooms.retain(|_| k.next().unwrap_or(false));
}

#[cfg(test)]
mod tests {
    use super::super::model::GameMode;
    use super::super::{found, Founding};
    use super::*;

    fn company() -> Company {
        let mut c = found(&Founding { name: "Rooms".into(), game_mode: GameMode::Normal, date: "2024-03-04".into(), ..Default::default() }, "Luc");
        c.cash += 10_000_000_00;
        yard::reconcile(&mut c.site);
        c
    }

    fn module(c: &Company, area: Area) -> u32 {
        c.site.layout.iter().find(|p| p.area == area).unwrap().id
    }

    #[test]
    fn a_room_is_laid_out_inside_its_building_and_paid() {
        let mut c = company();
        let offices = module(&c, Area::Offices);
        let r = Rect { x: 0, y: 4, w: 6, h: 5 };
        let cash = c.cash;
        let price = cost(&c, RoomKind::Office, &r);
        assert!(price > 0);
        let id = add(&mut c, offices, 0, RoomKind::Office, r).unwrap();
        assert_eq!(c.cash, cash - price);
        assert_eq!(c.site.rooms.len(), 1);
        // not over it, not outside, not too small, not of the workshop, not on a floor it has not
        assert_eq!(add(&mut c, offices, 0, RoomKind::Rest, Rect { x: 4, y: 6, w: 4, h: 4 }), Err("Another room is there already."));
        assert_eq!(add(&mut c, offices, 0, RoomKind::Rest, Rect { x: 18, y: 4, w: 4, h: 4 }), Err("It does not fit in the building."));
        assert_eq!(add(&mut c, offices, 0, RoomKind::Rest, Rect { x: 10, y: 10, w: 1, h: 4 }), Err("A room is at least 2 m by 2 m."));
        assert_eq!(add(&mut c, offices, 0, RoomKind::Tools, Rect { x: 10, y: 10, w: 3, h: 3 }), Err("This room does not go in this building."));
        assert_eq!(add(&mut c, offices, 5, RoomKind::Rest, Rect { x: 10, y: 10, w: 3, h: 3 }), Err("The building has no such floor."));
        // beside it, it fits; taken out, its place is free again
        add(&mut c, offices, 0, RoomKind::Rest, Rect { x: 6, y: 4, w: 4, h: 4 }).unwrap();
        remove(&mut c, id).unwrap();
        add(&mut c, offices, 0, RoomKind::Toilets, Rect { x: 0, y: 4, w: 3, h: 3 }).unwrap();
        let bay = module(&c, Area::Workshop);
        add(&mut c, bay, 0, RoomKind::Parts, Rect { x: 1, y: 1, w: 3, h: 5 }).unwrap();
    }

    #[test]
    fn a_building_pulled_down_takes_its_rooms_with_it() {
        let mut c = company();
        let offices = module(&c, Area::Offices);
        add(&mut c, offices, 0, RoomKind::Office, Rect { x: 0, y: 4, w: 6, h: 5 }).unwrap();
        let bay = module(&c, Area::Workshop);
        add(&mut c, bay, 0, RoomKind::Tools, Rect { x: 1, y: 1, w: 3, h: 3 }).unwrap();
        yard::demolish(&mut c, offices).unwrap();
        assert_eq!(c.site.rooms.iter().map(|o| o.kind).collect::<Vec<_>>(), vec![RoomKind::Tools]);
        // a building moved keeps them
        let p = *c.site.layout.iter().find(|p| p.id == bay).unwrap();
        let spot = (0..40).flat_map(|y| (0..40).map(move |x| (x, y))).find(|(x, y)| (*x, *y) != (p.x, p.y) && yard::fits(&c.site, Area::Workshop, *x, *y, 0, Some(bay)).is_ok()).unwrap();
        yard::move_module(&mut c, bay, spot.0, spot.1, 0).unwrap();
        assert_eq!(c.site.rooms.len(), 1);
    }

    #[test]
    fn an_office_floor_given_up_takes_its_rooms() {
        let mut s = Site { offices: 2, ..Site::default() };
        yard::reconcile(&mut s);
        let offices = s.layout.iter().find(|p| p.area == Area::Offices).unwrap().id;
        s.rooms.push(Room { id: 1, module: offices, floor: 1, kind: RoomKind::Archive, x: 0, y: 4, w: 3, h: 3 });
        s.rooms.push(Room { id: 2, module: offices, floor: 0, kind: RoomKind::Archive, x: 0, y: 4, w: 3, h: 3 });
        prune(&mut s);
        assert_eq!(s.rooms.len(), 2);
        s.offices = 1;
        prune(&mut s);
        assert_eq!(s.rooms.iter().map(|o| o.id).collect::<Vec<_>>(), vec![2]);
    }
}
