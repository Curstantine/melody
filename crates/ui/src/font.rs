use iced::advanced::graphics::text::cosmic_text::fontdb;

/// Sorted, de-duplicated family names for the font picker.
/// Uses fontdb (memory-mapped, parses name tables only), so nothing stays in RAM.
pub fn list_font_families() -> Vec<String> {
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
