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
        assert!(
            !font_bytes.is_empty(),
            "Loaded font buffer must not be empty"
        );
    }
}

#[test]
fn test_cjk_font_bundle() {
    use iced::advanced::graphics::text::cosmic_text::{
        Attrs, Buffer, FontSystem, Metrics, Shaping,
    };

    let bundle = font::load_cjk_fonts();
    let mut db = iced::advanced::graphics::text::cosmic_text::fontdb::Database::new();
    for data in &bundle.font_data {
        db.load_font_source(
            iced::advanced::graphics::text::cosmic_text::fontdb::Source::Binary(
                std::sync::Arc::new(data.clone()),
            ),
        );
    }
    let mut font_system = FontSystem::new_with_locale_and_db("en-US".to_string(), db);

    let default_family = bundle
        .primary_family
        .as_deref()
        .unwrap_or("Noto Sans CJK JP");

    for shaping in [Shaping::Basic, Shaping::Advanced] {
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 18.0));
        let attrs = Attrs::new().family(iced::advanced::graphics::text::cosmic_text::Family::Name(
            default_family,
        ));
        // "周杰伦 - 晴天"
        buffer.set_text(&mut font_system, "周杰伦 - 晴天", attrs, shaping);
        buffer.shape_until_scroll(&mut font_system, false);

        println!("=== {:?} shaping for '周杰伦 - 晴天' ===", shaping);
        let mut missing_glyphs = 0;
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                println!(
                    "  Glyph id: {} from font {:?}",
                    glyph.glyph_id, glyph.font_id
                );
                if glyph.glyph_id == 0 {
                    missing_glyphs += 1;
                }
            }
        }
        if shaping == Shaping::Advanced {
            assert_eq!(
                missing_glyphs, 0,
                "Shaping::Advanced must find valid glyphs for all CJK characters"
            );
        }
    }
}
