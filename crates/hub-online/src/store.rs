//! Where the session token is kept: the OS's credential store (the `keyring` crate), never a
//! file, a log or the company's save:
//!
//! * Windows: the Credential Manager;
//! * macOS: the Keychain;
//! * Linux: the Secret Service (GNOME Keyring, KWallet, KeePassXC …, over D-Bus).
//!
//! When the store is not there or refuses (a Linux desktop without a keyring service, a locked
//! keyring the player does not unlock, no D-Bus session), the token is kept in memory instead:
//! the player stays signed in until Omsi-Hub closes, and `lasting()` says so, so the page can tell
//! them. Elsewhere (Android, iOS) there is no store at all and it is memory from the start.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// The entry's names in the OS's store.
pub const SERVICE: &str = "Omsi-Hub online";
pub const ACCOUNT: &str = "session";

/// A place for the one secret.
pub trait SecretStore: Send {
    fn load(&self) -> Option<String>;
    /// Whether it was kept.
    fn save(&self, secret: &str) -> bool;
    fn clear(&self);
    /// Whether what it keeps outlives the program.
    fn lasting(&self) -> bool;
}

/// In memory only (the tests', and where the OS has no store).
#[derive(Default)]
pub struct MemoryStore(Mutex<Option<String>>);

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// One that holds `secret` already.
    pub fn with(secret: &str) -> Self {
        Self(Mutex::new(Some(secret.to_string())))
    }
}

impl SecretStore for MemoryStore {
    fn load(&self) -> Option<String> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    fn save(&self, secret: &str) -> bool {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(secret.to_string());
        true
    }
    fn clear(&self) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
    fn lasting(&self) -> bool {
        false
    }
}

/// What a keyring answered: the store is not there (or refused), as opposed to "no entry".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unavailable;

/// The OS's keyring, one entry of it.
pub trait Keyring: Send + Sync {
    /// The secret (None: no entry yet).
    fn get(&self) -> Result<Option<String>, Unavailable>;
    fn set(&self, secret: &str) -> Result<(), Unavailable>;
    fn delete(&self);
}

/// A keyring, or memory once it is found not there: the first refusal turns to memory for the
/// rest of the run (and `lasting()` is false from then on).
pub struct Kept<K: Keyring> {
    keyring: K,
    memory: MemoryStore,
    fallen_back: AtomicBool,
}

impl<K: Keyring> Kept<K> {
    pub fn new(keyring: K) -> Self {
        Self { keyring, memory: MemoryStore::new(), fallen_back: AtomicBool::new(false) }
    }

    fn in_memory(&self) -> bool {
        self.fallen_back.load(Ordering::Relaxed)
    }

    fn fall_back(&self) {
        self.fallen_back.store(true, Ordering::Relaxed);
    }
}

impl<K: Keyring> SecretStore for Kept<K> {
    fn load(&self) -> Option<String> {
        if self.in_memory() {
            return self.memory.load();
        }
        match self.keyring.get() {
            Ok(s) => s.filter(|t| !t.is_empty()),
            Err(Unavailable) => {
                self.fall_back();
                self.memory.load()
            }
        }
    }
    fn save(&self, secret: &str) -> bool {
        if !self.in_memory() && self.keyring.set(secret).is_ok() {
            return true;
        }
        // (no keyring: the session lasts as long as the program)
        self.fall_back();
        self.memory.save(secret)
    }
    fn clear(&self) {
        self.memory.clear();
        // (also after falling back to memory: a session an earlier run kept there must not come
        // back at the next start)
        self.keyring.delete();
    }
    fn lasting(&self) -> bool {
        !self.in_memory()
    }
}

/// The OS's credential store, through the `keyring` crate.
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub struct OsKeyring;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
impl OsKeyring {
    fn entry() -> Result<keyring::Entry, Unavailable> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(|_| Unavailable)
    }
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
impl Keyring for OsKeyring {
    fn get(&self) -> Result<Option<String>, Unavailable> {
        match Self::entry()?.get_password() {
            Ok(s) => Ok(Some(s)),
            Err(keyring::Error::NoEntry) => Ok(None),
            // (an entry that does not read - not text, two of them - is as none: signed out)
            Err(keyring::Error::BadEncoding(_) | keyring::Error::Ambiguous(_)) => Ok(None),
            // (no Secret Service, no D-Bus, a locked keyring left locked, no access)
            Err(_) => Err(Unavailable),
        }
    }
    fn set(&self, secret: &str) -> Result<(), Unavailable> {
        Self::entry()?.set_password(secret).map_err(|_| Unavailable)
    }
    fn delete(&self) {
        if let Ok(e) = Self::entry() {
            let _ = e.delete_credential();
        }
    }
}

/// The OS's store where there is one (memory once it is found not there), else memory.
pub fn os_store() -> Box<dyn SecretStore> {
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    return Box::new(Kept::new(OsKeyring));
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return Box::new(MemoryStore::new());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A keyring in a test: there or not, and what it holds.
    #[derive(Default)]
    struct Fake {
        there: AtomicBool,
        held: Mutex<Option<String>>,
    }

    impl Fake {
        fn new(there: bool) -> Self {
            Self { there: AtomicBool::new(there), held: Mutex::new(None) }
        }
        fn gate(&self) -> Result<(), Unavailable> {
            if self.there.load(Ordering::Relaxed) {
                Ok(())
            } else {
                Err(Unavailable)
            }
        }
    }

    impl Keyring for &Fake {
        fn get(&self) -> Result<Option<String>, Unavailable> {
            self.gate()?;
            Ok(self.held.lock().unwrap().clone())
        }
        fn set(&self, secret: &str) -> Result<(), Unavailable> {
            self.gate()?;
            *self.held.lock().unwrap() = Some(secret.into());
            Ok(())
        }
        fn delete(&self) {
            *self.held.lock().unwrap() = None;
        }
    }

    #[test]
    fn a_keyring_keeps_the_session_for_the_next_start() {
        let k = Fake::new(true);
        let s = Kept::new(&k);
        assert_eq!(s.load(), None, "no entry yet: signed out, the keyring still there");
        assert!(s.lasting());
        assert!(s.save("r1"));
        assert_eq!(k.held.lock().unwrap().as_deref(), Some("r1"));
        // (the next start)
        assert_eq!(Kept::new(&k).load().as_deref(), Some("r1"));
        s.clear();
        assert_eq!(*k.held.lock().unwrap(), None);
    }

    /// No Secret Service (or it refuses): the session is kept in memory for this run, and the
    /// store says it does not last, so the page can tell the player.
    #[test]
    fn without_a_keyring_the_session_is_kept_in_memory_and_said_so() {
        let k = Fake::new(false);
        let s = Kept::new(&k);
        assert_eq!(s.load(), None);
        assert!(!s.lasting());
        assert!(s.save("r1"));
        assert_eq!(s.load().as_deref(), Some("r1"));
        assert_eq!(*k.held.lock().unwrap(), None, "nothing written anywhere");
        s.clear();
        assert_eq!(s.load(), None);
        // (one that refuses only to write: memory too, from then on)
        let k = Fake::new(true);
        let s = Kept::new(&k);
        assert!(s.lasting());
        k.there.store(false, Ordering::Relaxed);
        assert!(s.save("r2"));
        assert!(!s.lasting());
        k.there.store(true, Ordering::Relaxed);
        assert_eq!(s.load().as_deref(), Some("r2"), "the session of this run, from memory");
        assert_eq!(*k.held.lock().unwrap(), None);
    }
}
