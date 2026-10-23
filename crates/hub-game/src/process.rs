//! The player's openOMSI started and watched: one game at a time, run from its own folder with
//! the flags of a [`LaunchSpec`], and its end told (how long it ran, its exit code) so the app
//! can read the plugin's reports then.

use crate::detect;
use crate::launch::{LaunchSpec, SpecError};
use crate::version::GameVersion;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// A game that was started.
#[derive(Debug)]
pub struct Game {
    child: Child,
    pub exe: PathBuf,
    started: Instant,
    /// Ended by the app ([`Game::stop`]): its end has no exit code of its own (Windows gives a
    /// killed program the code 1, which is no error of the game's).
    stopped: bool,
}

/// How a game ended.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Ended {
    /// The exit code (None: ended by a signal, or killed).
    pub code: Option<i32>,
    /// How long it ran.
    pub seconds: f64,
}

/// Why a game could not be started.
#[derive(Debug)]
pub enum StartError {
    Spec(SpecError),
    Io(std::io::Error),
    /// Windows' app control (Smart App Control, or an organisation's policy) refused the program.
    Blocked,
}

/// What the app says when Windows' app control refuses openOMSI (a text to translate).
pub const BLOCKED: &str = "Windows blocked openOMSI: Smart App Control (or your organisation's app control) does not let this unsigned program start. Smart App Control has no list of allowed programs: openOMSI only starts with it off (Windows Security, App & browser control, Smart App Control), and it cannot be switched on again without reinstalling Windows. Under your organisation's policy, ask your administrator.";

/// The system's codes for a program its app control refused: `ERROR_SYSTEM_INTEGRITY_POLICY_VIOLATION`
/// (Smart App Control, Windows Defender Application Control) and `ERROR_ACCESS_DISABLED_BY_POLICY`
/// (AppLocker, a software restriction policy).
const BLOCKED_CODES: [i32; 2] = [4551, 1260];

impl StartError {
    fn of_io(e: std::io::Error) -> Self {
        if cfg!(windows) && e.raw_os_error().is_some_and(|c| BLOCKED_CODES.contains(&c)) {
            StartError::Blocked
        } else {
            StartError::Io(e)
        }
    }
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::Spec(e) => write!(f, "{e}"),
            StartError::Io(e) => write!(f, "openOMSI could not be started: {e}"),
            StartError::Blocked => f.write_str(BLOCKED),
        }
    }
}

/// The command that starts `exe` with `spec`: in the program's folder (openOMSI finds its own
/// files beside it), no console window on Windows.
pub fn command(exe: &Path, spec: &LaunchSpec) -> Result<Command, SpecError> {
    let mut cmd = Command::new(exe);
    cmd.args(spec.args()?);
    if let Some(dir) = exe.parent().filter(|d| !d.as_os_str().is_empty()) {
        cmd.current_dir(dir);
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    no_window(&mut cmd);
    Ok(cmd)
}

#[cfg(windows)]
fn no_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn no_window(_: &mut Command) {}

/// The program at `exe` made executable where the system has the bit (Linux, macOS): a release
/// unpacked from its ZIP by many tools loses it. The bit is set for whoever may read the file.
#[cfg(unix)]
pub fn ensure_executable(exe: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(exe)?.permissions();
    let mode = perm.mode();
    if mode & 0o111 == 0 {
        perm.set_mode(mode | ((mode & 0o444) >> 2));
        std::fs::set_permissions(exe, perm)?;
    }
    Ok(())
}

/// (Windows has no executable bit.)
#[cfg(not(unix))]
pub fn ensure_executable(_exe: &Path) -> std::io::Result<()> {
    Ok(())
}

/// The mark Windows puts on a file that came from the internet (the `Zone.Identifier` stream a
/// browser or an unzipping tool writes) taken off openOMSI's program and the libraries beside it,
/// as "Unblock" in the file's properties does: with it, SmartScreen and Smart App Control judge
/// the unsigned program as a download and may refuse it. What will not go is left as it is.
#[cfg(windows)]
pub fn unblock(exe: &Path) {
    let mut files = vec![exe.to_path_buf()];
    if let Some(dir) = exe.parent() {
        if let Ok(rd) = std::fs::read_dir(dir) {
            files.extend(rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("dll") || x.eq_ignore_ascii_case("exe"))));
        }
    }
    for f in files {
        let mut mark = f.into_os_string();
        mark.push(":Zone.Identifier");
        let _ = std::fs::remove_file(mark);
    }
}

/// (Only Windows marks downloads so.)
#[cfg(not(windows))]
pub fn unblock(_exe: &Path) {}

