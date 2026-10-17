//! A weather file (`.owt`): what the drive steps show of it, and one's own weather written as
//! one.
//!
//! The keywords: `[name]`, `[description]` … `[end]`, `[fog]` (the visibility in m, the fog's
//! height), `[wind]` (where from in degrees, m/s), `[temp]` (°C, the dew point), `[press]`
//! (hPa), `[clouds]` (the clouds' texture - `-1`: none - and their base in m), `[precip]` (0
//! none, 1 rain, 2 snow; its intensity 0-255; three more values), `[groundwet]` (how wet the
//! roads are, 0-255, and two more values), `[snow]` and `[snowOnRoad]` (no values). Some of
//! OMSI's own weather files are UTF-16.

use crate::text::{parse_f32, parse_i32, CfgFile, Codepage};
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

impl Precipitation {
    /// Its code in the file.
    pub fn code(self) -> i32 {
        match self {
            Precipitation::None => 0,
            Precipitation::Rain => 1,
            Precipitation::Snow => 2,
        }
    }

    pub fn of_code(code: i32) -> Precipitation {
        match code {
            1 => Precipitation::Rain,
            2 => Precipitation::Snow,
            _ => Precipitation::None,
        }
    }
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
    /// The fog's height (`[fog]`'s second value).
    pub fog_height: f32,
    /// Where the wind comes from (degrees) and its speed (m/s).
    pub wind_dir: f32,
    pub wind_speed: f32,
    /// The dew point, °C.
    pub dew_point: f32,
    /// The air pressure, hPa.
    pub pressure: f32,
    /// The clouds' base, m.
    pub cloud_base_m: f32,
    /// `[precip]`'s values after its kind, as the file has them (the first: the intensity,
    /// 0-255).
    pub precip_values: Vec<f32>,
    /// `[groundwet]`'s values as the file has them (the first: the wetness, 0-255; empty: the
    /// file has none).
    pub ground_wet_values: Vec<f32>,
    /// Snow on the roads (`[snowOnRoad]`).
    pub snow_on_road: bool,
    pub codepage: Option<Codepage>,
}

/// The first of `raw` as 0..1, unless `now` (the field) was changed from it: then `now`.
fn raw_or(raw: Option<f32>, now: f32) -> f32 {
    match raw {
        Some(r) if ((r / 255.0) - now).abs() < 1e-6 => r,
        _ => now * 255.0,
    }
}

impl Weather {
    pub fn parse(cfg: &CfgFile) -> Weather {
        let mut w = Weather { path: cfg.path.clone(), codepage: Some(cfg.codepage), ..Weather::default() };
        let mut r = cfg.reader();
        while let Some(k) = r.next_keyword() {
            match k.to_ascii_lowercase().as_str() {
                "name" => w.name = r.value().trim().to_string(),
                "description" => w.description = r.until("end").join("\n").trim().to_string(),
                "fog" => {
                    let v = r.values(2);
                    w.fog_m = v.first().map_or(0.0, |s| parse_f32(s));
                    w.fog_height = v.get(1).map_or(0.0, |s| parse_f32(s));
                }
                "wind" => {
                    let v = r.values(2);
                    w.wind_dir = v.first().map_or(0.0, |s| parse_f32(s));
                    w.wind_speed = v.get(1).map_or(0.0, |s| parse_f32(s));
                }
                "temp" => {
                    let v = r.values(2);
                    w.temperature = v.first().map_or(0.0, |s| parse_f32(s));
                    w.dew_point = v.get(1).map_or(w.temperature, |s| parse_f32(s));
                }
                "press" => w.pressure = parse_f32(r.value()),
                "clouds" => {
                    let v = r.values(2);
                    let tex = v.first().map_or("", |s| s.trim());
                    w.clouds = if tex == "-1" { String::new() } else { tex.to_string() };
                    w.cloud_base_m = v.get(1).map_or(0.0, |s| parse_f32(s));
                }
                "precip" => {
                    w.precipitation = Precipitation::of_code(parse_i32(r.value()));
                    w.precip_values = r.run().into_iter().map(parse_f32).collect();
                    w.intensity = w.precip_values.first().map_or(0.0, |v| (v / 255.0).clamp(0.0, 1.0));
                }
                "groundwet" => {
                    w.ground_wet_values = r.run().into_iter().map(parse_f32).collect();
                    w.ground_wet = w.ground_wet_values.first().map_or(0.0, |v| (v / 255.0).clamp(0.0, 1.0));
                }
                "snow" => w.snow = true,
                "snowonroad" => w.snow_on_road = true,
                _ => {}
            }
        }
        w
    }

    pub fn load(path: &Path) -> Result<Weather, ReadError> {
        let mut w = Weather::parse(&CfgFile::read(path)?);
        w.path = path.to_path_buf();
        Ok(w)
    }

