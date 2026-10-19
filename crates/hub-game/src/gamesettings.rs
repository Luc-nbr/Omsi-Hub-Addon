//! openOMSI's own settings (`~/.openomsi/settings.cfg`, `C:\Users\<you>\.openomsi` on Windows),
//! changed from Omsi-Hub's Settings page: the keys openOMSI's user guide names, each with the
//! values it allows ([`FIELDS`]).
//!
//! The file is `key=value` lines, written by openOMSI's launcher or by hand; openOMSI keeps the
//! keys its page does not manage, and so does this. A change touches the key's own lines only:
//! every other line - comments, blank lines, the keys Omsi-Hub does not know, their order - stays
//! as it was, and so do the line ends. A key set back to openOMSI's default is taken out, so that
//! openOMSI's own default holds again. Nothing is written while the game runs (the caller's
//! check): openOMSI reads the file when it starts.

use std::path::{Path, PathBuf};

/// What a setting takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// `1` or `0`.
    Switch,
    /// One of the values, each with its label.
    Choice(&'static [(&'static str, &'static str)]),
    /// A number from `min` to `max` in steps of `step`; `default` when openOMSI's guide says it.
    Number { min: f64, max: f64, step: f64, default: Option<f64> },
    /// A value the guide names without its values (`hint` says what it takes): one line of text.
    Text { hint: &'static str },
}

/// A setting of openOMSI that Omsi-Hub changes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field {
    pub key: &'static str,
    /// The group it is shown in, and its label (English: the page translates them).
    pub group: &'static str,
    pub label: &'static str,
    pub kind: Kind,
}

const fn field(group: &'static str, key: &'static str, label: &'static str, kind: Kind) -> Field {
    Field { key, group, label, kind }
}

const fn number(min: f64, max: f64, step: f64, default: Option<f64>) -> Kind {
    Kind::Number { min, max, step, default }
}

/// The groups, in the page's order.
pub const GROUPS: [&str; 7] = ["Graphics", "Mirrors", "Screen and navigator", "Language", "Sound and radio", "Driving and controls", "Passengers and traffic"];

const fn text(hint: &'static str) -> Kind {
    Kind::Text { hint }
}

