//! The company's link to the plugin in the player's openOMSI (the old `omsihub/addon.rs`, what
//! the company's pages called of it): the duty a drive is to drive, given to the plugin with the
//! company's parts of it; the plugin's side of the control room's messages; the notices for the
//! driver.
//!
//! **Delivered by the plugin port.** The company builds all of it here (the duty sheet, the
//! fare, the contract's note, the crew's strip: `launch::plugin_parts`, `launch::crew_of`), and
//! these functions are where it goes to the plugin (`hub_game::bridge`'s assignment, inbox and
//! notices, through `addon`). Until the plugin port fills them in, nothing reaches the game:
//! the drive starts with the duty's command line alone and the plugin's own defaults.

use hub_game::bridge;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The company's parts of a duty's assignment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parts {
    pub sheet: Vec<bridge::SheetRow>,
    pub controls: Vec<bridge::ControlNote>,
    pub fare: Option<bridge::Fare>,
    pub contract: Option<bridge::ContractNote>,
}

/// The duty the plugin was last given (the old `addon.json`'s `assigned`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Assigned {
    pub issued: u64,
    pub driver: String,
    pub map: String,
    pub bus: String,
    /// A company's duty: the company's name, the duty's line (the timetable's name) and tour.
    /// Empty: a drive of the player's own.
    pub company: String,
    pub line: String,
    pub tour: String,
    /// The company whose duty it is, by its id ("": a drive of the player's own).
    pub company_id: String,
}

/// Give the plugin the duty of a drive about to start: `company` names the company's duty (its
/// name and the duty's id) and gives its parts, `crew` the driver's strip and the bus's notes.
/// **The plugin port** writes it (`hub_game::bridge::write_assignment`); until then it is only
/// said in the log.
pub fn assign_duty(d: &hub_core::Duty, company: Option<(&str, &str, Parts)>, crew: Option<bridge::Crew>, company_id: Option<&str>) {
    let what = company.as_ref().map(|(name, duty, _)| format!("{name} {duty}")).unwrap_or_else(|| "a drive of the player's own".into());
    hub_core::log_line(&format!("plugin: the duty for {} ({what}, crew {}, company {}) waits for the plugin port", d.bus, crew.is_some(), company_id.unwrap_or("-")));
}

/// The plugin's folder and the company's duty it was given, while it has one (the messages'
/// link): none until the plugin port.
pub fn duty_link() -> Option<(PathBuf, Assigned)> {
    None
}

/// Cards for the driver in the game Omsi-Hub started: **the plugin port** shows them
/// (`bridge::write_notices`).
pub fn notify_game(_notices: Vec<bridge::Notice>) {}
