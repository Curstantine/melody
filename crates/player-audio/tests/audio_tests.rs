use player_audio::{AudioConverter, AudioStatus};

#[test]
fn test_audio_status_atomic_primitives() {
    let status = AudioStatus::new();

    assert_eq!(status.position_ms(), 0);
    assert_eq!(status.duration_ms(), 0);
    assert!(!status.is_playing());
    assert!(!status.is_loaded());
    assert_eq!(status.volume(), 1.0);
    assert!(!status.is_ended());

    // Update position and verify relaxed read
    status.set_position_ms(12500);
    assert_eq!(status.position_ms(), 12500);

    // Update duration
    status.set_duration_ms(240000);
    assert_eq!(status.duration_ms(), 240000);

    // Update playing state
    status.set_playing(true);
    assert!(status.is_playing());

    // Volume clamping
    status.set_volume(0.75);
    assert!((status.volume() - 0.75).abs() < 0.002);

    status.set_volume(2.5); // clamps to 2.0
    assert!((status.volume() - 2.0).abs() < 0.002);

    status.set_volume(-0.5); // clamps to 0.0
    assert_eq!(status.volume(), 0.0);

    // End flag & swap
    status.set_ended(true);
    assert!(status.is_ended());
    assert!(status.take_ended());
    assert!(!status.is_ended());
}

#[test]
fn test_audio_converter_mono_to_stereo() {
    let mut converter = AudioConverter::new(1, 44100, 2, 44100);

    let input = vec![0.5, -0.5, 1.0, 0.0];
    let mut output = Vec::new();

    converter.convert(&input, &mut output);

    // Each mono sample should be duplicated into L and R channels
    assert_eq!(output.len(), 8);
    assert_eq!(output, vec![0.5, 0.5, -0.5, -0.5, 1.0, 1.0, 0.0, 0.0]);
}

#[test]
fn test_audio_converter_resampling() {
    // 22050 Hz to 44100 Hz stereo (2x upsampling)
    let mut converter = AudioConverter::new(2, 22050, 2, 44100);

    let input = vec![0.0, 0.0, 1.0, 1.0];
    let mut output = Vec::new();

    converter.convert(&input, &mut output);

    // Should produce roughly 4 frames (8 samples)
    assert!(output.len() >= 6);
}
