//! A drive as the app plans it: the map, the bus, the day and time, the line and tour - what
//! the page's drive steps and the company's planning hand over. `hub-api` turns it into the
//! game's command line (`hub_game::launch::LaunchSpec`, official flags only) and the plugin's
//! assignment (what the command line cannot say: a company's duty, legs, a free line, the
//! display font).
//!
//! Its JSON is what the page sends (the old launcher's `Duty`, the same field names), so the
//! ported pages send it unchanged.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Duty {
    /// The map's `global.cfg`, relative to the OMSI 2 folder.
    pub map: String,
    /// The bus's `.bus` file, relative to the OMSI 2 folder.
    pub bus: String,
    /// The livery (`.cti` item name or index).
    pub paint: Option<String>,
    /// The bus options chosen (script variable, value), set after the livery.
    pub set_vars: Vec<(String, f32)>,
    /// The display font for the bus's destination displays (the plugin puts it on).
    pub display_font: Option<String>,
    /// The number plate typed (wins over the bus's own list).
    pub plate: Option<String>,
    /// The fleet number picked from the bus's `[number]` list.
    pub number: Option<String>,
    /// The depot (`.hof`) file's name.
    pub hof: Option<String>,
    /// The entry point the bus is put at.
    pub entry: Option<i32>,
    pub line: Option<String>,
    pub tour: Option<String>,
    /// The trip of the tour to start with: its departure (`HH:MM`) or its place (1 = the first).
    pub trip: Option<String>,
    /// Drive on with the rest of the tour after that trip.
    pub whole_tour: bool,
    /// A duty of several tours: its parts after the first (the plugin chains them,
    /// `duty.start` on each trip's end). `line`, `tour` and `trip` are its first part's.
    pub legs: Vec<String>,
    /// A free drive along one route of `line`: the route's trip file name (the plugin follows
    /// it; nobody keeps the time).
    pub free_line: Option<String>,
    /// `HH:MM`.
    pub time: String,
    /// `YYYY-MM-DD`.
    pub date: Option<String>,
    pub weather: Option<String>,
    pub traffic: Option<u32>,
    pub passengers: Option<bool>,
    pub schedule: Option<bool>,
    pub autostart: Option<bool>,
    /// Start as a pedestrian beside the bus.
    pub on_foot: Option<bool>,
    /// The driver (the career's name).
    pub profile: Option<String>,
    /// LAN play: `host`, or `join:<ip[:port] | empty = search>`.
    pub lan: Option<String>,
    /// The name the other LAN players see.
    pub lan_name: Option<String>,
    /// Season: `spring`/`summer`/`autumn`/`winter` with `-early`/`-late`; none: by date.
    pub season: Option<String>,
    /// One of OMSI's tutorials (1..4): nothing else of the duty counts.
    pub tutorial: Option<usize>,
    /// A situation (`.osn`) to continue: nothing else of the duty counts.
    pub situation: Option<String>,
    /// Start with the game's object editor (the plugin opens it).
    pub editor: bool,
    /// A duty of the player's bus company (its id); none: a drive of the player's own.
    pub company: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_s_duty_reads() {
        let d: Duty = serde_json::from_str(r#"{"map":"maps/Grundorf/global.cfg","bus":"Vehicles/X/a.bus","time":"08:15","line":"10","tour":"2","legs":["11/3"],"set_vars":[["door",1.0]]}"#).unwrap();
        assert_eq!((d.line.as_deref(), d.tour.as_deref(), d.time.as_str()), (Some("10"), Some("2"), "08:15"));
        assert_eq!(d.legs, ["11/3"]);
        assert_eq!(d.set_vars, [("door".to_string(), 1.0)]);
        assert!(!d.editor && d.company.is_none());
    }
}