/// Every key of `settings.cfg` that openOMSI's user guide (`docs/USER_GUIDE.md`) names, with the
/// values it gives: a choice or a range where it says them, else a line of text with the guide's
/// words as its hint. (`version` is the file's own, and `texture_memory` and `navigator_opacity`
/// are old names openOMSI no longer writes: none of them is changed here.)
pub const FIELDS: &[Field] = &[
    // the graphics
    field("Graphics", "graphics_api", "Graphics API", Kind::Choice(&[("auto", "Automatic"), ("vulkan", "Vulkan"), ("dx12", "DirectX 12"), ("gl", "OpenGL"), ("angle", "ANGLE")])),
    field("Graphics", "enhanced", "Enhanced graphics (physically based)", Kind::Switch),
    field("Graphics", "graphics", "Graphics mode", Kind::Choice(&[("enhanced_plus", "Enhanced+")])),
    field("Graphics", "fullscreen", "Full screen", Kind::Switch),
    field("Graphics", "vsync", "Vertical sync", Kind::Switch),
    field("Graphics", "render_scale", "Render scale", text("auto, or a fraction such as 0.75")),
    field("Graphics", "msaa", "Anti-aliasing (MSAA)", Kind::Choice(&[("1", "Off"), ("2", "2×"), ("4", "4×"), ("8", "8×")])),
    field("Graphics", "post_aa", "Smoothing after the picture (FXAA)", Kind::Choice(&[("fxaa", "On"), ("off", "Off")])),
    field("Graphics", "anisotropy", "Texture filtering", Kind::Choice(&[("1", "Off"), ("2", "2×"), ("4", "4×"), ("8", "8×"), ("16", "16×")])),
    field("Graphics", "view_distance", "View distance (metres)", number(100.0, 10000.0, 50.0, None)),
    field("Graphics", "video_memory", "Video memory (MB; default: nine tenths of the card's)", number(256.0, 65536.0, 256.0, None)),
    field("Graphics", "shadows", "Shadows", Kind::Switch),
    field("Graphics", "shadow_size", "Shadow map size", text("a size, as openOMSI's Settings page writes it")),
    field("Graphics", "shadow_blobs", "Shadow meshes and blobs under vehicles", Kind::Switch),
    field("Graphics", "ssao", "Ambient occlusion", Kind::Switch),
    field("Graphics", "reflections", "Reflections (0: off)", number(0.0, 1.0, 0.05, None)),
    field("Graphics", "led_glow", "Glow of LED displays (0: off)", number(0.0, 15.0, 1.0, None)),
    field("Graphics", "led_mips", "Sharpness of LED displays (mip level)", number(0.0, 4.0, 0.05, Some(1.3))),
    field("Graphics", "detail_textures", "Detail textures", Kind::Switch),
    field("Graphics", "texture_compression", "Texture compression", Kind::Switch),
    field("Graphics", "gpu_texture_compression", "Compressed textures kept on the graphics card", Kind::Switch),
    field("Graphics", "cloud_quality", "Cloud quality", Kind::Choice(&[("low", "Low")])),
    field("Graphics", "rain_quality", "Rain quality", text("High, Medium or Low")),
    field("Graphics", "puddles", "Puddles", text("High, Low or Off")),
    // the mirrors
    field("Mirrors", "mirror_size", "Mirror size (0: no mirrors)", text("a size, as openOMSI's Settings page writes it")),
    field("Mirrors", "mirror_refresh", "Mirror refresh", text("a rate, as openOMSI's Settings page writes it")),
    field("Mirrors", "mirror_hud", "Mirror panels", Kind::Choice(&[("0", "Off"), ("1", "The right mirror"), ("2", "The left mirror"), ("3", "Both mirrors")])),
    field("Mirrors", "mirror_hud_every_frame", "Mirror panels drawn every frame", Kind::Switch),
    // the screen and the navigator
    field("Screen and navigator", "ui_scale", "Size of the game's menus", number(0.5, 2.0, 0.05, Some(1.0))),
    field("Screen and navigator", "ui_scale_window", "Menus grow with large windows", Kind::Switch),
    field("Screen and navigator", "ui_opacity", "Opacity of the game's panels", number(0.2, 1.0, 0.05, Some(0.85))),
    field("Screen and navigator", "notes", "Notes in the top left corner", Kind::Switch),
    field("Screen and navigator", "navigator", "Navigator", Kind::Switch),
    field(
        "Screen and navigator",
        "navigator_corner",
        "Navigator's corner",
        Kind::Choice(&[("bottom-left", "Bottom left"), ("bottom-right", "Bottom right"), ("top-left", "Top left"), ("top-right", "Top right")]),
    ),
    field("Screen and navigator", "nav_light", "Navigator in a light look", Kind::Switch),
    field("Screen and navigator", "nav_ai", "Other vehicles on the navigator", Kind::Switch),
    field("Screen and navigator", "nav_surface", "Water, land and rail on the navigator", Kind::Switch),
    // the language of the game
    field("Language", "language", "The game's language", Kind::Choice(&[("ENG", "English"), ("DEU", "Deutsch"), ("FRA", "Français")])),
    // sound and the radio
    field("Sound and radio", "volume", "Volume", text("as openOMSI's Settings page writes it")),
    field("Sound and radio", "radio_speaker", "Radio through a small speaker", number(0.0, 1.0, 0.05, Some(0.0))),
    field("Sound and radio", "radio_hiss", "Radio: FM hiss", number(0.0, 1.0, 0.05, Some(0.0))),
    field("Sound and radio", "radio_dropouts", "Radio: signal dropouts", number(0.0, 1.0, 0.05, Some(0.0))),
    // driving
    field("Driving and controls", "drive_keys", "Driving keys", Kind::Choice(&[("simple", "Simple"), ("wasd", "W A S D"), ("arrows", "Arrow keys"), ("omsi", "As in OMSI 2")])),
    field("Driving and controls", "wheel_range", "Steering wheel's turn (degrees)", number(180.0, 1080.0, 10.0, Some(900.0))),
    field("Driving and controls", "wheel_lock", "Wheel turn for the bus's full lock (degrees; 0: the whole wheel)", number(0.0, 1080.0, 10.0, None)),
    field("Driving and controls", "steering_linear", "Linear steering", Kind::Switch),
    field("Driving and controls", "old_steering", "Old steering (the wheel stays where you leave it)", Kind::Switch),
    field("Driving and controls", "ff_invert", "Force feedback the other way", Kind::Switch),
    field("Driving and controls", "mouse_sens", "Mouse sensitivity (1: as in OMSI 2)", number(0.1, 3.0, 0.05, Some(1.0))),
    field("Driving and controls", "mouse_smooth", "Smooth mouse steering", Kind::Switch),
    field("Driving and controls", "mouse_hold", "Hold the cursor while the mouse steers", Kind::Switch),
    field("Driving and controls", "right_stick_look", "Look round with the right stick", Kind::Switch),
    field("Driving and controls", "ctrl_off", "Controllers switched off", text("their names, separated by |")),
    field("Driving and controls", "fov", "Field of view (degrees; 0: the bus's cameras)", number(0.0, 120.0, 1.0, Some(0.0))),
    field("Driving and controls", "head_idle", "Head sway", number(0.0, 1.0, 0.05, Some(0.0))),
    field("Driving and controls", "head_idle_pace", "Pace of the head sway", number(0.5, 2.0, 0.05, Some(1.0))),
    field("Driving and controls", "collision_objects", "Collisions with objects", Kind::Switch),
    field("Driving and controls", "collision_vehicles", "Collisions with vehicles", Kind::Switch),
    // the passengers and the traffic
    field("Passengers and traffic", "boarding", "Boarding", Kind::Choice(&[("auto", "Automatic"), ("pay", "Passengers pay at the driver"), ("walk", "Passengers walk in")])),
    field("Passengers and traffic", "exact_fare", "Passengers pay the exact fare", Kind::Switch),
    field("Passengers and traffic", "pax_bad_driving", "Passengers mind bad driving", Kind::Switch),
    field("Passengers and traffic", "ai_wait_timed_stops_only", "Timetable buses wait only at timed stops", Kind::Switch),
];

