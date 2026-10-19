//! How the line editor writes money, dates and bus names, as the bus company's pages and the
//! bus step write them: money as `company::words::eur` writes it, a day as "Tue 5 Mar 1989",
//! bus names in the order a person sorts them (2 before 10).

use crate::launcher::company::{self as co, Cents};
use std::cmp::Ordering;
use std::path::Path;

/// Whole money, as the company's pages write it (`company::words::eur`: the currency chosen,
/// grouped as the language groups it).
pub(crate) fn eur(c: Cents) -> String {
    crate::company::words::eur(c)
}

const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// "Tue 5 Mar 1989".
pub(crate) fn day_label(date: &str) -> String {
    let Some(d) = co::dates::parse(date) else { return date.to_string() };
    let (y, m, day) = co::dates::civil_from_days(d);
    format!("{} {} {} {}", crate::lineeditor::ui::tr(WEEKDAYS[co::dates::weekday(d) as usize]), day, crate::lineeditor::ui::tr(MONTHS[(m as usize).clamp(1, 12) - 1]), y)
}

/// Names in the order a person sorts them: letters without regard to case, numbers by their
/// value (2 before 10).
pub(crate) fn name_cmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let x: String = std::iter::from_fn(|| a.next_if(|c| c.is_ascii_digit())).collect();
                let y: String = std::iter::from_fn(|| b.next_if(|c| c.is_ascii_digit())).collect();
                let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
                let o = x.len().cmp(&y.len()).then_with(|| x.cmp(y));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(x), Some(y)) => {
                if x != y {
                    return x.cmp(&y);
                }
                a.next();
                b.next();
            }
        }
    }
}

/// The model and the version of a bus: `[friendlyname]`'s second line split at its first
/// " - "; without one, the version is the bus's own paint, or its file's name.
pub(crate) fn split_type(type_name: &str, file: &str, default_paint: &str) -> (String, String) {
    use crate::launcher::{display_bus_name, vehicle_type_label};
    let label = vehicle_type_label(type_name, Path::new(file));
    let parts: Vec<&str> = label.split(" - ").map(str::trim).filter(|s| !s.is_empty()).collect();
    let model = parts.first().map(|s| s.to_string()).unwrap_or_else(|| label.clone());
    let version = if parts.len() > 1 {
        parts[1..].join(" · ")
    } else if !default_paint.trim().is_empty() {
        display_bus_name(default_paint)
    } else {
        display_bus_name(&Path::new(file).file_stem().unwrap_or_default().to_string_lossy())
    };
    (model, version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_sorted_as_a_person_sorts_them() {
        let mut v = vec!["Line 10", "line 2", "Line 1"];
        v.sort_by(|a, b| name_cmp(a, b));
        assert_eq!(v, ["Line 1", "line 2", "Line 10"]);
    }
}
