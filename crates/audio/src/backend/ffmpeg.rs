use std::path::{Path, PathBuf};
use std::time::Duration;

use ffmpeg_next::format::context::Input;
use ffmpeg_next::media::Type;
use ffmpeg_next::software::resampling::context::Context as ResamplerContext;
use ffmpeg_next::util::{
    channel_layout::ChannelLayout,
    format::{sample::Type as SampleType, Sample},
    frame::Audio as AudioFrame,
};
use tracing::{info, warn};

use crate::backend::RingSink;
use crate::resample::ResamplingConfig;

fn init_ffmpeg() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = ffmpeg_next::init();
    });
}

fn target_channel_layout(channels: usize) -> ChannelLayout {
    match channels {
        1 => ChannelLayout::MONO,
        2 => ChannelLayout::STEREO,
        3 => ChannelLayout::SURROUND,
        4 => ChannelLayout::QUAD,
        5 => ChannelLayout::_5POINT0,
        6 => ChannelLayout::_5POINT1,
        8 => ChannelLayout::_7POINT1,
        n => ChannelLayout::default(n as i32),
    }
}

fn get_decoder_channel_layout(decoder: &ffmpeg_next::codec::decoder::Audio) -> ChannelLayout {
    if decoder.channel_layout().is_empty() {
        let nb_channels = unsafe { (*decoder.as_ptr()).ch_layout.nb_channels };
        let ch = if nb_channels > 0 {
            nb_channels as usize
        } else if decoder.channels() > 0 {
            decoder.channels() as usize
        } else {
            2
        };
        target_channel_layout(ch)
    } else {
        decoder.channel_layout()
    }
}

fn create_resampler(
    src_format: Sample,
    src_channel_layout: ChannelLayout,
    src_rate: u32,
    dst_sample_rate: u32,
    dst_channels: usize,
    resampling_cfg: &ResamplingConfig,
) -> Result<ResamplerContext, ffmpeg_next::Error> {
    let dst_format = Sample::F32(SampleType::Packed);
    let dst_channel_layout = target_channel_layout(dst_channels);
    let options = resampling_cfg.to_dictionary();

    match ResamplerContext::get_with(
        src_format,
        src_channel_layout,
        src_rate,
        dst_format,
        dst_channel_layout,
        dst_sample_rate,
        options,
    ) {
        Ok(resampler) => Ok(resampler),
        Err(err) => {
            warn!(
                "Failed to initialize resampler with {:?}: {}. Falling back to default SWR Kaiser sinc.",
                resampling_cfg, err
            );
            let fallback_dict = ResamplingConfig::default().to_dictionary();
            ResamplerContext::get_with(
                src_format,
                src_channel_layout,
                src_rate,
                dst_format,
                dst_channel_layout,
                dst_sample_rate,
                fallback_dict,
            )
        }
    }
}

fn push_frame_data(data_bytes: &[u8], total_floats: usize, sink: &mut RingSink<'_>) -> bool {
    let byte_count = total_floats * std::mem::size_of::<f32>();
    if data_bytes.len() < byte_count {
        return true;
    }
    let ptr = data_bytes.as_ptr();
    if (ptr as usize).is_multiple_of(std::mem::align_of::<f32>()) {
        let float_slice = unsafe { std::slice::from_raw_parts(ptr as *const f32, total_floats) };
        sink.push_interleaved(float_slice)
    } else {
        let mut aligned = vec![0.0f32; total_floats];
        unsafe {
            std::ptr::copy_nonoverlapping(ptr, aligned.as_mut_ptr() as *mut u8, byte_count);
        }
        sink.push_interleaved(&aligned)
    }
}

pub(crate) struct LoadedTrack {
    path: PathBuf,
    duration_ms: u64,
    input: Input,
    decoder: ffmpeg_next::codec::decoder::Audio,
    stream_index: usize,
    resampler: ResamplerContext,
    dst_channels: usize,
    reached_eof: bool,
    decoded_frame: AudioFrame,
    resampled_frame: AudioFrame,
}