    /// The file's text again (`\r\n` line ends), its fields as they are now.
    pub fn to_text(&self) -> String {
        let mut s = String::from("====================================\r\n\tOmsi Weather File (*.owt), written by Omsi-Hub\r\n====================================\r\n\r\n");
        s.push_str(&format!("[name]\r\n{}\r\n\r\n[description]\r\n{}\r\n[end]\r\n\r\n", self.name, self.description.replace('\n', "\r\n")));
        s.push_str(&format!("[fog]\r\n{}\r\n{}\r\n\r\n", self.fog_m, self.fog_height));
        s.push_str(&format!("[wind]\r\n{}\r\n{}\r\n\r\n", self.wind_dir, self.wind_speed));
        s.push_str(&format!("[temp]\r\n{}\r\n{}\r\n\r\n", self.temperature, self.dew_point));
        s.push_str(&format!("[press]\r\n{}\r\n\r\n", self.pressure));
        let clouds = if self.clouds.trim().is_empty() { "-1" } else { self.clouds.trim() };
        s.push_str(&format!("[clouds]\r\n{clouds}\r\n{}\r\n\r\n", self.cloud_base_m));
        s.push_str(&format!("[precip]\r\n{}\r\n{}\r\n", self.precipitation.code(), raw_or(self.precip_values.first().copied(), self.intensity)));
        for v in self.precip_values.iter().skip(1) {
            s.push_str(&format!("{v}\r\n"));
        }
        s.push_str("\r\n");
        // (a file without it keeps none, unless the roads are wet now)
        if !self.ground_wet_values.is_empty() || self.ground_wet != 0.0 {
            s.push_str(&format!("[groundwet]\r\n{}\r\n", raw_or(self.ground_wet_values.first().copied(), self.ground_wet)));
            for v in self.ground_wet_values.iter().skip(1) {
                s.push_str(&format!("{v}\r\n"));
            }
            s.push_str("\r\n");
        }
        if self.snow {
            s.push_str("[snow]\r\n\r\n");
        }
        if self.snow_on_road {
            s.push_str("[snowOnRoad]\r\n\r\n");
        }
        s
    }

    /// Written to `path` in its codepage (Windows-1252 for a new one).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        crate::timetable::write_text(path, &self.to_text(), self.codepage.unwrap_or(Codepage::Windows1252))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_weather_file_is_read_and_written_back() {
        let text = "\t[fog]\r\n\t{range}\r\n[name]\r\nGrey day\r\n\r\n[description]\r\nRain and wind.\r\n[end]\r\n\r\n[fog]\r\n199.99999686135\r\n0.99999998430675\r\n\r\n[wind]\r\n230\r\n4.3\r\n\r\n[temp]\r\n15\r\n13.4\r\n\r\n[press]\r\n997\r\n\r\n[clouds]\r\nOvercast 1\r\n200\r\n\r\n[precip]\r\n1\r\n173\r\n0\r\n-2\r\n0\r\n\r\n[groundwet]\r\n78\r\n255\r\n115.9\r\n\r\n[snowOnRoad]\r\n";
        let w = Weather::parse(&CfgFile::from_str("grey.owt", text));
        assert_eq!((w.name.as_str(), w.description.as_str(), w.clouds.as_str()), ("Grey day", "Rain and wind.", "Overcast 1"));
        assert_eq!((w.fog_m, w.wind_dir, w.wind_speed, w.temperature, w.dew_point, w.pressure, w.cloud_base_m), (200.0, 230.0, 4.3, 15.0, 13.4, 997.0, 200.0));
        assert_eq!((w.precipitation, w.precip_values.len(), w.snow, w.snow_on_road), (Precipitation::Rain, 4, false, true));
        assert!((w.intensity - 173.0 / 255.0).abs() < 1e-6 && (w.ground_wet - 78.0 / 255.0).abs() < 1e-6);
        let again = Weather::parse(&CfgFile::from_str("grey.owt", &w.to_text()));
        assert_eq!(again, w);
        // (a field changed is written as it is now)
        let mut snowy = w.clone();
        (snowy.precipitation, snowy.intensity, snowy.ground_wet, snowy.snow, snowy.clouds) = (Precipitation::Snow, 0.5, 0.0, true, String::new());
        let back = Weather::parse(&CfgFile::from_str("snowy.owt", &snowy.to_text()));
        assert_eq!((back.precipitation, back.intensity, back.ground_wet, back.snow, back.clouds.as_str()), (Precipitation::Snow, 0.5, 0.0, true, ""));
        assert!(snowy.to_text().contains("[clouds]\r\n-1\r\n"));
    }
}
