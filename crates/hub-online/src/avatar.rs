//! The profile photo, the same in the app and on the website: one small square picture in the
//! public bucket `avatars`, in a folder only its owner knows and may write to (`my_avatar_folder()`,
//! not the user id). `profiles.avatar` names the one shown; the leaderboard and the friends carry
//! it, and its address is `/storage/v1/object/public/avatars/<name>` for anyone
//! (the website's migration `…_avatars.sql`).
//!
//! A new photo goes under a new name (never over the old one: no cache shows the old picture), is
//! made the photo with `set_avatar`, and the one it replaces is deleted. The picture comes made
//! small already (the page draws it at 256 px); what is sent is checked to be a JPEG, PNG or WebP
//! and at most `MAX_BYTES`.

use crate::client::Client;
use crate::codes::{self, Error};
use crate::http::Body;
use serde_json::{json, Value};

pub(crate) const BUCKET: &str = "avatars";
/// The bucket's limit.
pub const MAX_BYTES: usize = 1 << 20;

/// The picture's content type and its file's ending, from its first bytes (None: not one of ours).
pub fn picture_kind(b: &[u8]) -> Option<(&'static str, &'static str)> {
    if b.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(("image/jpeg", "jpg"))
    } else if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("image/png", "png"))
    } else if b.len() > 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some(("image/webp", "webp"))
    } else {
        None
    }
}

/// A folder as the server names one (a uuid), else nothing to write in.
fn folder_ok(f: &str) -> bool {
    f.len() == 36 && f.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-')
}

impl crate::Endpoints {
    /// The photo `name`'s address (public: an `<img>` loads it as it is).
    pub fn avatar_url(&self, name: &str) -> String {
        self.storage(&format!("/object/public/{BUCKET}/{name}"))
    }
}

impl Client {
    /// The photo `name`'s address.
    pub fn avatar_url(&self, name: &str) -> String {
        self.ep.avatar_url(name)
    }

    /// The player's photo folder; `no_profile` before the display name is chosen.
    fn avatar_folder(&mut self) -> Result<String, Error> {
        let f = codes::word_of(&self.rpc("my_avatar_folder", json!({}))?).unwrap_or_default();
        match f.as_str() {
            "no_profile" => Err(Error::Code("no_profile")),
            f if folder_ok(f) => Ok(f.to_string()),
            _ => Err(Error::Code("server")),
        }
    }

    /// Pictures of the bucket deleted (`DELETE /object/avatars`, `{prefixes}`).
    fn avatars_remove(&mut self, names: &[String]) -> Result<(), Error> {
        if names.is_empty() {
            return Ok(());
        }
        let r = self.authed("DELETE", &self.ep.storage(&format!("/object/{BUCKET}")), Body::Json(&json!({ "prefixes": names })), &[], false)?;
        if r.ok() {
            Ok(())
        } else {
            Err(codes::of_storage(r.status, &r.body))
        }
    }

    /// The picture `bytes` (a JPEG, PNG or WebP of at most `MAX_BYTES`) is the player's photo now;
    /// its name (for `avatar_url`).
    pub fn set_avatar(&mut self, bytes: &[u8]) -> Result<String, Error> {
        let (kind, ext) = picture_kind(bytes).ok_or(Error::Code("bad_photo"))?;
        if bytes.len() > MAX_BYTES {
            return Err(Error::Code("photo_too_large"));
        }
        let folder = self.avatar_folder()?;
        let name = format!("{folder}/{}.{ext}", crate::oauth::random(12).map_err(|_| Error::Code("server"))?);
        let url = self.ep.storage(&format!("/object/{BUCKET}/{name}"));
        // (a name of its own: the picture never changes, so it may be kept for long)
        let r = self.authed("POST", &url, Body::Bytes(bytes, kind), &[("cache-control", "max-age=31536000")], false)?;
        if !r.ok() {
            return Err(Error::Code(match codes::storage_status(r.status, &r.body) {
                413 => "photo_too_large",
                415 | 422 => "bad_photo",
                429 => "too_many",
                _ => "upload_failed",
            }));
        }
        let said = codes::word_of(&self.rpc("set_avatar", json!({ "p_name": name }))?).unwrap_or_default();
        match said.as_str() {
            "ok" => {}
            s if s.starts_with("ok:") => {
                // (the old one is gone from the profile; one that will not go is only a file left)
                let _ = self.avatars_remove(&[s[3..].to_string()]);
            }
            s => {
                let _ = self.avatars_remove(std::slice::from_ref(&name));
                return Err(Error::Code(if s == "no_profile" { "no_profile" } else { "bad_photo" }));
            }
        }
        Ok(name)
    }

    /// No photo any more (the picture deleted too).
    pub fn clear_avatar(&mut self) -> Result<(), Error> {
        let old = codes::word_of(&self.rpc("clear_avatar", json!({}))?).unwrap_or_default();
        if !old.is_empty() {
            let _ = self.avatars_remove(&[old]);
        }
        Ok(())
    }

