//! Omsi-Hub, the desktop app: a Tauri 2 window (stock Tauri) with the page in `ui/`, and the
//! commands the page calls.
//!
//! * `call {cmd, args}`, `call_bytes {cmd, args}` - hub-api's commands, run off the main
//!   thread;
//! * `pick_folder`, `pick_file` - the system's pickers; `open_url`, `open_path` - the system's
//!   browser and file manager;
//! * `selftest_report` - the page's selftests' report: printed, and the app ends with it.
//!
//! The `hub` protocol answers `hub://localhost/bytes/<cmd>?a=<json args>` (on Windows
//! `http://hub.localhost/bytes/…`) with a bytes command's answer, for `<img src>`.
//!
//! The updates (`hub_api::update`): hub-api looks at this repository's releases and offers one;
//! Tauri's updater plugin (`Updates`) downloads the release named in its `latest.json`, checks
//! its signature against `plugins.updater.pubkey` (tauri.conf.json), installs it and starts the
//! new version. Without a public key there, the plugin is not used and the offer links to the
//! release's page (docs/UPDATES.md).
//!
//! `OMSIHUB_SELFTEST=1` runs the page's selftests and ends with their report (exit code 0 when
//! all passed, 1 when one failed, 2 when the page gave none within a minute);
//! `OMSIHUB_SELFTEST_FIXTURES=<the repository's fixtures/>` adds the first-run setup, filled in
//! with the fake folders there, and the old saves of `fixtures/old-openomsi` offered and copied;
//! the data folder is then a temporary one unless `OMSIHUB_DATA_DIR` says another.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;

/// The environment variable of the selftest mode.
const SELFTEST_ENV: &str = "OMSIHUB_SELFTEST";
/// With it, the selftest goes through the first-run setup with the fake folders in this folder.
const FIXTURES_ENV: &str = "OMSIHUB_SELFTEST_FIXTURES";

/// hub-api's view of the app.
struct App(AppHandle);

impl hub_api::Platform for App {
    fn emit(&self, event: &str, payload: Value) {
        let _ = self.0.emit(event, payload);
    }
    fn quit(&self) {
        self.0.exit(0);
    }
    fn emitter(&self) -> Option<hub_api::Emitter> {
        let h = self.0.clone();
        Some(Arc::new(move |event: &str, payload: Value| {
            let _ = h.emit(event, payload);
        }))
    }
}

/// The updates' installation: Tauri's updater plugin, on the release `latest.json` names.
struct Updates(AppHandle);

impl hub_api::update::Installer for Updates {
    fn install(&self, version: &str, progress: &mut dyn FnMut(u64, Option<u64>)) -> Result<(), String> {
        tauri::async_runtime::block_on(async {
            let update = self.0.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;
            let Some(update) = update else { return Err(format!("{version} is not in the release's latest.json")) };
            hub_core::log_line(&format!("update: latest.json offers {} (the look found {version})", update.version));
            // (its signature checked against the public key before anything is installed; on
            // Windows the NSIS installer then ends this app and starts the new one)
            update.download_and_install(|chunk, total| progress(chunk as u64, total), || {}).await.map_err(|e| e.to_string())
        })
    }

    fn restart(&self) {
        self.0.restart();
    }
}

