use crate::frame::{calculate_entropy, group_into_runs, FrameInfo};
use anyhow::{anyhow, Result};
use colored::Colorize;
use log::debug;
use plotters::prelude::*;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub fn analyze(input_path: &Path) -> Result<()> {
    println!(
        "{} {}",
        "「analyzing」".cyan().bold(),
        input_path.display().to_string().yellow()
    );
    println!();

    let reported_duration = mp3_duration::from_path(input_path)?;
    let naive_duration = calculate_naive_duration(input_path)?;

    println!(
        "{} {:.3}s",
        "「reported duration」".green().bold(),
        reported_duration.as_secs_f64()
    );
    println!(
        "{} {:.3}s",
        "「frame-based duration」".green().bold(),
        naive_duration
    );

    let diff = (reported_duration.as_secs_f64() - naive_duration).abs();
    if diff > 1.0 {
        println!(
            "{} {:.3}s difference",
            "「duration mismatch」".red().bold(),
            diff
        );
    } else {
        println!("{}", "「duration check passed」".green().bold());
    }
    println!();

    analyze_structure(input_path)?;

    Ok(())
}

fn calculate_naive_duration(input_path: &Path) -> Result<f64> {
    let mut file = File::open(input_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let mut total_duration = 0.0;
    let mut pos = 0;

    if buffer.len() > 10 && &buffer[0..3] == b"ID3" {
        let size = ((buffer[6] as usize) << 21)
            | ((buffer[7] as usize) << 14)
            | ((buffer[8] as usize) << 7)
            | (buffer[9] as usize);
        pos = 10 + size;
        debug!("skipped ID3v2 tag: {} bytes", 10 + size);
    }

    while pos + 4 <= buffer.len() {
        if buffer[pos] != 0xFF || (buffer[pos + 1] & 0xE0) != 0xE0 {
            pos += 1;
            continue;
        }

        let header = u32::from_be_bytes([
            buffer[pos],
            buffer[pos + 1],
            buffer[pos + 2],
            buffer[pos + 3],
        ]);

        let version = (header >> 19) & 0x3;
        let layer = (header >> 17) & 0x3;
        let bitrate_index = (header >> 12) & 0xF;
        let sample_rate_index = (header >> 10) & 0x3;
        let padding = (header >> 9) & 0x1;

        if version == 1
            || layer == 0
            || bitrate_index == 0
            || bitrate_index == 15
            || sample_rate_index == 3
        {
            pos += 1;
            continue;
        }

        let sample_rates = match version {
            0 => [11025, 12000, 8000],  
            2 => [22050, 24000, 16000], 
            3 => [44100, 48000, 32000], 
            _ => {
                pos += 1;
                continue;
            }
        };
        let sample_rate = sample_rates[sample_rate_index as usize];

        let bitrates = if version == 3 {
            [
                0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
            ]
        } else {
            [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0]
        };
        let bitrate = bitrates[bitrate_index as usize] * 1000;

        if bitrate == 0 {
            pos += 1;
            continue;
        }

        let samples_per_frame = if version == 3 { 1152 } else { 576 };
        let frame_size = (samples_per_frame / 8 * bitrate) / sample_rate + padding as usize;

        total_duration += samples_per_frame as f64 / sample_rate as f64;

        pos += frame_size;
    }

    Ok(total_duration)
}

fn analyze_structure(input_path: &Path) -> Result<()> {
    let file = File::open(input_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    hint.with_extension("mp3");

    let meta_opts: MetadataOptions = Default::default();
    let format_opts: FormatOptions = Default::default();

    let probed = symphonia::default::get_probe().format(&hint, mss, &format_opts, &meta_opts)?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("no audio track found"))?;

    let codec_params = track.codec_params.clone();
    let track_id = track.id;

    println!(
        "{} {:?}",
        "「codec」".cyan().bold(),
        codec_params.codec.to_string().yellow()
    );
    if let Some(sr) = codec_params.sample_rate {
        println!("{} {}Hz", "「sample rate」".cyan().bold(), sr.to_string().yellow());
    }
    if let Some(ch) = codec_params.channels {
        println!(
            "{} {}",
            "「channels」".cyan().bold(),
            ch.count().to_string().yellow()
        );
    }
    println!();

    let decoder_opts: DecoderOptions = Default::default();
    let mut decoder = symphonia::default::get_codecs().make(&codec_params, &decoder_opts)?;

    let mut frame_count = 0;
    let mut valid_frames = 0;
    let mut total_samples = 0u64;
    let mut corrupted_frames = 0;
    let mut frame_infos: Vec<FrameInfo> = Vec::new();
    let mut byte_offset = 0;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::ResetRequired) | Err(SymphoniaError::IoError(_)) => {
                break;
            }
            Err(err) => {
                debug!("error reading packet: {:?}", err);
                continue;
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let entropy = calculate_entropy(&packet.data);
        let size = packet.data.len();

        match decoder.decode(&packet) {
            Ok(decoded) => {
                valid_frames += 1;
                frame_infos.push(FrameInfo {
                    is_valid: true,
                    entropy,
                    size,
                    byte_offset,
                });

                if let AudioBufferRef::F32(buf) = decoded {
                    total_samples += buf.frames() as u64 * buf.spec().channels.count() as u64;
                }
            }
            Err(err) => {
                debug!("failed to decode frame {}: {:?}", frame_count, err);
                corrupted_frames += 1;

                frame_infos.push(FrameInfo {
                    is_valid: false,
                    entropy,
                    size,
                    byte_offset,
                });
            }
        }

        byte_offset += size;
        frame_count += 1;
    }

    println!(
        "{} {}",
        "「total frames」".cyan().bold(),
        frame_count.to_string().yellow()
    );
    println!(
        "{} {}",
        "「valid frames」".green().bold(),
        valid_frames.to_string().yellow()
    );

    if corrupted_frames > 0 {
        println!(
            "{} {}",
            "「corrupted frames」".red().bold(),
            corrupted_frames.to_string().yellow()
        );
    }

    if let Some(sample_rate) = codec_params.sample_rate {
        let channel_count = codec_params
            .channels
            .map(|c| c.count() as f64)
            .unwrap_or(2.0);
        let actual_duration_secs =
            total_samples as f64 / (sample_rate as f64 * channel_count);
        println!(
            "{} {:.3}s",
            "「decoded duration」".cyan().bold(),
            actual_duration_secs.to_string().yellow()
        );
    }

    println!();
    println!("{}", "「generating analysis graph」".magenta().bold());
    let graph_dir = PathBuf::from(".");
    generate_contiguity_graph(&frame_infos, &graph_dir)?;

    Ok(())
}

