//! A weather file (`.owt`): what the drive steps show of it.
//!
//! **Status: the contract.** The reader is `todo!()` until the hub-omsi port.
#![allow(unused_variables)]

use crate::text::CfgFile;
use crate::ReadError;
use std::path::{Path, PathBuf};

/// What falls.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Precipitation {
    #[default]
    None,
    Rain,
    Snow,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Weather {
    pub path: PathBuf,
    /// `[name]`, and `[description]`.
    pub name: String,
    pub description: String,
    /// The visibility, m.
    pub fog_m: f32,
    /// °C.
    pub temperature: f32,
    /// The clouds' texture (empty: clear).
    pub clouds: String,
    pub precipitation: Precipitation,
    /// Its intensity, 0..1.
    pub intensity: f32,
    /// Snow on the ground.
    pub snow: bool,
    /// How wet the roads are, 0..1.
    pub ground_wet: f32,
}

impl Weather {
    pub fn parse(cfg: &CfgFile) -> Weather {
        todo!("hub-omsi port: .owt")
    }

    pub fn load(path: &Path) -> Result<Weather, ReadError> {
        let mut w = Weather::parse(&CfgFile::read(path)?);
        w.path = path.to_path_buf();
        Ok(w)
    }
}
