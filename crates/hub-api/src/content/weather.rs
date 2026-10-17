//! One's own weather on the day step: a weather value of its own (`custom:vis=…;br=…;…`) as
//! the step's editor shows its fields, a preset (`Weather/<name>.owt`) made one's own to edit,
//! and the fields written back.
//!
//! * `hub_more_weather_read {weather}` - `{ custom, as_custom, fresh }`: the fields when the
//!   weather is one's own (else null), the preset as one's own weather's value (else null), and
//!   the value of a fresh one;
//! * `hub_more_weather_write {custom}` - `{ weather, custom }`: the fields as the weather's
//!   value, made sound first, and the fields as they are kept.
//!
//! The value is the drive's `weather` as any other, and goes to the game as it is.

use crate::{arg, Answer};
use serde_json::{json, Value};

/// What a value of one's own weather starts with.
pub const PREFIX: &str = "custom:";

/// One's own weather.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomWeather {
    /// m, 50 to 50 000.
    pub visibility_m: f32,
    /// The daylight's strength, 0 to 1.5.
    pub brightness: f32,
    /// Where the wind comes from (degrees) and its speed (m/s).
    pub wind_dir: f32,
    pub wind_speed: f32,
    /// °C, and the relative humidity (%).
    pub temp_c: f32,
    pub humidity: f32,
    /// hPa.
    pub pressure: f32,
    /// 0 (a clear sky) to 1 (overcast), and the clouds' base (m).
    pub cloud_cover: f32,
    pub cloud_base_m: f32,
    /// 0 none, 1 rain, 2 snow; its intensity 0 to 255.
    pub precip: i32,
    pub precip_intensity: f32,
    /// How wet the roads are, 0 to 1.
    pub road_wetness: f32,
    pub snow_cover: bool,
    pub snow_on_road: bool,
}

impl Default for CustomWeather {
    fn default() -> CustomWeather {
        CustomWeather {
            visibility_m: 20_000.0,
            brightness: 1.0,
            wind_dir: 270.0,
            wind_speed: 3.0,
            temp_c: 15.0,
            humidity: 60.0,
            pressure: 1013.0,
            cloud_cover: 0.3,
            cloud_base_m: 1500.0,
            precip: 0,
            precip_intensity: 0.0,
            road_wetness: 0.0,
            snow_cover: false,
            snow_on_road: false,
        }
    }
}

/// `v` within `lo..=hi` (a value that is no number: `d`), never `-0`.
fn within(v: f32, lo: f32, hi: f32, d: f32) -> f32 {
    if v.is_finite() {
        v.clamp(lo, hi) + 0.0
    } else {
        d
    }
}

impl CustomWeather {
    /// Every field within what the game takes.
    pub fn normalize(&mut self) {
        let d = CustomWeather::default();
        self.visibility_m = within(self.visibility_m, 50.0, 50_000.0, d.visibility_m);
        self.brightness = within(self.brightness, 0.0, 1.5, d.brightness);
        self.wind_dir = if self.wind_dir.is_finite() { self.wind_dir.rem_euclid(360.0) + 0.0 } else { d.wind_dir };
        self.wind_speed = within(self.wind_speed, 0.0, 40.0, d.wind_speed);
        self.temp_c = within(self.temp_c, -50.0, 50.0, d.temp_c);
        self.humidity = within(self.humidity, 0.0, 100.0, d.humidity);
        self.pressure = within(self.pressure, 870.0, 1085.0, d.pressure);
        self.cloud_cover = within(self.cloud_cover, 0.0, 1.0, d.cloud_cover);
        self.cloud_base_m = within(self.cloud_base_m, 0.0, 20_000.0, d.cloud_base_m);
        self.precip = self.precip.clamp(0, 2);
        self.precip_intensity = within(self.precip_intensity, 0.0, 255.0, d.precip_intensity);
        self.road_wetness = within(self.road_wetness, 0.0, 1.0, d.road_wetness);
    }

