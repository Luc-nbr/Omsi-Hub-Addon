//! The bus company, RosterFlow and the careers: **the target of the company port.**
//!
//! Empty for now. The old `omsi-launcher-core/src/company/*` (about 47k lines) moves here as
//! it is; its dependencies on the old launcher become:
//!
//! | old | new |
//! |---|---|
//! | `crate::LineInfo` / `TourInfo` / `TripInfo` / `StopInfo` | `crate::timetable::*` |
//! | `crate::Duty` | `crate::Duty` |
//! | `crate::list_lines` / `lines_on` | `crate::timetable::list_lines` |
//! | `crate::data_dir` | `crate::data_dir` (Omsi-Hub's own folder, not `~/.openomsi`) |
//! | `crate::resolve_content`, `content_dir`, `root` | `crate::paths::*` (through `hub_omsi::vfs`) |
//! | `crate::log_line` / `log_to_file` | `crate::log_line` |
//! | `crate::display_bus_name`, `vehicle_type_label` | the same, in `crate` |
//! | `omsi_vehicle::Hof`, `omsi_timetable::*`, `omsi_map::*` | `hub_omsi::{hof, timetable, map}` |
//!
//! The modules that go here, in the old file names: `plan`, `studio`, `staff`, `clock`, `day`,
//! `concessions`, `contracts`, `messages`, `crew`, `cloud`, `store`, `career`, `rosterflow*`,
//! `plandays*`, `econ_uk*`, `together/*`, and beside them `compose`, `busoptions`, `qr*`,
//! `lines`, `linehof`, `owndepot`, `depot`, `service`, `ttstore`.
