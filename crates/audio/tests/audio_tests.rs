use audio::{AudioConverter, AudioStatus};

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

    // FFT resampler operates in 1024-frame chunks
    let input = vec![0.5f32; 1024 * 2];
    let mut output = Vec::new();

    converter.convert(&input, &mut output);

    // 2x upsampling of 1024 stereo frames should produce ~2048 stereo frames (4096 samples)
    assert!(output.len() >= 2048);
}

#[test]
fn test_audio_converter_unaligned_packet_residuals() {
    // 48000 Hz to 44100 Hz stereo
    let mut converter = AudioConverter::new(2, 48000, 2, 44100);

    // Feed non-multiple of chunk size (e.g. 3000 frames)
    // Packet 1: 3000 frames -> processes 2048 frames (2 chunks), leaves 952 frames buffered.
    let packet1 = vec![0.1f32; 3000 * 2];
    let mut output1 = Vec::new();
    converter.convert(&packet1, &mut output1);
    assert!(!output1.is_empty());

    // Packet 2: 3000 frames -> total 3952 frames -> processes 3072 frames (3 chunks), leaves 880 frames buffered.
    let packet2 = vec![0.1f32; 3000 * 2];
    let mut output2 = Vec::new();
    converter.convert(&packet2, &mut output2);
    assert!(!output2.is_empty());

    // Flush on EOF: flushes the remaining 880 frames
    let mut output_flush = Vec::new();
    converter.flush(&mut output_flush);
    assert!(!output_flush.is_empty());
}

#[test]
fn test_ring_buffer_flush_synchronization() {
    use ringbuf::traits::{Consumer, Producer, Split};
    use ringbuf::HeapRb;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    let rb = HeapRb::<f32>::new(1024);
    let (mut producer, mut consumer) = rb.split();
    let flush_flag = Arc::new(AtomicBool::new(false));

    // Simulate old track samples in the buffer
    let old_samples = vec![1.0f32; 256];
    producer.push_slice(&old_samples);

    // Simulate worker requesting a flush
    let flush_flag_worker = flush_flag.clone();
    flush_flag_worker.store(true, Ordering::SeqCst);

    // Simulate callback clearing when it sees the flush flag
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(5));
        if flush_flag.swap(false, Ordering::SeqCst) {
            consumer.clear();
        }
        consumer
    });

    // Worker waits for flush acknowledgment
    let start = std::time::Instant::now();
    while flush_flag_worker.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(1));
        if start.elapsed() > Duration::from_millis(100) {
            panic!("Flush timed out");
        }
    }

    // Now worker pushes the new track's samples starting at 0:00
    let new_samples = vec![0.5f32; 128];
    producer.push_slice(&new_samples);

    let mut consumer = handle.join().unwrap();
    // Consumer callback reads the new track's samples from the very beginning
    let mut read_buf = vec![0.0f32; 128];
    let read = consumer.pop_slice(&mut read_buf);
    assert_eq!(read, 128);
    assert_eq!(read_buf, new_samples);
}

