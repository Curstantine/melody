use std::collections::HashSet;
use std::sync::Mutex;

fn intern_str(s: &str) -> &'static str {
    static INTERNED: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut guard = INTERNED.lock().unwrap();
    let set = guard.get_or_insert_with(HashSet::new);
    if let Some(&existing) = set.get(s) {
        existing
    } else {
        let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
        set.insert(leaked);
        leaked
    }
}

pub fn font_from_family_name(name: Option<&str>) -> iced::Font {
    if let Some(name) = name {
        let family_static = intern_str(name);
        iced::Font::new(family_static)
    } else {
        iced::Font::DEFAULT
    }
}

/// Sorted, de-duplicated family names for the font picker.
/// Uses fontdb (memory-mapped, parses name tables only), so nothing stays in RAM.
pub fn list_font_families() -> Vec<String> {
    use iced::advanced::graphics::text::cosmic_text::fontdb;

    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let mut families = std::collections::BTreeSet::new();
    for face in db.faces() {
        if let Some((name, _)) = face.families.first() {
            families.insert(name.clone());
        }
    }

    families.into_iter().collect()
}
