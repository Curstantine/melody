use player_core::get_or_create_thumbnail;

#[test]
fn test_thumbnail_generation_and_caching() {
    let temp_dir = std::env::temp_dir().join("vibecore_test_thumbnails");
    let _ = std::fs::create_dir_all(&temp_dir);

    // Create a 600x600 test image
    let mut img = image::RgbImage::new(600, 600);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = image::Rgb([(x % 256) as u8, (y % 256) as u8, 128]);
    }

    let song_path = temp_dir.join("test_track.flac");
    let cover_path = temp_dir.join("cover.jpg");

    std::fs::write(&song_path, b"dummy audio content").expect("Write song failed");
    img.save(&cover_path).expect("Save test cover failed");

    // 1. First call: extracts, resizes, and writes thumbnail to disk cache
    let thumb_opt = get_or_create_thumbnail(&song_path, 200);
    assert!(thumb_opt.is_some(), "Thumbnail should be generated");
    let thumb_bytes = thumb_opt.unwrap();
    assert!(!thumb_bytes.is_empty());

    // Verify thumbnail is downscaled to 200x200
    let decoded =
        image::load_from_memory(&thumb_bytes).expect("Failed to decode generated thumbnail");
    assert_eq!(decoded.width(), 200);
    assert_eq!(decoded.height(), 200);

    // 2. Second call: should hit disk cache directly
    let cached_thumb = get_or_create_thumbnail(&song_path, 200);
    assert_eq!(cached_thumb, Some(thumb_bytes));

    // 3. Track with non-square cover (800x500)
    let non_square_dir = temp_dir.join("non_square_dir");
    let _ = std::fs::create_dir_all(&non_square_dir);
    let non_square_song = non_square_dir.join("non_square.mp3");
    let non_square_cover = non_square_dir.join("cover.jpg");
    std::fs::write(&non_square_song, b"dummy audio").expect("Write song failed");
    let non_square_img = image::RgbImage::new(800, 500);
    non_square_img
        .save(&non_square_cover)
        .expect("Save non-square cover failed");

    let non_square_thumb =
        get_or_create_thumbnail(&non_square_song, 200).expect("Thumbnail failed");
    let decoded_non_square =
        image::load_from_memory(&non_square_thumb).expect("Decode non-square failed");
    assert_eq!(decoded_non_square.width(), 200);
    assert_eq!(decoded_non_square.height(), 200);

    // 4. Track with no cover in an isolated directory
    let isolated_dir = temp_dir.join("no_cover_dir");
    let _ = std::fs::create_dir_all(&isolated_dir);
    let no_cover_song = isolated_dir.join("no_cover.mp3");
    std::fs::write(&no_cover_song, b"dummy audio").expect("Write song failed");
    let none_res = get_or_create_thumbnail(&no_cover_song, 200);
    assert_eq!(none_res, None);

    // Cleanup test files
    let _ = std::fs::remove_dir_all(&temp_dir);
}
