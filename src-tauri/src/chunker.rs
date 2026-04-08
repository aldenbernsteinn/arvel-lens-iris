use crate::error::AppError;
use std::path::Path;

const SAMPLE_RATE: u32 = 16_000;
const CHUNK_DURATION_SECS: f32 = 240.0; // 4 minutes
const OVERLAP_SECS: f32 = 5.0;

const CHUNK_SAMPLES: usize = (SAMPLE_RATE as f32 * CHUNK_DURATION_SECS) as usize;
const OVERLAP_SAMPLES: usize = (SAMPLE_RATE as f32 * OVERLAP_SECS) as usize;

pub struct AudioChunk {
    pub samples: Vec<f32>,
    pub start_time: f32,
    pub _index: usize,
    pub total_chunks: usize,
}

pub fn load_and_chunk(wav_path: &Path) -> Result<Vec<AudioChunk>, AppError> {
    let mut reader = hound::WavReader::open(wav_path)
        .map_err(|e| AppError::Other(format!("Cannot read WAV: {e}")))?;

    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max = (1 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max)
                .collect()
        }
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .filter_map(|s| s.ok())
            .collect(),
    };

    if samples.len() <= CHUNK_SAMPLES {
        return Ok(vec![AudioChunk {
            samples,
            start_time: 0.0,
            _index: 0,
            total_chunks: 1,
        }]);
    }

    let step = CHUNK_SAMPLES - OVERLAP_SAMPLES;
    let mut chunks = Vec::new();
    let mut offset = 0usize;

    while offset < samples.len() {
        let end = (offset + CHUNK_SAMPLES).min(samples.len());
        let chunk_samples = samples[offset..end].to_vec();
        let start_time = offset as f32 / SAMPLE_RATE as f32;

        chunks.push(AudioChunk {
            samples: chunk_samples,
            start_time,
            _index: chunks.len(),
            total_chunks: 0, // filled below
        });

        if end >= samples.len() {
            break;
        }
        offset += step;
    }

    let total = chunks.len();
    for chunk in &mut chunks {
        chunk.total_chunks = total;
    }

    Ok(chunks)
}
