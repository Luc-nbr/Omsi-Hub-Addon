//! The companion server beside the real game, without the launcher: this version of the plugin
//! put into an openOMSI's `Plugins` folder and the add-on's companion server (`hub-companion`)
//! started for it, the bus's devices read from the OMSI 2 folder. For proving the plugin and the
//! devices' forms in the real openOMSI (docs/COMPANION-HANDOFF.md); the map's route and roads are
//! left out. Start openOMSI after it, pair the app (or `curl`) with the address and the code this
//! prints.
//!
//! ```text
//! cargo run -p hub-plugin-harness --example realgame -- "<openOMSI folder>" "<OMSI 2 folder>"
//! ```

use hub_companion::{Companion, Config, Source};
use hub_game::bridge::Assignment;
use hub_game::install;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

struct Game {
    plugin: PathBuf,
    roots: Vec<PathBuf>,
}

/// `file` (as the game names it, relative to its folder) in the first of `roots` that has it.
fn resolve(roots: &[PathBuf], file: &str) -> Option<(PathBuf, PathBuf)> {
    roots.iter().find_map(|r| {
        let p = file.split(['\\', '/']).filter(|s| !s.is_empty()).fold(r.clone(), |d, s| d.join(s));
        p.is_file().then(|| (p, r.clone()))
    })
}

impl Source for Game {
    fn plugin_dir(&self) -> Option<PathBuf> {
        Some(self.plugin.clone())
    }
    fn assignment(&self) -> Option<Assignment> {
        None
    }
    fn route(&self, _map: &str, _date: &str, _trip: &str) -> Option<Vec<[f64; 2]>> {
        None
    }
    fn roads(&self, _map: &str, _date: &str, _at: [f64; 2], _r: f64) -> Vec<(f64, Vec<[f64; 2]>)> {
        Vec::new()
    }
    fn world_of(&self, _map: &str, _tile: [i64; 2], _local: [f64; 2]) -> Option<[f64; 2]> {
        None
    }
    fn bus_model(&self, bus_file: &str) -> Option<(PathBuf, PathBuf)> {
        let (bus, root) = resolve(&self.roots, bus_file)?;
        Some((hub_companion::devices::model_of_bus(&bus)?, root))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(openomsi), Some(omsi)) = (args.first(), args.get(1)) else {
        eprintln!("usage: realgame <openOMSI folder> <OMSI 2 folder>");
        std::process::exit(2);
    };
    let plugins = Path::new(openomsi).join("Plugins");
    let status = install::install(&plugins).expect("the plugin installed");
    println!("plugin {:?} in {}", status.version, status.dir.display());
    let plugin = install::plugin_dir(&plugins);
    let roots = vec![PathBuf::from(omsi), PathBuf::from(openomsi)];

    let devices = std::env::temp_dir().join("hub-realgame-devices.json");
    let cfg = Config { port: hub_companion::DEFAULT_PORT, udp_port: hub_companion::UDP_PORT, name: "Real game".into(), version: env!("CARGO_PKG_VERSION").into(), devices_file: devices };
    let companion = Companion::start(cfg, Arc::new(Game { plugin, roots })).expect("the companion's ports are free");
    let status = companion.status();
    println!("companion on port {}, pairing code {}", companion.port(), status["code"].as_str().unwrap_or("?"));
    println!("{}", status["pair_url"].as_str().unwrap_or(""));
    let mut heard = None;
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let s = companion.status();
        let now = s.get("game").cloned();
        if now != heard {
            println!("game: {}", now.as_ref().map(|v| v.to_string()).unwrap_or_default());
            heard = now;
        }
    }
}
