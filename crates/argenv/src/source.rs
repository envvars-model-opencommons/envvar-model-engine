//! [`EnvSource`] — where a variable is read from.
//!
//! Reading is deliberately not hardwired to the current process. A launcher
//! composes the environment it is *about to hand a child process* and needs to
//! check that before `exec`, not after; a diagnostic tool reads a snapshot taken
//! from elsewhere; a test wants an isolated map instead of mutating global state
//! shared with every other test in the binary.
use std::collections::{BTreeMap, HashMap};

/// A set of environment variables that can be read and enumerated.
///
/// Enumeration is required because catching a misspelled name means noticing a
/// variable that is *present but undeclared* — which is impossible if you can
/// only look up names you already know.
pub trait EnvSource {
    /// The raw value of `name`, if present.
    fn get(&self, name: &str) -> Option<String>;

    /// Every variable name present in this source.
    fn names(&self) -> Vec<String>;
}

/// The current process's environment.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessEnv;

impl EnvSource for ProcessEnv {
    fn get(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
    fn names(&self) -> Vec<String> {
        std::env::vars().map(|(k, _)| k).collect()
    }
}

impl EnvSource for BTreeMap<String, String> {
    fn get(&self, name: &str) -> Option<String> {
        BTreeMap::get(self, name).cloned()
    }
    fn names(&self) -> Vec<String> {
        self.keys().cloned().collect()
    }
}

impl EnvSource for HashMap<String, String> {
    fn get(&self, name: &str) -> Option<String> {
        HashMap::get(self, name).cloned()
    }
    fn names(&self) -> Vec<String> {
        self.keys().cloned().collect()
    }
}

impl<T: EnvSource + ?Sized> EnvSource for &T {
    fn get(&self, name: &str) -> Option<String> {
        (**self).get(name)
    }
    fn names(&self) -> Vec<String> {
        (**self).names()
    }
}
