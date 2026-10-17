//! A bus: the header of its `.bus` file ([`Bus`]: names, model, cabin, coupling, numbers) and
//! its `passengercabin.cfg` ([`PassengerCabin`]). Not its scripts or model: Omsi-Hub needs what
//! the dealer, the fleet and the drive steps show.
//!
//! **Status: the contract.** The types are fixed; the readers are `todo!()` until the hub-omsi
//! port.
#![allow(unused_variables)]

use crate::text::CfgFile;
use crate::ReadError;
use std::path::{Path, PathBuf};

/// A bus's `.bus` file, its header.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bus {
    pub path: PathBuf,
    /// `[friendlyname]`: the manufacturer, the type's name, the default livery.
    pub manufacturer: String,
    pub type_name: String,
    pub default_paint: String,
    /// `[description]`.
    pub description: String,
    /// The model's `model.cfg`, relative to the bus's folder.
    pub model: Option<String>,
    /// The sound's and the passenger cabin's files, relative to the bus's folder.
    pub sound: Option<String>,
    pub passenger_cabin: Option<String>,
    /// Only for the AI (`[aiveh]`).
    pub ai_only: bool,
    /// A section coupled behind (its `.bus`, and whether it is turned round) and before.
    pub couple_back: Option<(String, bool)>,
    pub couple_front: Option<(String, bool)>,
    /// The bounding box: size x, y, z, then centre x, y, z (m).
    pub bounding_box: Option<[f32; 6]>,
    /// The fleet numbers of its `[number]` list with the plate each gives (empty: none).
    pub numbers: Vec<(String, String)>,
    /// Its registration list (`[registration_*]`), when it has one.
    pub registration_list: Option<String>,
    /// It runs on rails (a tram).
    pub rail: bool,
}

impl Bus {
    pub fn parse(cfg: &CfgFile) -> Bus {
        todo!("hub-omsi port: .bus header")
    }

    pub fn load(path: &Path) -> Result<Bus, ReadError> {
        let mut b = Bus::parse(&CfgFile::read(path)?);
        b.path = path.to_path_buf();
        Ok(b)
    }

    /// The bus's folder (its files' paths are relative to it).
    pub fn dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new(""))
    }

    /// It is a rear section of an articulated or coupled bus (another section couples to its
    /// front).
    pub fn is_rear_section(&self) -> bool {
        self.couple_front.is_some()
    }
}

/// A place for a passenger.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PassengerPlace {
    pub pos: [f32; 3],
    /// The seat's height (0: a standing place).
    pub height: f32,
    pub rotation: f32,
}

/// A bus's `passengercabin.cfg`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PassengerCabin {
    pub places: Vec<PassengerPlace>,
    /// The doors passengers get in and out by.
    pub entries: usize,
    pub exits: usize,
}

impl PassengerCabin {
    pub fn parse(cfg: &CfgFile) -> PassengerCabin {
        todo!("hub-omsi port: passengercabin.cfg")
    }

    pub fn load(path: &Path) -> Result<PassengerCabin, ReadError> {
        Ok(PassengerCabin::parse(&CfgFile::read(path)?))
    }

    pub fn seats(&self) -> usize {
        self.places.iter().filter(|p| p.height > 0.0).count()
    }

    pub fn standing(&self) -> usize {
        self.places.len() - self.seats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cabin_counts_seats_and_standing_places() {
        let seat = PassengerPlace { height: 0.45, ..PassengerPlace::default() };
        let c = PassengerCabin { places: vec![seat.clone(), seat, PassengerPlace::default()], ..PassengerCabin::default() };
        assert_eq!((c.seats(), c.standing()), (2, 1));
        let b = Bus { path: PathBuf::from("Vehicles/X/a.bus"), ..Bus::default() };
        assert_eq!(b.dir(), Path::new("Vehicles/X"));
    }
}
