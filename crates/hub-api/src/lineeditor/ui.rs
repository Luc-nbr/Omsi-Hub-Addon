//! What the line editor took of the game program's interface (`omsi_ui`), on Omsi-Hub's own
//! parts: its texts in the page's language (`company::words`), and the tones of its notes as the
//! page names them (the theme's colours were only ever compared, never drawn here).

use std::borrow::Cow;

/// `text` in the interface's language.
pub fn tr(text: &str) -> Cow<'static, str> {
    Cow::Owned(crate::company::words::tr(text))
}

pub mod i18n {
    /// The interface's language ("" until one is set).
    pub fn language() -> String {
        crate::company::words::language()
    }

    pub fn set_language(lang: &str) {
        crate::company::words::set_language(lang)
    }
}

/// A note's tone (the game interface's theme colour it was drawn in).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Ok,
    Warn,
    Danger,
    Soft,
    Dim,
    Faint,
}

pub mod theme {
    use super::Color;
    pub const OK: Color = Color::Ok;
    pub const WARN: Color = Color::Warn;
    pub const DANGER: Color = Color::Danger;
    pub const TEXT_SOFT: Color = Color::Soft;
    pub const TEXT_DIM: Color = Color::Dim;
    pub const TEXT_FAINT: Color = Color::Faint;
}

/// The game program kept these texts from machine translation; the page has none.
pub fn mt_protect<'a>(_texts: impl IntoIterator<Item = &'a str>) {}