/// The setting called `key`.
pub fn field_of(key: &str) -> Option<&'static Field> {
    FIELDS.iter().find(|f| f.key == key)
}

/// Where openOMSI keeps its settings, for the home folder `home`.
pub fn path(home: &Path) -> PathBuf {
    home.join(".openomsi").join("settings.cfg")
}

/// The line's key and value, when it is a `key=value` line (not a comment).
fn key_value(line: &str) -> Option<(&str, &str)> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') || t.starts_with(';') || t.starts_with("//") {
        return None;
    }
    let (k, v) = t.split_once('=')?;
    Some((k.trim(), v.trim()))
}

/// The value of `key` in the file's text (the last of its lines, as a file read top to bottom
/// leaves it; None: not there).
pub fn value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().filter_map(key_value).rfind(|(k, _)| *k == key).map(|(_, v)| v)
}

/// Why a value is refused for a setting.
#[derive(Debug, Clone, PartialEq)]
pub enum Refusal {
    UnknownKey(String),
    BadValue { key: String, value: String },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::UnknownKey(k) => write!(f, "{k} is no setting Omsi-Hub changes"),
            Refusal::BadValue { key, value } => write!(f, "{value} is no value of {key}"),
        }
    }
}

/// `value` as `field` writes it (a number in its range, on its steps; a choice of its values),
/// or why not.
pub fn checked(field: &Field, value: &str) -> Result<String, Refusal> {
    let v = value.trim();
    let bad = || Refusal::BadValue { key: field.key.to_string(), value: v.to_string() };
    match field.kind {
        Kind::Switch => match v {
            "1" | "0" => Ok(v.to_string()),
            _ => Err(bad()),
        },
        Kind::Choice(options) => options.iter().find(|(o, _)| *o == v).map(|(o, _)| o.to_string()).ok_or_else(bad),
        // (one line, as a `key=value` file holds it)
        Kind::Text { .. } => {
            if v.chars().count() > 200 || v.chars().any(char::is_control) {
                return Err(bad());
            }
            Ok(v.to_string())
        }
        Kind::Number { min, max, step, .. } => {
            let n: f64 = v.parse().ok().filter(|n: &f64| n.is_finite()).ok_or_else(bad)?;
            if n < min - 1e-9 || n > max + 1e-9 {
                return Err(bad());
            }
            let stepped = min + ((n - min) / step).round() * step;
            // (written without the float's noise: 0.85, not 0.8500000000000001)
            let text = format!("{:.4}", stepped.clamp(min, max));
            Ok(text.trim_end_matches('0').trim_end_matches('.').to_string())
        }
    }
}

/// The file's text with `key` set to `value` (None: taken out, openOMSI's default again). Every
/// line of the key gets the value; a key not there yet goes at the end. The other lines and the
/// file's line ends stay as they were.
pub fn with(text: &str, key: &str, value: Option<&str>) -> String {
    let crlf = text.contains("\r\n");
    let nl = if crlf { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = Vec::new();
    let mut found = false;
    for line in text.lines() {
        match key_value(line) {
            Some((k, _)) if k == key => {
                found = true;
                if let Some(v) = value {
                    lines.push(format!("{key}={v}"));
                }
            }
            _ => lines.push(line.to_string()),
        }
    }
    if !found {
        if let Some(v) = value {
            lines.push(format!("{key}={v}"));
        }
    }
    let mut out = lines.join(nl);
    if !out.is_empty() {
        out.push_str(nl);
    }
    out
}

/// `text` written to `path` through a file beside it (a write cut off leaves the old file).
pub fn write(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let part = path.with_extension("cfg.omsihub-part");
    std::fs::write(&part, text)?;
    std::fs::rename(&part, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&part);
    })
}