    /// The weather's value: `custom:` and its fields, each as precise as it matters.
    pub fn encode(&self) -> String {
        let b = |x: bool| u8::from(x);
        format!(
            "{PREFIX}vis={:.0};br={:.2};wd={:.0};ws={:.1};t={:.1};rh={:.0};p={:.0};c={:.3};cb={:.0};pt={};pi={:.0};wet={:.2};snow={};snowroad={}",
            self.visibility_m,
            self.brightness,
            self.wind_dir,
            self.wind_speed,
            self.temp_c,
            self.humidity,
            self.pressure,
            self.cloud_cover,
            self.cloud_base_m,
            self.precip,
            self.precip_intensity,
            self.road_wetness,
            b(self.snow_cover),
            b(self.snow_on_road)
        )
    }

    /// One's own weather of the value `w` (None: it is not one); a field it lacks or cannot be
    /// read keeps the default.
    pub fn decode(w: &str) -> Option<CustomWeather> {
        let t = w.trim();
        let body = t.get(..PREFIX.len()).filter(|p| p.eq_ignore_ascii_case(PREFIX)).map(|_| &t[PREFIX.len()..])?;
        let mut c = CustomWeather::default();
        for part in body.split(';') {
            let Some((k, v)) = part.split_once('=') else { continue };
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
            let Ok(x) = v.parse::<f32>() else { continue };
            match k.as_str() {
                "vis" => c.visibility_m = x,
                "br" => c.brightness = x,
                "wd" => c.wind_dir = x,
                "ws" => c.wind_speed = x,
                "t" => c.temp_c = x,
                "rh" => c.humidity = x,
                "p" => c.pressure = x,
                "c" => c.cloud_cover = x,
                "cb" => c.cloud_base_m = x,
                "pt" => c.precip = x.round() as i32,
                "pi" => c.precip_intensity = x,
                "wet" => c.road_wetness = x,
                "snow" => c.snow_cover = x != 0.0,
                "snowroad" => c.snow_on_road = x != 0.0,
                _ => {}
            }
        }
        c.normalize();
        Some(c)
    }

    /// A preset as one's own weather: its fields taken over (the humidity from its dew point;
    /// a sky with clouds as mostly covered, one without as clear).
    pub fn from_weather(w: &hub_omsi::weather::Weather) -> CustomWeather {
        let mut c = CustomWeather {
            visibility_m: w.fog_m,
            wind_dir: w.wind_dir,
            wind_speed: w.wind_speed,
            temp_c: w.temperature,
            humidity: humidity_of(w.temperature, w.dew_point),
            pressure: if w.pressure > 0.0 { w.pressure } else { 1013.0 },
            cloud_cover: if w.clouds.trim().is_empty() { 0.0 } else { 0.7 },
            cloud_base_m: w.cloud_base_m,
            precip: w.precipitation.code(),
            precip_intensity: w.intensity * 255.0,
            road_wetness: w.ground_wet,
            snow_cover: w.snow,
            snow_on_road: w.snow_on_road,
            ..CustomWeather::default()
        };
        c.normalize();
        c
    }

    /// The fields as the editor shows them.
    pub fn to_json(&self) -> Value {
        json!({
            "visibility_m": self.visibility_m, "brightness": self.brightness, "wind_dir": self.wind_dir, "wind_speed": self.wind_speed,
            "temp_c": self.temp_c, "humidity": self.humidity, "pressure": self.pressure, "cloud_cover": self.cloud_cover,
            "cloud_base_m": self.cloud_base_m, "precip": self.precip, "precip_intensity": self.precip_intensity,
            "road_wetness": self.road_wetness, "snow_cover": self.snow_cover, "snow_on_road": self.snow_on_road,
        })
    }

    /// The editor's fields (a field left out keeps the default), made sound.
    pub fn of_json(v: &Value) -> CustomWeather {
        let mut c = CustomWeather::default();
        let f = |k: &str, d: f32| v.get(k).and_then(Value::as_f64).map_or(d, |x| x as f32);
        let b = |k: &str, d: bool| v.get(k).and_then(|x| x.as_bool().or_else(|| x.as_f64().map(|n| n != 0.0))).unwrap_or(d);
        c.visibility_m = f("visibility_m", c.visibility_m);
        c.brightness = f("brightness", c.brightness);
        c.wind_dir = f("wind_dir", c.wind_dir);
        c.wind_speed = f("wind_speed", c.wind_speed);
        c.temp_c = f("temp_c", c.temp_c);
        c.humidity = f("humidity", c.humidity);
        c.pressure = f("pressure", c.pressure);
        c.cloud_cover = f("cloud_cover", c.cloud_cover);
        c.cloud_base_m = f("cloud_base_m", c.cloud_base_m);
        c.precip = f("precip", c.precip as f32).round() as i32;
        c.precip_intensity = f("precip_intensity", c.precip_intensity);
        c.road_wetness = f("road_wetness", c.road_wetness);
        c.snow_cover = b("snow_cover", c.snow_cover);
        c.snow_on_road = b("snow_on_road", c.snow_on_road);
        c.normalize();
        c
    }
}

