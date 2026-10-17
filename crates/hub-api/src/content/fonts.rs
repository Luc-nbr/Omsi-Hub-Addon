//! The bus's display font on the bus step, and a destination on the bus's own display.
//!
//! The previews of these need the bus's displays read from its model (its text textures, its
//! matrix scripts) and the fonts drawn into them as the game draws them. Omsi-Hub has neither
//! reader nor renderer yet, so the previews are not offered: the font section and the depot
//! editor's preview say the displays could not be read (`{ failed: true }`, which the page
//! shows as such). What is kept still works: the font chosen per bus (`hub_core::busfonts`,
//! handed to the plugin with the duty) and a font file added to the content folder's `Fonts`.
//!
//! * `hub_more_font_view {bus, text, source?}` - `{ failed: true, available: false, options:
//!   [], chosen }`;
//! * `hub_more_font_set {bus, font}` or `{bus, style: {rows, bold, spacing}}` - kept at once;
//! * `hub_more_font_add {bus, file?}` - the font file `file` copied into the content folder's
//!   `Fonts` and chosen for the bus: `{ names, missing, vector }` or `{ error }` (the app has no
//!   file dialog for it yet: without `file` it says so);
//! * `hub_more_font_pic {bus, text, key}` (bytes) - no previews are drawn: an error;
//! * `hub_display_preview {bus, text}` - `{ failed: true }`.

use crate::Answer;
use hub_core::busfonts::{self, BusFonts, DisplayFontSpec};
use serde_json::{json, Value};
use std::path::Path;

/// Why no preview is drawn.
pub const NOT_DRAWN: &str = "The bus's displays are not read yet: no previews of display fonts";

/// The font chosen for a bus as the page reads it.
fn chosen_json(c: &DisplayFontSpec) -> Value {
    json!({ "key": c.key(), "name": c.name, "vector": c.is_vector(), "rows": c.rows, "bold": c.bold, "spacing": c.spacing, "arg": c.to_arg() })
}

pub fn view(args: &Value) -> Answer {
    let bus = super::str_arg(args, "bus");
    let chosen = BusFonts::load().spec_for(&bus);
    Ok(json!({ "failed": true, "available": false, "reason": NOT_DRAWN, "options": [], "chosen": chosen.as_ref().map(chosen_json) }))
}

/// A font chosen for `bus` (`font`: a `DisplayFontSpec` argument, null: as the bus), the
/// settings chosen so far staying with it; or the chosen font's settings (`style`: rows, bold,
/// spacing).
pub fn set(args: &Value) -> Answer {
    let bus = super::str_arg(args, "bus");
    let mut choices = BusFonts::load();
    set_in(&mut choices, &bus, args);
    choices.save().map_err(|e| e.to_string())?;
    Ok(json!(true))
}

fn set_in(choices: &mut BusFonts, bus: &str, args: &Value) {
    if let Some(style) = args.get("style") {
        if let Some(mut f) = choices.spec_for(bus) {
            f.rows = style.get("rows").and_then(Value::as_f64).filter(|r| *r >= 1.0).map(|r| r.round() as u32);
            f.bold = style.get("bold").and_then(Value::as_bool).unwrap_or(false);
            f.spacing = style.get("spacing").and_then(Value::as_f64).filter(|s| *s >= 0.0).map(|s| s.round() as u32);
            choices.set_spec(bus, Some(&f));
        }
    } else {
        let style = choices.spec_for(bus);
        let font = args.get("font").and_then(Value::as_str).and_then(DisplayFontSpec::parse).map(|mut f| {
            if let Some(s) = style.as_ref() {
                (f.rows, f.bold, f.spacing) = (s.rows, s.bold, s.spacing);
            }
            f
        });
        choices.set_spec(bus, font.as_ref());
    }
}

/// "Add font…": the font file `file` into the content folder's `Fonts`, and its first font
/// chosen for `bus`.
pub fn add(args: &Value) -> Answer {
    let bus = super::str_arg(args, "bus");
    let file = super::str_arg(args, "file");
    if file.trim().is_empty() {
        return Ok(json!({ "error": "Choosing a font file here is not available yet: copy it into the Fonts folder of openOMSI's content folder" }));
    }
    let Some(dir) = busfonts::content_fonts_dir() else { return Ok(json!({ "error": "no content folder" })) };
    match busfonts::add_font(Path::new(file.trim()), &dir) {
        Ok(added) => {
            let first = added.names.first().map(|n| match added.file.as_deref() {
                Some(f) => DisplayFontSpec::vector(n, f, 0),
                None => DisplayFontSpec::named(n),
            });
            if let Some(first) = first {
                set(&json!({ "bus": bus, "font": first.to_arg() }))?;
            }
            Ok(json!({ "names": added.names, "missing": added.missing, "vector": added.file.is_some() }))
        }
        Err(e) => Ok(json!({ "error": format!("{e:#}") })),
    }
}

pub fn picture(_args: &Value) -> Result<Vec<u8>, String> {
    Err(NOT_DRAWN.to_string())
}

/// `text` on `bus`'s own display: not drawn yet.
pub fn display_preview(_bus: &str, _text: &str) -> Value {
    json!({ "failed": true, "available": false, "reason": NOT_DRAWN })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_font_and_its_settings_are_kept_per_bus() {
        let mut f = BusFonts::default();
        let bus = "Vehicles/SD200/SD200.bus";
        set_in(&mut f, bus, &json!({ "font": DisplayFontSpec::named("Matrix 16").to_arg() }));
        assert_eq!(f.spec_for(bus).map(|s| s.name), Some("Matrix 16".to_string()));
        set_in(&mut f, bus, &json!({ "style": { "rows": 7, "bold": true, "spacing": null } }));
        let s = f.spec_for(bus).unwrap();
        assert_eq!((s.rows, s.bold, s.spacing), (Some(7), true, None));
        // (another font keeps the settings)
        set_in(&mut f, bus, &json!({ "font": DisplayFontSpec::named("Matrix 8").to_arg() }));
        let s = f.spec_for(bus).unwrap();
        assert_eq!((s.name.as_str(), s.rows, s.bold), ("Matrix 8", Some(7), true));
        // (null: as the bus)
        set_in(&mut f, bus, &json!({ "font": null }));
        assert_eq!(f.spec_for(bus), None);
    }

    #[test]
    fn the_previews_say_they_are_not_drawn() {
        assert_eq!(display_preview("b", "12 Zoo")["failed"], true);
        assert!(picture(&json!({})).is_err());
        assert!(add(&json!({ "bus": "b" })).unwrap()["error"].is_string());
    }
}
