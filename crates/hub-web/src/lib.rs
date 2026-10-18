//! The bus company and RosterFlow in the browser: the website's page (omsihub.com, `company`)
//! loads this module, built for `wasm32-wasip1`, lays the cloud save's files out in the folders
//! WASI gives it (`/data`, the data folder; `/omsi`, the OMSI 2 content the desktop app sent with
//! the save) and asks the commands as the desktop's page asks them (`ui/js/api.js`): the same
//! `hub_api::call`, without the parts that need the computer (`hub-api` without "native").
//!
//! The page writes a command and its arguments into memory it asked for ([`hub_alloc`]) and calls
//! [`hub_call`] (or [`hub_call_bytes`]); the answer waits in [`hub_out`] / [`hub_out_len`] until
//! the next call. A JSON answer is `{"ok": …}` or `{"err": "…"}`; bytes come with a first byte,
//! 1 for the bytes, 0 for an error's words. What the commands tell the page meanwhile
//! (`Platform::emit`) is kept for [`hub_events`].

#[cfg(target_arch = "wasm32")]
mod web {
    use serde_json::{json, Value};
    use std::sync::Mutex;

    static OUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    static EVENTS: Mutex<Vec<Value>> = Mutex::new(Vec::new());

    struct Page;

    impl hub_api::Platform for Page {
        fn emit(&self, event: &str, payload: Value) {
            EVENTS.lock().unwrap_or_else(|e| e.into_inner()).push(json!({ "event": event, "payload": payload }));
        }
    }

    fn put(bytes: Vec<u8>) -> *const u8 {
        let mut out = OUT.lock().unwrap_or_else(|e| e.into_inner());
        *out = bytes;
        out.as_ptr()
    }

    /// The text at `ptr` (`len` bytes, written by the page).
    unsafe fn text(ptr: *const u8, len: usize) -> String {
        if len == 0 {
            return String::new();
        }
        String::from_utf8_lossy(std::slice::from_raw_parts(ptr, len)).into_owned()
    }

    unsafe fn args_of(ptr: *const u8, len: usize) -> Value {
        serde_json::from_str(&text(ptr, len)).unwrap_or(Value::Null)
    }

    /// `len` bytes for the page to write into (given back with [`hub_free`]).
    #[no_mangle]
    pub extern "C" fn hub_alloc(len: usize) -> *mut u8 {
        let mut v = Vec::<u8>::with_capacity(len.max(1));
        let p = v.as_mut_ptr();
        std::mem::forget(v);
        p
    }

    #[no_mangle]
    pub unsafe extern "C" fn hub_free(ptr: *mut u8, len: usize) {
        drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
    }

    /// A command (`cmd`, its arguments as JSON): its answer, then the work it left for later.
    #[no_mangle]
    pub unsafe extern "C" fn hub_call(cmd_ptr: *const u8, cmd_len: usize, args_ptr: *const u8, args_len: usize) -> *const u8 {
        let (cmd, args) = (text(cmd_ptr, cmd_len), args_of(args_ptr, args_len));
        let answer = hub_api::call(&cmd, &args, &Page);
        hub_api::run_later();
        let v = match answer {
            Ok(v) => json!({ "ok": v }),
            Err(e) => json!({ "err": e }),
        };
        put(serde_json::to_vec(&v).unwrap_or_default())
    }

    /// A command that answers bytes (a picture, a printout).
    #[no_mangle]
    pub unsafe extern "C" fn hub_call_bytes(cmd_ptr: *const u8, cmd_len: usize, args_ptr: *const u8, args_len: usize) -> *const u8 {
        let (cmd, args) = (text(cmd_ptr, cmd_len), args_of(args_ptr, args_len));
        let answer = hub_api::call_bytes(&cmd, &args, &Page);
        hub_api::run_later();
        put(match answer {
            Ok(mut b) => {
                b.insert(0, 1);
                b
            }
            Err(e) => [&[0u8][..], e.as_bytes()].concat(),
        })
    }

    /// What the commands told the page since it last asked, as a JSON list.
    #[no_mangle]
    pub extern "C" fn hub_events() -> *const u8 {
        let list: Vec<Value> = std::mem::take(&mut *EVENTS.lock().unwrap_or_else(|e| e.into_inner()));
        put(serde_json::to_vec(&list).unwrap_or_default())
    }

    /// A cloud save (`v<n>.omsave`: zstd around the archive, `hub_core::company::cloud`) laid
    /// out in the data folder as the desktop lays one out (`cloud::install`), under its own id, with
    /// the settings the page runs under: the OMSI 2 content the desktop sent with it in `/omsi`,
    /// the driver whose company it is, the page's language (`args`: `{ "language": "nl" }`).
    /// Answers `{ok: {id, name, profile}}`.
    #[no_mangle]
    pub unsafe extern "C" fn hub_install(ptr: *const u8, len: usize, args_ptr: *const u8, args_len: usize) -> *const u8 {
        let bytes = if len == 0 { &[][..] } else { std::slice::from_raw_parts(ptr, len) };
        let args = args_of(args_ptr, args_len);
        let v = match install(bytes, &args) {
            Ok(v) => json!({ "ok": v }),
            Err(e) => json!({ "err": e }),
        };
        put(serde_json::to_vec(&v).unwrap_or_default())
    }