fn generate_contiguity_graph(frames: &[FrameInfo], output_dir: &Path) -> Result<()> {
    let runs = group_into_runs(frames);
    let output_path = output_dir.join("contiguity_entropy.png");

    let root = BitMapBackend::new(&output_path, (7200, 3600)).into_drawing_area();
    root.fill(&WHITE)?;

    let total_bytes = if let Some(last) = frames.last() {
        last.byte_offset + last.size
    } else {
        return Ok(());
    };

    let max_entropy = frames.iter().map(|f| f.entropy).fold(0.0_f64, f64::max);

    let mut chart = ChartBuilder::on(&root)
        .caption("Frame Contiguity and Entropy by Bytes", ("sans-serif", 120))
        .margin(40)
        .x_label_area_size(160)
        .y_label_area_size(200)
        .build_cartesian_2d(0..total_bytes, 0.0..max_entropy.max(8.0))?;

    chart
        .configure_mesh()
        .x_desc("Byte Position")
        .y_desc("Entropy (bits)")
        .draw()?;

    for run in &runs {
        let color = if run.is_valid {
            GREEN.mix(0.3)
        } else {
            RED.mix(0.3)
        };

        chart.draw_series(std::iter::once(Rectangle::new(
            [(run.start_byte, 0.0), (run.end_byte, run.avg_entropy)],
            color.filled(),
        )))?;
    }

    chart
        .draw_series(LineSeries::new(
            frames
                .iter()
                .filter(|f| f.is_valid)
                .map(|f| (f.byte_offset, f.entropy)),
            &BLUE,
        ))?
        .label("Valid Frame Entropy")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 80, y)], &BLUE));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;
    println!(
        "{} {}",
        "「graph saved」".green().bold(),
        output_path.display().to_string().cyan()
    );

    Ok(())
}
