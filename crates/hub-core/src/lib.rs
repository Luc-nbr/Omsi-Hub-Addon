//! Omsi-Hub's domain, apart from the app and from the game: the bus company and RosterFlow
//! (`company`, ported from the old `omsi-launcher-core/src/company`), and what they stand on.
//!
//! * [`timetable`] - our own timetable types (`LineInfo`, `TourInfo`, `TripInfo`, `StopInfo`):
//!   a map's lines on a date, as the company plans them. Built from `hub-omsi`'s reader
//!   ([`timetable::list_lines`]);
//! * [`duty`] - a drive as the app plans it ([`Duty`]), turned into the game's command line by
//!   `hub-game`;
//! * [`paths`] - Omsi-Hub's data folder ([`data_dir`]) and the player's folders (openOMSI,
//!   OMSI 2), the content resolved through them ([`resolve_content`]);
//! * [`settings`] - what the first-run setup and the settings page keep;
//! * [`log`] - one line into Omsi-Hub's log ([`log_line`]);
//! * [`migrate`] - the saves of the old Omsi-Hub (`~/.openomsi`) found and, later, imported;
//! * [`company`] - the bus company, RosterFlow, careers: the target of the port.

pub mod company;
pub mod duty;
pub mod log;
pub mod migrate;
pub mod paths;
pub mod settings;
pub mod timetable;

pub use duty::Duty;
pub use log::log_line;
pub use paths::{content_dir, data_dir, resolve_content, root, vfs};
pub use timetable::{LineInfo, StopInfo, TourInfo, TripInfo, DEFAULT_DATE};

/// Names in older vehicle packs use underscores as spaces: "MAN_SD_202" is "MAN SD 202".
pub fn display_bus_name(name: &str) -> String {
    name.replace('_', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A bus type's label: its own name, else its file's name.
pub fn vehicle_type_label(type_name: &str, path: &std::path::Path) -> String {
    if type_name.trim().is_empty() {
        display_bus_name(&path.file_stem().unwrap_or_default().to_string_lossy())
    } else {
        display_bus_name(type_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_names_read_as_words() {
        assert_eq!(display_bus_name(" MAN_SD__202 "), "MAN SD 202");
        assert_eq!(vehicle_type_label("", std::path::Path::new("Vehicles/X/Citaro_O530.bus")), "Citaro O530");
        assert_eq!(vehicle_type_label("Citaro", std::path::Path::new("a.bus")), "Citaro");
    }
}
