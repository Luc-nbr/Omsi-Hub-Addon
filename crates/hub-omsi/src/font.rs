//! Display fonts (`.oft`): the names of the fonts a file holds (`[newfont]`), for the bus's
//! display font choice. Their bitmaps are not read.
//!
//! **Status: the contract.** The reader is `todo!()` until the hub-omsi port.
#![allow(unused_variables)]

use crate::text::CfgFile;
use crate::ReadError;
use std::path::Path;

/// The font names in a `.oft` file, in its order.
pub fn font_names(cfg: &CfgFile) -> Vec<String> {
    todo!("hub-omsi port: .oft names")
}

pub fn load_names(path: &Path) -> Result<Vec<String>, ReadError> {
    Ok(font_names(&CfgFile::read(path)?))
}
