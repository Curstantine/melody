fn intern_str(s: &str) -> &'static str {
    use std::collections::HashSet;
    use std::sync::Mutex;

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