/// The setting `key` set to `value` (None or "": openOMSI's default) in the settings file at
/// `path`. Returns the value written.
pub fn set(path: &Path, key: &str, value: Option<&str>) -> Result<Option<String>, String> {
    let field = field_of(key).ok_or_else(|| Refusal::UnknownKey(key.to_string()).to_string())?;
    let value = match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => Some(checked(field, v).map_err(|e| e.to_string())?),
        None => None,
    };
    let text = match std::fs::read(path) {
        // (a file that is no plain text is not written over: its other lines would change)
        Ok(b) => String::from_utf8(b).map_err(|_| format!("{} is not plain text: not changed", path.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let new = with(&text, key, value.as_deref());
    if new != text {
        write(path, &new).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "# openOMSI settings\r\nversion=3\r\nmsaa=4\r\nmy_mod_key=x\r\n\r\nfullscreen=1\r\nmsaa=2\r\n";

    #[test]
    fn a_change_touches_only_the_key_s_own_lines() {
        let t = with(FILE, "msaa", Some("8"));
        assert_eq!(t, "# openOMSI settings\r\nversion=3\r\nmsaa=8\r\nmy_mod_key=x\r\n\r\nfullscreen=1\r\nmsaa=8\r\n");
        // (a new key at the end, the line ends kept)
        assert_eq!(with(FILE, "vsync", Some("0")), format!("{FILE}vsync=0\r\n"));
        // (back to openOMSI's default: the key's lines out, the rest as it was)
        assert_eq!(with(FILE, "msaa", None), "# openOMSI settings\r\nversion=3\r\nmy_mod_key=x\r\n\r\nfullscreen=1\r\n");
        assert_eq!(with("", "fov", Some("70")), "fov=70\n");
        assert_eq!(with("a=1", "a", None), "");
    }

    #[test]
    fn a_value_is_read_as_the_file_leaves_it() {
        assert_eq!(value(FILE, "msaa"), Some("2"), "the last line of the key");
        assert_eq!(value(FILE, "fullscreen"), Some("1"));
        assert_eq!(value("#msaa=4\n; fov=3", "msaa"), None, "comments are no values");
        assert_eq!(value(" ui_scale = 1.25 ", "ui_scale"), Some("1.25"));
    }

    #[test]
    fn only_the_values_a_setting_allows_are_written() {
        let f = |k: &str| field_of(k).unwrap();
        assert_eq!(checked(f("msaa"), "4"), Ok("4".into()));
        assert!(checked(f("msaa"), "3").is_err());
        assert_eq!(checked(f("fullscreen"), "1"), Ok("1".into()));
        assert!(checked(f("fullscreen"), "yes").is_err());
        assert_eq!(checked(f("ui_scale"), "1.2300"), Ok("1.25".into()), "on its steps");
        assert_eq!(checked(f("ui_opacity"), "0.85"), Ok("0.85".into()));
        assert_eq!(checked(f("wheel_range"), "900"), Ok("900".into()));
        assert!(checked(f("ui_scale"), "5").is_err() && checked(f("fov"), "-1").is_err() && checked(f("fov"), "NaN").is_err());
        assert_eq!(checked(f("render_scale"), " 0.75 "), Ok("0.75".into()), "a line of text");
        assert!(checked(f("ctrl_off"), "a\nb").is_err());
        assert!(field_of("texture_memory").is_none() && field_of("version").is_none(), "old names and the file's own key are left alone");
    }

    #[test]
    fn every_setting_is_in_a_group_and_its_choices_are_its_own() {
        for f in FIELDS {
            assert!(GROUPS.contains(&f.group), "{}", f.key);
            assert_eq!(FIELDS.iter().filter(|x| x.key == f.key).count(), 1, "{} twice", f.key);
            if let Kind::Number { min, max, default: Some(d), .. } = f.kind {
                assert!(min <= d && d <= max, "{}", f.key);
            }
        }
    }

    #[test]
    fn the_file_is_written_and_a_wrong_value_leaves_it_alone() {
        let dir = std::env::temp_dir().join(format!("hub-game-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = path(&dir);
        assert_eq!(set(&p, "language", Some("DEU")), Ok(Some("DEU".into())), "the file made");
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "language=DEU\n");
        std::fs::write(&p, FILE).unwrap();
        assert!(set(&p, "msaa", Some("3")).unwrap_err().contains("no value"));
        assert!(set(&p, "my_mod_key", Some("y")).unwrap_err().contains("no setting"));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), FILE, "refused: not written");
        assert_eq!(set(&p, "msaa", Some("")), Ok(None));
        assert!(!std::fs::read_to_string(&p).unwrap().contains("msaa"));
        assert!(!p.with_extension("cfg.omsihub-part").exists());
        std::fs::write(&p, b"msaa=4\nname=M\xfcller\n").unwrap();
        assert!(set(&p, "msaa", Some("2")).unwrap_err().contains("not plain text"));
        assert_eq!(std::fs::read(&p).unwrap(), b"msaa=4\nname=M\xfcller\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
