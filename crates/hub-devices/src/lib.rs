//! The devices of a bus - its IBIS, its ticket printer or ALMEX, any screen with keys or touch
//! fields - read from the bus's own files, and their screens laid flat for the phone and tablet
//! companion to draw as the game draws them.
//!
//! A faithful port of the device pipeline of Luc's career add-on for OMSI 2
//! (<https://github.com/Luc-nbr/Omsi-career-addon>: `core/busmodule.ts`, `busscherm.ts`,
//! `schermcfg.ts`, `schermvorm.ts`, `schermtextuur.ts`, `textuur.ts`, `kleurstelling.ts`,
//! `oft.ts` and `o3d.ts`), where every rule was measured against the game and a full
//! installation. Nothing here knows the game or the app: it reads files.

pub mod cfg;
pub mod ctc;
pub mod fonts;
pub mod form;
pub mod layout;
pub mod modules;
pub mod o3d;
pub mod protocol;
pub mod scripts;
pub mod testbus;
pub mod text;
pub mod textures;
