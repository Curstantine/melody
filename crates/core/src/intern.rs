//! Thread-safe string interning for repeated library metadata.
//!
//! Album and artist names dominate metadata duplication: a 100k-track library
//! may contain only a few thousand distinct artists and tens of thousands of
//! albums. Storing one `Arc<str>` per distinct value and sharing it across rows
//! mirrors the reference-counted metadata pools used by foobar2000/MusicBee and
//! keeps memory proportional to the *distinct* catalog rather than track count.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};

/// Shared, interned string handle. Dereferences to `&str`.
pub type InternedStr = Arc<str>;

struct Interner {
    pool: Mutex<HashSet<Arc<str>>>,
}

impl Interner {
    fn new() -> Self {
        Self {
            pool: Mutex::new(HashSet::new()),
        }
    }

    /// Returns a shared handle for `s`, allocating only the first time a value
    /// is seen.
    fn intern(&self, s: &str) -> Arc<str> {
        if s.is_empty() {
            return Arc::from("");
        }

        let mut pool = match self.pool.lock() {
            Ok(pool) => pool,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(existing) = pool.get(s) {
            return existing.clone();
        }
        let interned: Arc<str> = Arc::from(s);
        pool.insert(interned.clone());
        interned
    }

    fn clear(&self) {
        if let Ok(mut pool) = self.pool.lock() {
            pool.clear();
            pool.shrink_to_fit();
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.pool.lock().map(|pool| pool.len()).unwrap_or(0)
    }
}

static GLOBAL: OnceLock<Interner> = OnceLock::new();

fn global() -> &'static Interner {
    GLOBAL.get_or_init(Interner::new)
}

/// Interns `s` into the global metadata pool.
pub fn intern(s: &str) -> InternedStr {
    global().intern(s)
}

/// Interns an optional string, preserving `None`.
pub fn intern_opt(s: Option<&str>) -> Option<InternedStr> {
    s.map(intern)
}

/// Drops every pooled string. Call when the library is cleared to avoid
/// retaining metadata from a previous scan.
pub fn clear() {
    global().clear();
}

/// Number of distinct interned strings currently retained.
#[cfg(test)]
pub fn distinct_count() -> usize {
    global().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_dedupes_equal_strings() {
        let a = intern("Pink Floyd");
        let b = intern("Pink Floyd");
        assert!(Arc::ptr_eq(&a, &b));
        assert!(distinct_count() >= 1);
    }

    #[test]
    fn intern_opt_preserves_none() {
        assert!(intern_opt(None).is_none());
        let some = intern_opt(Some("Yes")).unwrap();
        assert_eq!(&*some, "Yes");
    }

    #[test]
    fn clear_resets_pool() {
        let value = intern("Transient Artist XYZ");
        assert_eq!(&*value, "Transient Artist XYZ");
        clear();
        let recreated = intern("Transient Artist XYZ");
        assert!(!Arc::ptr_eq(&value, &recreated));
    }
}
