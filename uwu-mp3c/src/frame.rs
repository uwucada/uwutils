use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FrameInfo {
    pub is_valid: bool,
    pub entropy: f64,
    pub size: usize,
    pub byte_offset: usize,
}

#[derive(Debug)]
pub struct FrameRun {
    pub start_byte: usize,
    pub end_byte: usize,
    pub is_valid: bool,
    pub avg_entropy: f64,
}

pub fn calculate_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let mut freq: HashMap<u8, usize> = HashMap::new();
    for &byte in data {
        *freq.entry(byte).or_insert(0) += 1;
    }

    let len = data.len() as f64;
    let mut entropy = 0.0;

    for &count in freq.values() {
        let p = count as f64 / len;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }

    entropy
}

pub fn group_into_runs(frames: &[FrameInfo]) -> Vec<FrameRun> {
    if frames.is_empty() {
        return Vec::new();
    }

    let mut runs = Vec::new();
    let mut current_run_start_byte = frames[0].byte_offset;
    let mut current_is_valid = frames[0].is_valid;
    let mut current_entropy_sum = frames[0].entropy;
    let mut current_count = 1;
    let mut current_end_byte = frames[0].byte_offset + frames[0].size;

    for i in 1..frames.len() {
        if frames[i].is_valid == current_is_valid {
            current_entropy_sum += frames[i].entropy;
            current_count += 1;
            current_end_byte = frames[i].byte_offset + frames[i].size;
        } else {
            runs.push(FrameRun {
                start_byte: current_run_start_byte,
                end_byte: current_end_byte,
                is_valid: current_is_valid,
                avg_entropy: current_entropy_sum / current_count as f64,
            });

            current_run_start_byte = frames[i].byte_offset;
            current_is_valid = frames[i].is_valid;
            current_entropy_sum = frames[i].entropy;
            current_count = 1;
            current_end_byte = frames[i].byte_offset + frames[i].size;
        }
    }

    runs.push(FrameRun {
        start_byte: current_run_start_byte,
        end_byte: current_end_byte,
        is_valid: current_is_valid,
        avg_entropy: current_entropy_sum / current_count as f64,
    });

    runs
}
