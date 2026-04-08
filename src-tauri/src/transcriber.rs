use crate::chunker;
use crate::error::AppError;
use parakeet_rs::{ParakeetTDT, TimestampMode, Transcriber};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

#[derive(Clone, Serialize, serde::Deserialize)]
pub struct TimedSegment {
    pub text: String,
    pub start: f32,
    pub end: f32,
}

#[derive(Clone, Serialize)]
pub struct TranscriptResult {
    pub text: String,
    pub segments: Vec<TimedSegment>,
    pub duration_secs: f32,
}

#[derive(Clone, Serialize)]
pub struct TranscriptionProgress {
    pub stage: String,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub percent: f32,
    pub partial_text: String,
}

struct TranscribeRequest {
    samples: Vec<f32>,
    timestamp_mode: TimestampMode,
    response: mpsc::Sender<Result<ChunkResult, String>>,
}

pub(crate) struct ChunkResult {
    text: String,
    segments: Vec<TimedSegment>,
}

pub struct TranscriberWorker {
    sender: mpsc::Sender<TranscribeRequest>,
}

impl TranscriberWorker {
    pub fn spawn(model_path: PathBuf) -> Result<Self, AppError> {
        let (tx, rx) = mpsc::channel::<TranscribeRequest>();

        thread::spawn(move || {
            let mut model = match ParakeetTDT::from_pretrained(&model_path, None) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Failed to load model: {e}");
                    return;
                }
            };

            for req in rx {
                let result = model.transcribe_samples(
                    req.samples,
                    16000,
                    1,
                    Some(req.timestamp_mode),
                );

                let mapped = match result {
                    Ok(r) => {
                        let segments: Vec<TimedSegment> = r
                            .tokens
                            .iter()
                            .map(|t| TimedSegment {
                                text: t.text.clone(),
                                start: t.start,
                                end: t.end,
                            })
                            .collect();
                        Ok(ChunkResult {
                            text: r.text,
                            segments,
                        })
                    }
                    Err(e) => Err(format!("{e}")),
                };

                let _ = req.response.send(mapped);
            }
        });

        Ok(Self { sender: tx })
    }

    pub(crate) fn transcribe_chunk(
        &self,
        samples: Vec<f32>,
        mode: TimestampMode,
    ) -> Result<ChunkResult, AppError> {
        let (tx, rx) = mpsc::channel();
        self.sender
            .send(TranscribeRequest {
                samples,
                timestamp_mode: mode,
                response: tx,
            })
            .map_err(|_| AppError::ModelNotLoaded)?;

        rx.recv()
            .map_err(|_| AppError::TranscriptionError("Worker died".into()))?
            .map_err(|e| AppError::TranscriptionError(e))
    }
}

pub fn transcribe_wav(
    worker: &TranscriberWorker,
    wav_path: &Path,
    mode: TimestampMode,
    progress: &tauri::ipc::Channel<TranscriptionProgress>,
) -> Result<TranscriptResult, AppError> {
    let chunks = chunker::load_and_chunk(wav_path)?;
    let total = chunks.len();
    let mut all_text = String::new();
    let mut all_segments: Vec<TimedSegment> = Vec::new();

    for (i, chunk) in chunks.into_iter().enumerate() {
        let _ = progress.send(TranscriptionProgress {
            stage: "transcribing".into(),
            chunk_index: i,
            total_chunks: total,
            percent: (i as f32 / total as f32) * 100.0,
            partial_text: all_text.clone(),
        });

        let offset = chunk.start_time;
        let result = worker.transcribe_chunk(chunk.samples, mode)?;

        if !all_text.is_empty() && !result.text.is_empty() {
            all_text.push(' ');
        }
        all_text.push_str(&result.text);

        for mut seg in result.segments {
            seg.start += offset;
            seg.end += offset;
            all_segments.push(seg);
        }
    }

    let duration = all_segments
        .last()
        .map(|s| s.end)
        .unwrap_or(0.0);

    let _ = progress.send(TranscriptionProgress {
        stage: "complete".into(),
        chunk_index: total,
        total_chunks: total,
        percent: 100.0,
        partial_text: all_text.clone(),
    });

    Ok(TranscriptResult {
        text: all_text,
        segments: all_segments,
        duration_secs: duration,
    })
}