/// The relative humidity (%) of a temperature and its dew point (°C, the Magnus formula).
fn humidity_of(t: f32, dew: f32) -> f32 {
    let m = |x: f32| (17.62 * x / (243.12 + x)).exp();
    (100.0 * m(dew) / m(t)).clamp(0.0, 100.0)
}

/// A preset's value: a weather file, not the cycle, a METAR report or one's own.
fn is_preset(w: &str) -> bool {
    let l = w.trim().to_ascii_lowercase();
    !(l.is_empty() || l == "cycle" || l.starts_with("metar:") || l.starts_with(PREFIX))
}

pub fn read(args: &Value) -> Answer {
    let w = arg::<Option<String>>(args, "weather")?.unwrap_or_default();
    let preset = is_preset(&w).then(super::vfs).flatten().and_then(|v| v.resolve(&w)).and_then(|p| hub_omsi::weather::Weather::load(&p).ok());
    Ok(read_of(&w, preset.as_ref()))
}

/// `{ custom, as_custom, fresh }` of the weather `w` (`preset`: its file read, when it is one).
pub fn read_of(w: &str, preset: Option<&hub_omsi::weather::Weather>) -> Value {
    let custom = CustomWeather::decode(w).map(|c| c.to_json());
    let as_custom = preset.filter(|_| is_preset(w)).map(|p| CustomWeather::from_weather(p).encode());
    json!({ "custom": custom, "as_custom": as_custom, "fresh": CustomWeather::default().encode() })
}

pub fn write(args: &Value) -> Answer {
    let c = CustomWeather::of_json(args.get("custom").unwrap_or(&Value::Null));
    Ok(json!({ "weather": c.encode(), "custom": c.to_json() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ones_own_weather_goes_through_the_editor_unchanged() {
        let w = "custom:vis=1200;br=0.80;wd=270;ws=6.5;t=-3.0;rh=88;p=1013;c=0.750;cb=50;pt=2;pi=120;wet=0.40;snow=1;snowroad=0";
        let r = read_of(w, None);
        assert!(r["as_custom"].is_null());
        assert_eq!(r["custom"]["visibility_m"], 1200.0);
        let back = write(&json!({ "custom": r["custom"] })).unwrap();
        assert_eq!(back["weather"], w);
        // (out of range made sound)
        let back = write(&json!({ "custom": { "visibility_m": 10.0, "precip": 7, "wind_dir": 365 } })).unwrap();
        assert_eq!((back["custom"]["visibility_m"].as_f64(), back["custom"]["precip"].as_i64(), back["custom"]["wind_dir"].as_f64()), (Some(50.0), Some(2), Some(5.0)));
        // (the cycle, a METAR report and a preset are not one's own)
        for w in ["cycle", "metar:EDDH", "Weather/Grey.owt", ""] {
            assert!(read_of(w, None)["custom"].is_null(), "{w}");
        }
        // ("custom:" alone is a fresh one)
        assert_eq!(CustomWeather::decode("Custom:").unwrap(), CustomWeather::default());
        assert_eq!(read_of("", None)["fresh"], CustomWeather::default().encode());
    }

    #[test]
    fn a_preset_made_ones_own() {
        let path = crate::content::tests::fixture().join("Weather/Snow.owt");
        let snow = hub_omsi::weather::Weather::load(&path).unwrap();
        let r = read_of("Weather/Snow.owt", Some(&snow));
        let own = r["as_custom"].as_str().unwrap();
        let c = CustomWeather::decode(own).unwrap();
        assert_eq!(c.precip, snow.precipitation.code());
        assert_eq!(c.snow_cover, snow.snow);
        assert!((c.visibility_m - snow.fog_m.clamp(50.0, 50_000.0)).abs() < 1.0);
        assert!((50.0..=100.0).contains(&humidity_of(0.0, -1.0)) && humidity_of(10.0, 10.0) > 99.9);
    }
}
