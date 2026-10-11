#[path = "../src/font.rs"]
mod font;

#[test]
fn test_cjk_font_discovery() {
    let font_paths = font::discover_cjk_font_paths();
    println!("Discovered CJK font paths: {:?}", font_paths);

    for path in &font_paths {
        assert!(path.exists(), "Font path {:?} must exist", path);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        assert!(
            matches!(ext.as_str(), "ttf" | "otf" | "ttc" | "otc"),
            "Discovered file {:?} must have a font extension",
            path
        );
    }
}

#[test]
fn test_cjk_font_loading() {
    let fonts = font::load_cjk_font_data();
    println!("Loaded {} CJK font buffers", fonts.len());

    for font_bytes in &fonts {
        assert!(!font_bytes.is_empty(), "Loaded font buffer must not be empty");
    }
}
