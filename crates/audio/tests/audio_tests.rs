use audio::AudioStatus;

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

#[test]
fn test_track_decoding_to_eof() {
    let path = std::path::Path::new(
        "/home/curstantine/Music/Library/矢島舞依/The Un-Dead/01 Introduction.flac",
    );
    if !path.exists() {
        return;
    }
    let engine = audio::AudioEngine::new();
    let controller = engine.controller();
    controller.load(path.to_path_buf(), true);

    std::thread::sleep(std::time::Duration::from_millis(200));
    let dur = controller.status().duration_ms();
    assert!(dur > 0, "Duration must be greater than 0");

    // Seek to 1.5 seconds before end
    controller.seek(std::time::Duration::from_millis(dur.saturating_sub(1500)));

    let start = std::time::Instant::now();
    let mut saw_ended = false;
    while start.elapsed() < std::time::Duration::from_secs(5) {
        let pos = controller.status().position_ms();
        assert!(
            pos <= dur,
            "Position ({}) must never exceed duration ({})",
            pos,
            dur
        );
        if controller.status().is_ended() {
            saw_ended = true;
            assert_eq!(pos, dur, "When ended, position must equal duration");
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    assert!(saw_ended, "Track must reach ended state");

    // Test that a subsequent load command is immediately accepted and plays
    controller.load(path.to_path_buf(), true);
    let start2 = std::time::Instant::now();
    let mut second_track_played = false;
    while start2.elapsed() < std::time::Duration::from_secs(3) {
        if controller.status().is_playing() && controller.status().position_ms() > 100 {
            second_track_played = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(
        second_track_played,
        "Subsequent track load must play without hang"
    );
}

#[test]
fn test_gapless_status_and_boundary_transition() {
    use audio::{pop_samples_with_boundary, CallbackState, TrackBoundary};
    use ringbuf::traits::{Producer, Split};
    use ringbuf::HeapRb;
    use std::sync::atomic::AtomicBool;

    let status = AudioStatus::new();
    status.set_duration_ms(100_000);
    status.set_playing(true);

    assert!(!status.gapless_transition());
    status.set_gapless_transition(true);
    assert!(status.gapless_transition());
    assert!(status.take_gapless_transition());
    assert!(!status.gapless_transition());

    // Test sample-accurate boundary transition in pop_samples_with_boundary
    let audio_rb = HeapRb::<f32>::new(1024);
    let (mut audio_prod, mut audio_cons) = audio_rb.split();

    let boundary_rb = HeapRb::<TrackBoundary>::new(8);
    let (mut boundary_prod, mut boundary_cons) = boundary_rb.split();

    // Track 1 has 40 samples in the buffer
    let track1_samples = vec![1.0f32; 40];
    audio_prod.push_slice(&track1_samples);

    // Track 2 has 60 samples in the buffer
    let track2_samples = vec![2.0f32; 60];
    audio_prod.push_slice(&track2_samples);

    // Set boundary at sample 40, switching to Track 2 with duration 200_000 ms
    let _ = boundary_prod.try_push(TrackBoundary {
        switch_at_sample: 40,
        next_duration_ms: 200_000,
    });

    let flush_flag = AtomicBool::new(false);
    let mut current_boundary = None;
    let mut consumed_samples: u64 = 0;
    let mut pos_rem: u64 = 0;

    let mut state = CallbackState {
        consumer: &mut audio_cons,
        boundary_consumer: &mut boundary_cons,
        current_boundary: &mut current_boundary,
        consumed_samples: &mut consumed_samples,
        flush_flag: &flush_flag,
        status: &status,
        sample_rate: 44100,
        channels: 2,
        pos_rem: &mut pos_rem,
    };

    // Callback requests 100 samples (covering 40 samples of track 1 and 60 samples of track 2)
    let mut out = vec![0.0f32; 100];
    let read = pop_samples_with_boundary(&mut out, &mut state);

    assert_eq!(read, 100);
    assert_eq!(&out[..40], &track1_samples[..]);
    assert_eq!(&out[40..100], &track2_samples[..]);

    // Check that gapless transition was triggered
    assert!(status.take_gapless_transition());
    assert_eq!(status.duration_ms(), 200_000);
    assert_eq!(state.current_boundary, &None);
    assert_eq!(*state.consumed_samples, 100);
}

#[test]
fn test_resampling_configuration_and_dictionary() {
    use audio::{ResamplingConfig, ResamplingFilter, ResamplingQuality};

    let default_cfg = ResamplingConfig::default();
    assert_eq!(default_cfg.filter, ResamplingFilter::Kaiser);
    assert_eq!(default_cfg.quality, ResamplingQuality::Standard);
    assert!(default_cfg.linear_interp);

    for filter in ResamplingFilter::ALL {
        for quality in ResamplingQuality::ALL {
            let cfg = ResamplingConfig {
                filter,
                quality,
                linear_interp: true,
            };
            let _dict = cfg.to_dictionary();
        }
    }
}

#[test]
fn test_resampler_context_creation_with_filters() {
    let _ = ffmpeg_next::init();
    use audio::{ResamplingConfig, ResamplingFilter, ResamplingQuality};
    use ffmpeg_next::format::sample::Type as SampleType;
    use ffmpeg_next::format::Sample;
    use ffmpeg_next::software::resampling::context::Context as ResamplerContext;
    use ffmpeg_next::ChannelLayout;

    let src_format = Sample::I16(SampleType::Packed);
    let src_layout = ChannelLayout::STEREO;
    let src_rate = 44100;
    let dst_format = Sample::F32(SampleType::Packed);
    let dst_layout = ChannelLayout::STEREO;
    let dst_rate = 48000;

    let filters = [
        ResamplingFilter::Kaiser,
        ResamplingFilter::BlackmanNuttall,
        ResamplingFilter::Cubic,
    ];

    for filter in filters {
        let cfg = ResamplingConfig {
            filter,
            quality: ResamplingQuality::High,
            linear_interp: true,
        };
        let res = ResamplerContext::get_with(
            src_format,
            src_layout,
            src_rate,
            dst_format,
            dst_layout,
            dst_rate,
            cfg.to_dictionary(),
        );
        assert!(res.is_ok(), "Failed for filter {filter:?}: {:?}", res.err());
    }
}