/// The updater's public key (tauri.conf.json `plugins.updater.pubkey`): empty until the signing
/// key is made.
fn updater_pubkey(app: &AppHandle) -> String {
    app.config().plugins.0.get("updater").and_then(|u| u.get("pubkey")).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

#[tauri::command]
async fn call(app: AppHandle, cmd: String, args: Option<Value>) -> Result<Value, String> {
    let args = args.unwrap_or(Value::Null);
    tauri::async_runtime::spawn_blocking(move || {
        let t0 = std::time::Instant::now();
        let answer = hub_api::call(&cmd, &args, &App(app));
        // (a command the page waited long for, in the log: where the app lags is read there)
        let ms = t0.elapsed().as_millis();
        if ms >= SLOW_MS {
            let op = args.get("op").and_then(Value::as_str).map(|o| format!(" {o}")).unwrap_or_default();
            hub_core::log::log_line(&format!("slow: {cmd}{op} took {ms} ms"));
        }
        answer
    })
    .await
    .map_err(|e| e.to_string())?
}

/// A command slower than this is written in the log (ms).
const SLOW_MS: u128 = 250;

#[tauri::command]
async fn call_bytes(app: AppHandle, cmd: String, args: Option<Value>) -> Result<tauri::ipc::Response, String> {
    let args = args.unwrap_or(Value::Null);
    let bytes = tauri::async_runtime::spawn_blocking(move || hub_api::call_bytes(&cmd, &args, &App(app))).await.map_err(|e| e.to_string())??;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
async fn pick_folder(app: AppHandle, title: Option<String>) -> Option<String> {
    let mut d = app.dialog().file();
    if let Some(t) = title.filter(|t| !t.is_empty()) {
        d = d.set_title(t);
    }
    d.blocking_pick_folder().map(|p| p.to_string())
}

#[derive(Deserialize)]
struct Filter {
    name: String,
    extensions: Vec<String>,
}

#[tauri::command]
async fn pick_file(app: AppHandle, title: Option<String>, filters: Option<Vec<Filter>>) -> Option<String> {
    let mut d = app.dialog().file();
    if let Some(t) = title.filter(|t| !t.is_empty()) {
        d = d.set_title(t);
    }
    for f in filters.unwrap_or_default() {
        let ext: Vec<&str> = f.extensions.iter().map(String::as_str).collect();
        d = d.add_filter(f.name, &ext);
    }
    d.blocking_pick_file().map(|p| p.to_string())
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    // (web pages only: never a file or a program by way of a URL)
    if !(url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:")) {
        return Err(format!("not a web address: {url}"));
    }
    // (from an AppImage on Linux: the browser started without the AppImage's own libraries)
    #[cfg(target_os = "linux")]
    if let Some(dir) = std::env::var_os("APPDIR").filter(|_| std::env::var_os("APPIMAGE").is_some()) {
        return open_outside_appimage(&url, std::path::Path::new(&dir));
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// The variables an AppImage's start script points into the AppImage (its GTK, GIO, GStreamer and
/// libraries): `xdg-open` and the browser it starts would load those instead of the system's and
/// may not start at all. Their entries inside the AppImage are taken out for the browser.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const APPIMAGE_VARS: &[&str] = &[
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "PATH",
    "XDG_DATA_DIRS",
    "XDG_CONFIG_DIRS",
    "GDK_PIXBUF_MODULE_FILE",
    "GDK_PIXBUF_MODULEDIR",
    "GIO_MODULE_DIR",
    "GIO_EXTRA_MODULES",
    "GSETTINGS_SCHEMA_DIR",
    "GTK_PATH",
    "GTK_EXE_PREFIX",
    "GTK_DATA_PREFIX",
    "GTK_IM_MODULE_FILE",
    "GST_PLUGIN_PATH",
    "GST_PLUGIN_PATH_1_0",
    "GST_PLUGIN_SYSTEM_PATH",
    "GST_PLUGIN_SYSTEM_PATH_1_0",
    "GST_PLUGIN_SCANNER",
    "GST_PLUGIN_SCANNER_1_0",
    "PYTHONHOME",
    "PYTHONPATH",
    "PERLLIB",
    "QT_PLUGIN_PATH",
];

/// A path list (or one path) without its entries inside `appdir`; None when nothing is left.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn without_appdir(value: &std::ffi::OsStr, appdir: &std::path::Path) -> Option<std::ffi::OsString> {
    let kept: Vec<std::path::PathBuf> = std::env::split_paths(value).filter(|p| !p.as_os_str().is_empty() && !p.starts_with(appdir)).collect();
    if kept.is_empty() {
        None
    } else {
        std::env::join_paths(kept).ok()
    }
}

/// `url` in the system's browser from inside an AppImage: `xdg-open` (or `gio open`) with the
/// AppImage's own entries taken out of `APPIMAGE_VARS`.
#[cfg(target_os = "linux")]
fn open_outside_appimage(url: &str, appdir: &std::path::Path) -> Result<(), String> {
    use std::process::{Command, Stdio};
    let mut last = String::new();
    for (program, args) in [("xdg-open", vec![url]), ("gio", vec!["open", url])] {
        let mut c = Command::new(program);
        c.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        for k in APPIMAGE_VARS {
            if let Some(v) = std::env::var_os(k) {
                match without_appdir(&v, appdir) {
                    Some(v) => c.env(k, v),
                    None => c.env_remove(k),
                };
            }
        }
        match c.spawn() {
            Ok(mut child) => {
                // (reaped when it ends: the browser runs on by itself)
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                return Ok(());
            }
            Err(e) => last = format!("{program}: {e}"),
        }
    }
    Err(last)
}

#[tauri::command]
fn open_path(app: AppHandle, path: String) -> Result<(), String> {
    if !std::path::Path::new(&path).exists() {
        return Err(format!("{path} is not there"));
    }
    app.opener().open_path(path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn selftest_report(app: AppHandle, report: Value) {
    println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
    hub_core::log_line(&format!("selftest: {}", if report["ok"] == true { "passed" } else { "failed" }));
    app.exit(if report["ok"] == true { 0 } else { 1 });
}

/// `%xx` decoded.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            if let Some(v) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A bytes command of the `hub` protocol's path and query: (cmd, args).
fn bytes_request(path: &str, query: Option<&str>) -> Option<(String, Value)> {
    let cmd = percent_decode(path.trim_start_matches('/').strip_prefix("bytes/")?);
    let args = query.and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("a="))).map(percent_decode).and_then(|a| serde_json::from_str(&a).ok()).unwrap_or(Value::Null);
    Some((cmd, args))
}

/// The media type of some bytes, from their first ones.
fn sniff(b: &[u8]) -> &'static str {
    if b.starts_with(b"\x89PNG") {
        "image/png"
    } else if b.starts_with(b"\xFF\xD8") {
        "image/jpeg"
    } else if b.starts_with(b"<svg") || b.starts_with(b"<?xml") {
        "image/svg+xml"
    } else if b.starts_with(b"RIFF") && b.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

fn main() {
    let selftest = std::env::var_os(SELFTEST_ENV).is_some_and(|v| !v.is_empty() && v != "0");
    // (the selftest on the fixtures: a data folder of its own unless one is given, and the old
    // Omsi-Hub's saves from the fixture of an old ~/.openomsi, so the setup offers them)
    if selftest {
        if let Some(dir) = std::env::var_os(FIXTURES_ENV).map(std::path::PathBuf::from) {
            if std::env::var_os(hub_core::paths::DATA_DIR_ENV).is_none() {
                let data = std::env::temp_dir().join(format!("omsihub-selftest-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&data);
                std::env::set_var(hub_core::paths::DATA_DIR_ENV, data);
            }
            if std::env::var_os(hub_core::migrate::OLD_FOLDER_ENV).is_none() && dir.join("old-openomsi").is_dir() {
                std::env::set_var(hub_core::migrate::OLD_FOLDER_ENV, dir.join("old-openomsi"));
            }
        }
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .register_asynchronous_uri_scheme_protocol("hub", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let uri = request.uri().clone();
            std::thread::spawn(move || {
                let answer = match bytes_request(uri.path(), uri.query()) {
                    Some((cmd, args)) => hub_api::call_bytes(&cmd, &args, &App(app)),
                    None => Err("not found".into()),
                };
                let response = match answer {
                    Ok(bytes) => tauri::http::Response::builder().status(200).header("Content-Type", sniff(&bytes)).header("Access-Control-Allow-Origin", "*").body(bytes),
                    Err(e) => tauri::http::Response::builder().status(404).header("Content-Type", "text/plain; charset=utf-8").body(e.into_bytes()),
                };
                responder.respond(response.unwrap_or_default());
            });
        })
        .invoke_handler(tauri::generate_handler![call, call_bytes, pick_folder, pick_file, open_url, open_path, selftest_report])
        .setup(move |app| {
            let mut w = tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into())).title("Omsi-Hub").inner_size(1280.0, 820.0).min_inner_size(1024.0, 680.0).center();
            if selftest {
                w = w.initialization_script("window.__OMSIHUB_SELFTEST__ = true;");
                // (the first-run setup tried on fake folders: `fixtures/` of the repository)
                if let Some(dir) = std::env::var_os(FIXTURES_ENV).map(std::path::PathBuf::from) {
                    let path = |n: &str| dir.join(n).to_string_lossy().into_owned();
                    let fx = serde_json::json!({ "openomsi": path("openOMSI-0.2.30-windows-x64"), "old": path("openOMSI-0.2.10-windows-x64"), "omsi2": path("OMSI 2") });
                    w = w.initialization_script(format!("window.__OMSIHUB_FIXTURES__ = {fx};"));
                }
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    eprintln!("selftest: the page gave no report");
                    handle.exit(2);
                });
            }
            w.build()?;
            // (a build without the signing key's public half installs nothing itself)
            if !updater_pubkey(app.handle()).is_empty() {
                hub_api::update::set_installer(Arc::new(Updates(app.handle().clone())));
            }
            hub_core::log_line(&format!("app {} started", env!("CARGO_PKG_VERSION")));
            let _ = app.get_webview_window("main");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("the app could not start");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bytes_route_reads_its_command_and_arguments() {
        let (cmd, args) = bytes_request("/bytes/co_portrait", Some("a=%7B%22id%22%3A%22x%20y%22%7D")).unwrap();
        assert_eq!(cmd, "co_portrait");
        assert_eq!(args["id"], "x y");
        assert!(bytes_request("/other/x", None).is_none());
        assert_eq!(bytes_request("/bytes/x", None).unwrap().1, Value::Null);
        assert_eq!(sniff(b"\x89PNG\r\n"), "image/png");
        assert_eq!(percent_decode("a%2Fb%zz"), "a/b%zz");
    }

    /// From an AppImage, the browser is started without the AppImage's own paths.
    #[test]
    fn the_appimages_own_paths_are_taken_out() {
        use std::path::PathBuf;
        let appdir = std::env::temp_dir().join("Omsi-Hub.AppDir");
        let (inside, outside) = (appdir.join("usr").join("lib"), std::env::temp_dir().join("system-lib"));
        let both = std::env::join_paths([inside.clone(), outside.clone()]).unwrap();
        let kept: Vec<PathBuf> = std::env::split_paths(&without_appdir(&both, &appdir).unwrap()).collect();
        assert_eq!(kept, [outside]);
        assert_eq!(without_appdir(inside.as_os_str(), &appdir), None, "only the AppImage's: the variable goes");
        assert!(APPIMAGE_VARS.contains(&"LD_LIBRARY_PATH") && APPIMAGE_VARS.contains(&"XDG_DATA_DIRS"));
    }
}