    fn install(bytes: &[u8], args: &Value) -> Result<Value, String> {
        use hub_api::launcher::company::cloud;
        use std::io::Read;
        let mut archive = Vec::new();
        let d = ruzstd::decoding::StreamingDecoder::new(bytes).map_err(|e| format!("bad_save: {e}"))?;
        d.take(MAX_UNPACKED + 1).read_to_end(&mut archive).map_err(|e| format!("bad_save: {e}"))?;
        if archive.len() as u64 > MAX_UNPACKED {
            return Err("bad_save: too large".into());
        }
        let id = cloud::unpack(&archive).map_err(|e| format!("{e:#}"))?.company.id;
        let data = hub_api::launcher::paths::data_dir();
        let c = cloud::install(&data, &id, &archive, "", None).map_err(|e| format!("{e:#}"))?;
        let mut st = hub_api::launcher::settings::Settings::load();
        st.omsi2_folder = "/omsi".into();
        st.setup_done = true;
        st.migrated = true;
        if let Some(l) = args.get("language").and_then(Value::as_str) {
            st.language = l.to_string();
        }
        st.other.insert("driver".into(), json!(c.profile));
        st.save().map_err(|e| e.to_string())?;
        Ok(json!({ "id": c.id, "name": c.name, "profile": c.profile }))
    }

    /// A part of the content the desktop sent beside the save (`hub_core::webpack`, zstd'd: the
    /// map's timetable, the buses' files, the data folder's own) laid out: `omsi/…` under the OMSI 2
    /// folder, `data/…` under the data folder. Answers `{ok: {files}}`.
    #[no_mangle]
    pub unsafe extern "C" fn hub_content(ptr: *const u8, len: usize) -> *const u8 {
        let bytes = if len == 0 { &[][..] } else { std::slice::from_raw_parts(ptr, len) };
        let v = match content(bytes) {
            Ok(n) => json!({ "ok": { "files": n } }),
            Err(e) => json!({ "err": e }),
        };
        put(serde_json::to_vec(&v).unwrap_or_default())
    }

    fn content(bytes: &[u8]) -> Result<usize, String> {
        use std::io::Read;
        let mut archive = Vec::new();
        let d = ruzstd::decoding::StreamingDecoder::new(bytes).map_err(|e| format!("bad_content: {e}"))?;
        d.take(MAX_CONTENT + 1).read_to_end(&mut archive).map_err(|e| format!("bad_content: {e}"))?;
        if archive.len() as u64 > MAX_CONTENT {
            return Err("bad_content: too large".into());
        }
        let files = hub_api::launcher::webpack::unpack(&archive).map_err(|e| format!("{e:#}"))?;
        let n = files.len();
        for (name, bytes) in files {
            let path = std::path::Path::new("/").join(&name);
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| format!("{name}: {e}"))?;
            }
            std::fs::write(&path, bytes).map_err(|e| format!("{name}: {e}"))?;
        }
        Ok(n)
    }

    /// The most a part of the content may unpack to.
    const MAX_CONTENT: u64 = 1 << 30;

    /// The most a save may unpack to (the desktop's `MAX_UNPACKED`).
    const MAX_UNPACKED: u64 = 512 << 20;

    /// Company `id` packed as a cloud save again, as the desktop packs it (`cloud::pack`, then
    /// zstd): the bytes; their sha256 (hex) in [`hub_sha256`].
    #[no_mangle]
    pub unsafe extern "C" fn hub_pack(id_ptr: *const u8, id_len: usize) -> *const u8 {
        use sha2::{Digest, Sha256};
        let id = text(id_ptr, id_len);
        let data = hub_api::launcher::paths::data_dir();
        match hub_api::launcher::company::cloud::pack(&data, &id) {
            Ok(archive) => {
                let z = ruzstd::encoding::compress_to_vec(&archive[..], ruzstd::encoding::CompressionLevel::Fastest);
                *SHA.lock().unwrap_or_else(|e| e.into_inner()) = Sha256::digest(&z).iter().map(|b| format!("{b:02x}")).collect();
                put([&[1u8][..], &z].concat())
            }
            Err(e) => put([&[0u8][..], format!("{e:#}").as_bytes()].concat()),
        }
    }

    static SHA: Mutex<String> = Mutex::new(String::new());

    /// The sha256 of the save [`hub_pack`] made last, as text.
    #[no_mangle]
    pub extern "C" fn hub_sha256() -> *const u8 {
        let s = SHA.lock().unwrap_or_else(|e| e.into_inner()).clone();
        put(s.into_bytes())
    }

    /// The last answer: where it is, and how long.
    #[no_mangle]
    pub extern "C" fn hub_out() -> *const u8 {
        OUT.lock().unwrap_or_else(|e| e.into_inner()).as_ptr()
    }

    #[no_mangle]
    pub extern "C" fn hub_out_len() -> usize {
        OUT.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
}
