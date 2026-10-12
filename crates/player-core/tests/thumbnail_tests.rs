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

#[test]
fn test_flac_embedded_picture_empty_mime() {
    let temp_dir = std::env::temp_dir().join("vibecore_test_flac_pic");
    let _ = std::fs::create_dir_all(&temp_dir);
    let flac_file = temp_dir.join("test_empty_mime.flac");

    // 1. Create a 100x100 JPEG test image
    let img = image::RgbImage::new(100, 100);
    let mut jpeg_bytes = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(
            &mut std::io::Cursor::new(&mut jpeg_bytes),
            image::ImageOutputFormat::Jpeg(80),
        )
        .unwrap();

    // 2. Build a valid FLAC container with STREAMINFO and PICTURE blocks
    let mut flac_data = Vec::new();
    // FLAC marker
    flac_data.extend_from_slice(b"fLaC");

    // Block 0: STREAMINFO (34 bytes), not last
    flac_data.push(0x00); // is_last=0, type=0
    flac_data.extend_from_slice(&[0x00, 0x00, 0x22]); // len=34
    flac_data.resize(flac_data.len() + 34, 0);

    // Block 6: PICTURE, is_last=1
    // Format:
    // 4 bytes: picture type (3 = front cover)
    // 4 bytes: mime type length (0 = empty)
    // 0 bytes: mime type
    // 4 bytes: desc length (0)
    // 0 bytes: desc
    // 16 bytes: width, height, depth, colors (zeros)
    // 4 bytes: data length
    // data_len bytes: jpeg data
    let mut pic_payload = Vec::new();
    pic_payload.extend_from_slice(&3u32.to_be_bytes()); // type 3: Cover front
    pic_payload.extend_from_slice(&0u32.to_be_bytes()); // mime_len = 0
    pic_payload.extend_from_slice(&0u32.to_be_bytes()); // desc_len = 0
    pic_payload.extend_from_slice(&[0u8; 16]); // 16 bytes w, h, depth, colors
    pic_payload.extend_from_slice(&(jpeg_bytes.len() as u32).to_be_bytes());
    pic_payload.extend_from_slice(&jpeg_bytes);

    let pic_len = pic_payload.len() as u32;
    flac_data.push(0x80 | 0x06); // is_last=1, type=6 (PICTURE)
    flac_data.push(((pic_len >> 16) & 0xFF) as u8);
    flac_data.push(((pic_len >> 8) & 0xFF) as u8);
    flac_data.push((pic_len & 0xFF) as u8);
    flac_data.extend_from_slice(&pic_payload);

    std::fs::write(&flac_file, &flac_data).unwrap();

    // 3. Test extraction
    let extracted = player_core::extract_cover_art(&flac_file);
    assert!(extracted.is_some(), "Should extract embedded FLAC picture with empty MIME");
    let art_bytes = extracted.unwrap();
    assert_eq!(art_bytes, jpeg_bytes);

    // 4. Test thumbnail generation
    let thumb = get_or_create_thumbnail(&flac_file, 200);
    assert!(thumb.is_some(), "Thumbnail generation must succeed");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_user_reported_aurora_sky_track_cover_extraction() {
    let target = std::path::Path::new(
        "/home/curstantine/Music/Library/Aurora Sky/Spirits/01 Aurora Sky - I See You Shine In The Moonlight.flac",
    );
    if target.is_file() {
        let extracted = player_core::extract_cover_art(target);
        assert!(
            extracted.is_some(),
            "Must successfully extract cover from the user's Aurora Sky track"
        );
        let thumb = get_or_create_thumbnail(target, 200);
        assert!(thumb.is_some(), "Must generate thumbnail for the user's track");
        let decoded = image::load_from_memory(&thumb.unwrap()).unwrap();
        assert_eq!(decoded.width(), 200);
        assert_eq!(decoded.height(), 200);
    }
}