impl LoadedTrack {
    pub(crate) fn load(
        path: &Path,
        dst_sample_rate: u32,
        dst_channels: usize,
        resampling_cfg: &ResamplingConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        init_ffmpeg();

        let input = ffmpeg_next::format::input(path)?;
        let audio_stream = input
            .streams()
            .best(Type::Audio)
            .ok_or("No audio stream found in file")?;

        let stream_index = audio_stream.index();
        let time_base = audio_stream.time_base();

        let duration_ms = if audio_stream.duration() > 0 {
            let tb = f64::from(time_base.numerator()) / f64::from(time_base.denominator());
            (audio_stream.duration() as f64 * tb * 1000.0) as u64
        } else if input.duration() > 0 {
            (input.duration() as f64 / f64::from(ffmpeg_next::ffi::AV_TIME_BASE) * 1000.0) as u64
        } else {
            0
        };

        let context_decoder =
            ffmpeg_next::codec::context::Context::from_parameters(audio_stream.parameters())?;
        let decoder = context_decoder.decoder().audio()?;

        let src_format = decoder.format();
        let src_channel_layout = get_decoder_channel_layout(&decoder);
        let src_rate = if decoder.rate() > 0 {
            decoder.rate()
        } else {
            44100
        };

        let resampler = create_resampler(
            src_format,
            src_channel_layout,
            src_rate,
            dst_sample_rate,
            dst_channels,
            resampling_cfg,
        )?;

        Ok(Self {
            path: path.to_path_buf(),
            duration_ms,
            input,
            decoder,
            stream_index,
            resampler,
            dst_channels,
            reached_eof: false,
            decoded_frame: AudioFrame::empty(),
            resampled_frame: AudioFrame::empty(),
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn duration_ms(&self) -> u64 {
        self.duration_ms
    }

    pub(crate) fn reached_eof(&self) -> bool {
        self.reached_eof
    }

    pub(crate) fn seek_to_start(&mut self) {
        let _ = self.input.seek(0, ..);
        self.decoder.flush();
        while let Ok(Some(_)) = self.resampler.flush(&mut self.resampled_frame) {
            if self.resampled_frame.samples() == 0 {
                break;
            }
        }
        self.reached_eof = false;
    }

    pub(crate) fn seek(
        &mut self,
        target: Duration,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // ffmpeg-next input.seek passes stream_index = -1 to avformat_seek_file,
        // which expects timestamp in AV_TIME_BASE units (microseconds).
        let seek_micros = target.as_micros() as i64;
        self.input.seek(seek_micros, ..)?;
        self.decoder.flush();
        while let Ok(Some(_)) = self.resampler.flush(&mut self.resampled_frame) {
            if self.resampled_frame.samples() == 0 {
                break;
            }
        }
        self.reached_eof = false;
        info!("Seeked to {:?}", target);
        Ok(())
    }

    pub(crate) fn reconfigure_resampler(
        &mut self,
        dst_sample_rate: u32,
        dst_channels: usize,
        new_cfg: &ResamplingConfig,
    ) {
        self.dst_channels = dst_channels;
        let src_ch = get_decoder_channel_layout(&self.decoder);
        let src_rate = if self.decoder.rate() > 0 {
            self.decoder.rate()
        } else {
            44100
        };
        match create_resampler(
            self.decoder.format(),
            src_ch,
            src_rate,
            dst_sample_rate,
            dst_channels,
            new_cfg,
        ) {
            Ok(r) => self.resampler = r,
            Err(e) => warn!("Failed to re-initialize resampler: {}", e),
        }
    }

    fn drain_decoder_frames(&mut self, sink: &mut RingSink<'_>) {
        while self.decoder.receive_frame(&mut self.decoded_frame).is_ok() {
            if sink.is_flushing() {
                break;
            }

            if self.decoded_frame.channel_layout().is_empty() {
                let nb_channels = unsafe { (*self.decoded_frame.as_ptr()).ch_layout.nb_channels };
                let layout = if nb_channels > 0 {
                    target_channel_layout(nb_channels as usize)
                } else {
                    get_decoder_channel_layout(&self.decoder)
                };
                self.decoded_frame.set_channel_layout(layout);
            }

            match self
                .resampler
                .run(&self.decoded_frame, &mut self.resampled_frame)
            {
                Ok(_) => {
                    let samples_count = self.resampled_frame.samples();
                    if samples_count > 0 {
                        let total_floats = samples_count * self.dst_channels;
                        let data_bytes = self.resampled_frame.data(0);
                        if !push_frame_data(data_bytes, total_floats, sink) {
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!("Resampling error: {}", e);
                }
            }
        }
    }

    fn flush_resampler(&mut self, sink: &mut RingSink<'_>) {
        while let Ok(Some(_)) = self.resampler.flush(&mut self.resampled_frame) {
            if sink.is_flushing() {
                break;
            }

            let samples_count = self.resampled_frame.samples();
            if samples_count == 0 {
                break;
            }
            let total_floats = samples_count * self.dst_channels;
            let data_bytes = self.resampled_frame.data(0);
            if !push_frame_data(data_bytes, total_floats, sink) {
                break;
            }
        }
    }

    /// Decodes one packet (or flushes on EOF). Returns `true` when EOF has been reached.
    pub(crate) fn decode_step(&mut self, sink: &mut RingSink<'_>) -> bool {
        match self.input.packets().next() {
            Some((stream, packet)) => {
                if stream.index() == self.stream_index {
                    if let Err(e) = self.decoder.send_packet(&packet) {
                        warn!("Audio decoder send_packet error: {}", e);
                    } else {
                        self.drain_decoder_frames(sink);
                    }
                }
                false
            }
            None => {
                self.reached_eof = true;
                let _ = self.decoder.send_eof();
                self.drain_decoder_frames(sink);
                self.flush_resampler(sink);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ringbuf::traits::Split;
    use ringbuf::HeapRb;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    /// Builds a minimal valid RIFF WAVE (PCM 16-bit, 44100 Hz) file using
    /// `WAVE_FORMAT_PCM` (tag 1), which leaves channel layout unspecified
    /// (`AV_CHANNEL_ORDER_UNSPEC`), matching real-world `.wav` files.
    fn write_test_pcm_wav(path: &Path, channels: u16, num_frames: u32) {
        let sample_rate: u32 = 44100;
        let bits_per_sample: u16 = 16;
        let block_align = channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * u32::from(block_align);
        let data_size = num_frames * u32::from(block_align);
        let riff_size = 36 + data_size;

        let mut buf = Vec::with_capacity((44 + data_size) as usize);
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&riff_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes()); // WAVE_FORMAT_PCM
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        for i in 0..num_frames {
            let sample = ((i as f32 * 440.0 * std::f32::consts::TAU / sample_rate as f32).sin()
                * 16000.0) as i16;
            for _ in 0..channels {
                buf.extend_from_slice(&sample.to_le_bytes());
            }
        }
        std::fs::write(path, buf).expect("write test wav");
    }

    #[test]
    fn decodes_pcm_wav_with_unspecified_channel_layout() {
        for channels in [1u16, 2u16] {
            let wav_path = std::env::temp_dir().join(format!(
                "melody-test-unspecified-layout-{}ch-{}.wav",
                channels,
                std::process::id()
            ));
            write_test_pcm_wav(&wav_path, channels, 4410);

            let mut track = LoadedTrack::load(&wav_path, 48000, 2, &ResamplingConfig::default())
                .expect("load wav track");

            let rb = HeapRb::<f32>::new(65536);
            let (prod, _cons) = rb.split();
            let mut ring_producer = Some(prod);
            let flush_flag = Arc::new(AtomicBool::new(false));
            let mut written_samples = 0u64;
            let mut sink = RingSink {
                ring_producer: &mut ring_producer,
                flush_flag: &flush_flag,
                written_samples: &mut written_samples,
            };

            while !track.decode_step(&mut sink) {}
            let _ = std::fs::remove_file(&wav_path);

            assert!(
                written_samples > 0,
                "expected decoded samples from {channels}-ch PCM WAV with unspecified channel layout, got 0"
            );
        }
    }
}