/// openOMSI at `exe` started with `spec`.
pub fn start(exe: &Path, spec: &LaunchSpec) -> Result<Game, StartError> {
    // (the spec checked before the program is touched: a bad drive says so on every system)
    let cmd = command(exe, spec).map_err(StartError::Spec)?;
    ensure_executable(exe).map_err(StartError::Io)?;
    unblock(exe);
    Game::spawn(cmd, exe).map_err(StartError::of_io)
}

impl Game {
    /// Any program started as the game (the tests start a harmless one).
    pub fn spawn(mut cmd: Command, exe: &Path) -> std::io::Result<Game> {
        let child = cmd.spawn()?;
        Ok(Game { child, exe: exe.to_path_buf(), started: Instant::now(), stopped: false })
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Whether it ended, without waiting.
    pub fn poll(&mut self) -> Option<Ended> {
        match self.child.try_wait() {
            Ok(Some(status)) => Some(Ended { code: status.code().filter(|_| !self.stopped), seconds: self.started.elapsed().as_secs_f64() }),
            Ok(None) => None,
            Err(_) => Some(Ended { code: None, seconds: self.started.elapsed().as_secs_f64() }),
        }
    }

    /// Wait for its end.
    pub fn wait(mut self) -> Ended {
        let code = self.child.wait().ok().and_then(|s| s.code()).filter(|_| !self.stopped);
        Ended { code, seconds: self.started.elapsed().as_secs_f64() }
    }

    /// End it (the player asked to, or the app quits): no exit code then, as for a signal.
    pub fn stop(&mut self) -> Ended {
        // (one that ended by itself before keeps its own code)
        if matches!(self.child.try_wait(), Ok(None)) {
            self.stopped = true;
        }
        let _ = self.child.kill();
        let code = self.child.wait().ok().and_then(|s| s.code()).filter(|_| !self.stopped);
        Ended { code, seconds: self.started.elapsed().as_secs_f64() }
    }

    /// Watch it on a thread of its own; `on_end` is called once when it ended.
    pub fn watch(self, on_end: impl FnOnce(Ended) + Send + 'static) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || on_end(self.wait()))
    }
}

/// What `exe --version` prints, waiting at most `wait`: the last resort for a version no file
/// says (ported from the old add-on code). Starts the program, so the app asks only once per
/// program file (`crate::check_folder_probing` keeps the answer). openOMSI answers `--version`
/// from its command line's parser and ends before anything of the game starts.
pub fn ask_version(exe: &Path, wait: Duration) -> Option<GameVersion> {
    ensure_executable(exe).ok()?;
    unblock(exe);
    let mut cmd = Command::new(exe);
    cmd.arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    no_window(&mut cmd);
    let mut child = cmd.spawn().ok()?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < wait => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut out = String::new();
    std::io::Read::read_to_string(&mut child.stdout.take()?, &mut out).ok()?;
    detect::version_from_output(&out)
}

/// The bus of `spec` (its `export_glb` set) exported as glTF by the openOMSI at `exe`: the
/// program started without a window, waited for up to `wait`, and the file it wrote checked.
/// Fails with what the game said last when it wrote nothing (an openOMSI without
/// `--export-glb`, a bus it cannot read).
pub fn export_glb(exe: &Path, spec: &LaunchSpec, wait: Duration) -> Result<PathBuf, String> {
    let out = spec.export_glb.clone().ok_or("no file to export into")?;
    let args = spec.args().map_err(|e| e.to_string())?;
    ensure_executable(exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    if let Some(d) = out.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    let _ = std::fs::remove_file(&out);
    unblock(exe);
    let mut cmd = Command::new(exe);
    cmd.args(&args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    if let Some(dir) = exe.parent() {
        cmd.current_dir(dir);
    }
    no_window(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| match StartError::of_io(e) {
        StartError::Blocked => BLOCKED.to_string(),
        e => format!("{}: {e}", exe.display()),
    })?;
    // (what it says on the way is read beside it, so that a full pipe never stops it)
    let err = child.stderr.take().map(|mut e| {
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = std::io::Read::read_to_string(&mut e, &mut s);
            s
        })
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() < wait => std::thread::sleep(Duration::from_millis(100)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let said = err.and_then(|h| h.join().ok()).unwrap_or_default();
    let last = said.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
    let written = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    match status {
        None => Err(format!("openOMSI took longer than {} s to export the bus", wait.as_secs())),
        Some(_) if written > 12 => Ok(out),
        Some(s) => Err(if last.is_empty() { format!("openOMSI wrote no model ({s})") } else { format!("openOMSI wrote no model: {last}") }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake openOMSI that writes a model where `--export-glb` says, or says why not.
    #[cfg(unix)]
    fn fake_exporter(dir: &Path, writes: bool) -> PathBuf {
        let exe = dir.join(if writes { "openomsi" } else { "openomsi-old" });
        let body = if writes {
            "#!/bin/sh\nwhile [ $# -gt 0 ]; do if [ \"$1\" = --export-glb ]; then printf 'glTF-model-bytes' > \"$2\"; fi; shift; done\n"
        } else {
            "#!/bin/sh\necho 'error: unknown flag --export-glb' >&2\nexit 2\n"
        };
        std::fs::write(&exe, body).unwrap();
        exe
    }

    #[cfg(unix)]
    #[test]
    fn a_bus_is_exported_by_the_game_or_says_why_not() {
        let dir = std::env::temp_dir().join(format!("hub-game-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("models/a.glb");
        let spec = LaunchSpec { bus: Some("Vehicles/X/a.bus".into()), paint: Some("Red".into()), export_glb: Some(out.clone()), ..LaunchSpec::default() };
        let a = spec.args().unwrap();
        assert!(a.windows(2).any(|w| w == ["--export-glb", out.to_str().unwrap()]) && a.windows(2).any(|w| w == ["--paint", "Red"]), "{a:?}");
        // (the program unpacked without its executable bit gets it, as a drive's does)
        let exe = fake_exporter(&dir, true);
        assert_eq!(export_glb(&exe, &spec, Duration::from_secs(10)), Ok(out.clone()));
        assert_eq!(std::fs::read(&out).unwrap(), b"glTF-model-bytes");
        let old = fake_exporter(&dir, false);
        assert_eq!(export_glb(&old, &spec, Duration::from_secs(10)), Err("openOMSI wrote no model: error: unknown flag --export-glb".into()));
        assert!(!out.exists(), "the model of before is not taken for this one");
        assert!(export_glb(&exe, &LaunchSpec::default(), Duration::from_secs(1)).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A harmless program in the game's place: `rustc --version`, which every test run has.
    fn harmless() -> (Command, PathBuf) {
        let mut cmd = Command::new("rustc");
        cmd.arg("--version").stdout(Stdio::null());
        (cmd, PathBuf::from("rustc"))
    }

    #[test]
    fn a_started_program_is_watched_to_its_end() {
        let (cmd, exe) = harmless();
        let game = Game::spawn(cmd, &exe).unwrap();
        assert!(game.pid() > 0);
        let (tx, rx) = std::sync::mpsc::channel();
        game.watch(move |e| tx.send(e).unwrap()).join().unwrap();
        let ended = rx.recv().unwrap();
        assert_eq!(ended.code, Some(0));
        assert!(ended.seconds >= 0.0);
    }

    /// A game the app ends has no exit code of its own: not the error a killed program's code
    /// (1 on Windows) would make of it in the page (`gameend.js`).
    #[test]
    fn a_game_the_app_ended_says_no_exit_code() {
        // (`rustc -` reads its program from stdin: it waits while stdin stays open)
        let mut cmd = Command::new("rustc");
        cmd.arg("-").stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null());
        let mut game = Game::spawn(cmd, Path::new("rustc")).unwrap();
        assert!(game.poll().is_none(), "it runs");
        let ended = game.stop();
        assert_eq!(ended.code, None);
        assert_eq!(game.poll().map(|e| e.code), Some(None), "the bridge's loop hears it ended so too");
    }

    #[cfg(unix)]
    #[test]
    fn a_program_unpacked_without_its_executable_bit_gets_it() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("hub-game-exec-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("openomsi");
        std::fs::write(&exe, "#!/bin/sh\necho 'openomsi 0.2.31'\n").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o644)).unwrap();
        ensure_executable(&exe).unwrap();
        assert_eq!(std::fs::metadata(&exe).unwrap().permissions().mode() & 0o777, 0o755);
        // (and a script in openOMSI's place answers --version as the game does)
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(ask_version(&exe, Duration::from_secs(5)), Some(GameVersion::new(0, 2, 31)));
        assert_eq!(std::fs::metadata(&exe).unwrap().permissions().mode() & 0o777, 0o700);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_command_runs_in_the_program_s_folder_with_the_spec_s_flags() {
        let exe = Path::new("C:/Games/openOMSI/openomsi.exe");
        let spec = LaunchSpec { map: Some("maps/Grundorf/global.cfg".into()), ..LaunchSpec::default() };
        let cmd = command(exe, &spec).unwrap();
        assert_eq!(cmd.get_current_dir(), Some(Path::new("C:/Games/openOMSI")));
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(args, ["--map", "maps/Grundorf/global.cfg"]);
        assert!(matches!(start(exe, &LaunchSpec { time: Some("x".into()), ..spec }), Err(StartError::Spec(_))));
    }
}
