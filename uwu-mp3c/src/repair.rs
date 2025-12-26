use crate::frame::{calculate_entropy, group_into_runs, FrameInfo};
use anyhow::{anyhow, Result};
use colored::Colorize;
use log::debug;
use plotters::prelude::*;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{CodecParameters, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub fn repair(input_path: &Path, extract_path: &str) -> Result<()> {
    println!(
        "{} {}",
        "「repairing」".cyan().bold(),
        input_path.display().to_string().yellow()
    );
    println!();

    let reported_duration = mp3_duration::from_path(input_path)?;
    println!(
        "{} {:.3}s",
        "「original duration」".green().bold(),
        reported_duration.as_secs_f64()
    );

    let (output_dir, output_filename) = if extract_path.is_empty() {
        let input_parent = input_path.parent().unwrap_or_else(|| Path::new("."));
        let input_stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let output_name = format!("{}-repaired.mp3", input_stem);
        (input_parent.to_path_buf(), output_name)
    } else {
        let dir = PathBuf::from(extract_path);
        let input_stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let output_name = format!("{}-repaired.mp3", input_stem);
        (dir, output_name)
    };

    fs::create_dir_all(&output_dir)?;

    let output_path = output_dir.join(&output_filename);
    let corrupted_frames_dir = output_dir.join("corrupted_frames");
    fs::create_dir_all(&corrupted_frames_dir)?;

    repair_mp3(input_path, &output_path, &corrupted_frames_dir, &output_dir)?;

    let repaired_duration = mp3_duration::from_path(&output_path)?;
    println!();
    println!(
        "{} {:.3}s",
        "「repaired duration」".green().bold(),
        repaired_duration.as_secs_f64()
    );

    Ok(())
}

fn create_xing_header(
    frame_count: u32,
    audio_data_size: u32,
    _codec_params: &CodecParameters,
) -> Vec<u8> {
    let mut xing_frame = Vec::new();

    xing_frame.push(0xFF);
    xing_frame.push(0xFB);
    xing_frame.push(0x50);
    xing_frame.push(0x00);

    for _ in 0..32 {
        xing_frame.push(0x00);
    }

    xing_frame.extend_from_slice(b"Xing");

    xing_frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x03]);

    xing_frame.extend_from_slice(&frame_count.to_be_bytes());

    xing_frame.extend_from_slice(&audio_data_size.to_be_bytes());

    let target_size = 208;
    while xing_frame.len() < target_size {
        xing_frame.push(0x00);
    }

    xing_frame
}

fn repair_mp3(
    input_path: &Path,
    output_path: &Path,
    corrupted_frames_dir: &Path,
    graph_dir: &Path,
) -> Result<()> {
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

    let decoder_opts: DecoderOptions = Default::default();
    let mut decoder = symphonia::default::get_codecs().make(&codec_params, &decoder_opts)?;

    let mut valid_frames: Vec<Vec<u8>> = Vec::new();
    let mut frame_infos: Vec<FrameInfo> = Vec::new();
    let mut frame_count = 0;
    let mut total_samples = 0u64;
    let mut byte_offset = 0;
    let mut corrupted_count = 0;

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
                valid_frames.push(packet.data.to_vec());
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
                debug!("skipping corrupted packet {}: {:?}", frame_count, err);
                corrupted_count += 1;

                frame_infos.push(FrameInfo {
                    is_valid: false,
                    entropy,
                    size,
                    byte_offset,
                });

                let frame_path = corrupted_frames_dir.join(format!("frame_{:06}.bin", frame_count));
                let mut frame_file = File::create(&frame_path)?;
                frame_file.write_all(&packet.data)?;
            }
        }

        byte_offset += size;
        frame_count += 1;
    }

    let valid_frame_count = valid_frames.len();
    let audio_data_size: u32 = valid_frames.iter().map(|f| f.len() as u32).sum();

    println!(
        "{} {}",
        "「total frames」".cyan().bold(),
        frame_count.to_string().yellow()
    );
    println!(
        "{} {}",
        "「valid frames」".green().bold(),
        valid_frame_count.to_string().yellow()
    );

    if corrupted_count > 0 {
        println!(
            "{} {}",
            "「corrupted frames」".red().bold(),
            corrupted_count.to_string().yellow()
        );
        println!(
            "{} {}",
            "「saved to」".cyan().bold(),
            corrupted_frames_dir.display().to_string().yellow()
        );
    }

    debug!(
        "creating Xing header: {} frames, {} bytes",
        valid_frame_count, audio_data_size
    );
    let xing_header = create_xing_header(valid_frame_count as u32, audio_data_size, &codec_params);

    let output_file = File::create(output_path)?;
    let mut writer = BufWriter::new(output_file);

    writer.write_all(&xing_header)?;

    for frame in &valid_frames {
        writer.write_all(frame)?;
    }

    writer.flush()?;

    println!(
        "{} {}",
        "「repaired file」".green().bold(),
        output_path.display().to_string().cyan()
    );

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
    generate_contiguity_graph(&frame_infos, graph_dir)?;

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