    /// Every picture in the player's folder deleted (before the account is). Nothing to do without a
    /// profile, or on a server without photos yet.
    pub(crate) fn avatars_delete_all(&mut self) -> Result<(), Error> {
        let folder = match self.avatar_folder() {
            Ok(f) => f,
            Err(Error::Code("no_profile" | "unavailable")) => return Ok(()),
            Err(e) => return Err(e),
        };
        let r = self.authed("POST", &self.ep.storage(&format!("/object/list/{BUCKET}")), Body::Json(&json!({ "prefix": folder, "limit": 1000, "offset": 0 })), &[], false)?;
        if !r.ok() {
            return Err(codes::of_storage(r.status, &r.body));
        }
        let files: Vec<String> = r
            .body
            .as_array()
            .map(|a| a.iter().filter(|f| !f.get("id").unwrap_or(&Value::Null).is_null()).filter_map(|f| f.get("name").and_then(Value::as_str)).map(|n| format!("{folder}/{n}")).collect())
            .unwrap_or_default();
        self.avatars_remove(&files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{Answer, Mock};
    use crate::tests::{client, ok, refresh_route, signed_in};
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    const FOLDER: &str = "0f0e0d0c-0b0a-0908-0706-050403020100";
    const WEBP: &[u8] = b"RIFF\x10\0\0\0WEBPVP8 rest";

    /// The bucket's files and the profile's `avatar`.
    type State = Arc<Mutex<(BTreeMap<String, Vec<u8>>, Option<String>)>>;

    /// The project as far as photos go: the folder, the bucket and the profile's `avatar`.
    fn server() -> (Mock, State) {
        let state: State = Default::default();
        let s = state.clone();
        let m = Mock::start(move |r| -> Answer {
            if let Some(a) = refresh_route(r) {
                return a;
            }
            let mut st = s.lock().unwrap();
            let prefix = format!("/storage/v1/object/{BUCKET}/");
            match (r.method.as_str(), r.path.as_str()) {
                ("POST", "/rest/v1/rpc/my_avatar_folder") => ok(json!(FOLDER)),
                ("POST", "/rest/v1/rpc/set_avatar") => {
                    let name = r.body["p_name"].as_str().unwrap().to_string();
                    if !st.0.contains_key(&name) {
                        return ok(json!("bad"));
                    }
                    let old = st.1.replace(name.clone());
                    ok(json!(match old {
                        Some(o) if o != name => format!("ok:{o}"),
                        _ => "ok".into(),
                    }))
                }
                ("POST", "/rest/v1/rpc/clear_avatar") => ok(json!(st.1.take().unwrap_or_default())),
                ("POST", "/storage/v1/object/list/avatars") => {
                    let p = format!("{}/", r.body["prefix"].as_str().unwrap());
                    ok(json!(st.0.keys().filter_map(|k| k.strip_prefix(&p)).map(|n| json!({ "name": n, "id": n })).collect::<Vec<_>>()))
                }
                ("DELETE", "/storage/v1/object/avatars") => {
                    for p in r.body["prefixes"].as_array().unwrap() {
                        st.0.remove(p.as_str().unwrap());
                    }
                    ok(json!([]))
                }
                ("POST", p) if p.starts_with(&prefix) => {
                    st.0.insert(p[prefix.len()..].to_string(), r.raw.clone());
                    ok(json!({ "Key": p }))
                }
                _ => (404, vec![], Value::Null),
            }
        });
        (m, state)
    }

    #[test]
    fn a_picture_is_known_by_its_first_bytes() {
        assert_eq!(picture_kind(b"\xff\xd8\xff\xe0rest"), Some(("image/jpeg", "jpg")));
        assert_eq!(picture_kind(b"\x89PNG\r\n\x1a\nrest"), Some(("image/png", "png")));
        assert_eq!(picture_kind(WEBP), Some(("image/webp", "webp")));
        assert_eq!(picture_kind(b"GIF89a"), None);
        assert_eq!(picture_kind(b"<svg"), None);
    }

    #[test]
    fn a_new_photo_goes_in_the_folder_under_a_new_name_and_the_old_one_is_deleted() {
        let (m, state) = server();
        let mut c = client(&m, &signed_in("r1"));
        let first = c.set_avatar(WEBP).unwrap();
        assert!(first.starts_with(&format!("{FOLDER}/")) && first.ends_with(".webp"), "{first}");
        let put = m.requests().into_iter().find(|r| r.method == "POST" && r.path.starts_with("/storage/v1/object/avatars/")).unwrap();
        assert_eq!(put.header("content-type"), Some("image/webp"));
        assert_eq!(put.raw, WEBP);
        let second = c.set_avatar(b"\xff\xd8\xff\xe0jpeg").unwrap();
        assert_ne!(first, second);
        assert!(second.ends_with(".jpg"));
        assert_eq!(state.lock().unwrap().0.keys().cloned().collect::<Vec<_>>(), vec![second.clone()], "the old picture deleted");
        assert_eq!(state.lock().unwrap().1.as_deref(), Some(second.as_str()));
        assert_eq!(c.avatar_url(&second), format!("{}/storage/v1/object/public/avatars/{second}", m.base));
        c.clear_avatar().unwrap();
        assert!(state.lock().unwrap().0.is_empty() && state.lock().unwrap().1.is_none());
    }

    #[test]
    fn what_is_no_picture_or_too_large_is_not_sent() {
        let (m, _) = server();
        let mut c = client(&m, &signed_in("r1"));
        assert_eq!(c.set_avatar(b"<svg onload=x>"), Err(Error::Code("bad_photo")));
        let mut big = WEBP.to_vec();
        big.resize(MAX_BYTES + 1, 0);
        assert_eq!(c.set_avatar(&big), Err(Error::Code("photo_too_large")));
        assert!(m.requests().iter().all(|r| !r.path.contains("avatar")));
    }

    #[test]
    fn deleting_every_picture_empties_the_folder() {
        let (m, state) = server();
        let mut c = client(&m, &signed_in("r1"));
        state.lock().unwrap().0.insert(format!("{FOLDER}/a.webp"), vec![1]);
        state.lock().unwrap().0.insert(format!("{FOLDER}/b.jpg"), vec![2]);
        c.avatars_delete_all().unwrap();
        assert!(state.lock().unwrap().0.is_empty());
    }
}
