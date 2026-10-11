use player_core::{extract_palette, ExtractedColors};

#[test]
fn test_default_palette_on_empty() {
    let pal = extract_palette(&[]);
    assert_eq!(pal, ExtractedColors::default());
}

#[test]
fn test_extract_palette_from_image() {
    let mut img = image::RgbImage::new(100, 100);
    // Fill with predominantly electric blue/cyan and some orange
    for (x, _y, pixel) in img.enumerate_pixels_mut() {
        if x < 70 {
            *pixel = image::Rgb([20, 180, 240]); // Vivid cyan
        } else {
            *pixel = image::Rgb([255, 100, 20]); // Orange
        }
    }

    let mut jpeg_bytes = Vec::new();
    let dynamic_img = image::DynamicImage::ImageRgb8(img);
    dynamic_img
        .write_to(
            &mut std::io::Cursor::new(&mut jpeg_bytes),
            image::ImageOutputFormat::Jpeg(85),
        )
        .expect("JPEG encode failed");

    let pal = extract_palette(&jpeg_bytes);
    // Primary or secondary should capture the vibrant cyan and orange
    assert!(pal.primary[3] > 0.9);
    assert!(pal.secondary[3] > 0.9);
    assert!(pal.accent[3] > 0.9);
    assert!(pal.background[3] > 0.9);

    // Background should be deep/dark
    let bg_bright = 0.299 * pal.background[0] + 0.587 * pal.background[1] + 0.114 * pal.background[2];
    assert!(bg_bright < 0.35, "Background should be dark and ambient");
}
