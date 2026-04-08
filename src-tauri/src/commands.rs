use crate::error::AppError;
use crate::model_manager::DownloadProgress;
use crate::srt;
use crate::transcriber::{self, TranscriberWorker, TranscriptionProgress, TranscriptResult};
use crate::{audio, AppState};
use parakeet_rs::TimestampMode;
use std::path::PathBuf;
use tauri::ipc::Channel;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn check_model_status(state: State<'_, AppState>) -> Result<bool, AppError> {
    Ok(state.model_manager.is_ready())
}

#[tauri::command]
pub async fn download_model(
    state: State<'_, AppState>,
    progress: Channel<DownloadProgress>,
) -> Result<(), AppError> {
    state.model_manager.download(progress).await
}

#[tauri::command]
pub async fn check_ffmpeg() -> Result<bool, AppError> {
    Ok(audio::find_ffmpeg().is_ok())
}

#[tauri::command]
pub async fn transcribe_file(
    path: String,
    timestamp_mode: String,
    progress: Channel<TranscriptionProgress>,
    state: State<'_, AppState>,
) -> Result<TranscriptResult, AppError> {
    let input_path = PathBuf::from(&path);
    let mode = match timestamp_mode.as_str() {
        "sentences" => TimestampMode::Sentences,
        "tokens" => TimestampMode::Tokens,
        _ => TimestampMode::Words,
    };

    // Extract audio
    let _ = progress.send(TranscriptionProgress {
        stage: "extracting_audio".into(),
        chunk_index: 0,
        total_chunks: 0,
        percent: 0.0,
        partial_text: String::new(),
    });

    let temp_dir = state.temp_dir.clone();
    std::fs::create_dir_all(&temp_dir)?;
    let wav_path = temp_dir.join(format!("{}.wav", Uuid::new_v4()));

    // Run ffmpeg in a blocking task
    let input_clone = input_path.clone();
    let wav_clone = wav_path.clone();
    tokio::task::spawn_blocking(move || audio::extract_audio(&input_clone, &wav_clone))
        .await
        .map_err(|e| AppError::Other(format!("Task join error: {e}")))??;

    // Ensure worker is ready
    {
        let mut worker_guard = state.worker.lock().map_err(|e| {
            AppError::TranscriptionError(format!("Lock error: {e}"))
        })?;

        if worker_guard.is_none() {
            let model_dir = state.model_manager.model_dir().to_path_buf();
            let w = TranscriberWorker::spawn(model_dir)?;
            *worker_guard = Some(w);
        }
    }

    // Transcribe
    let worker_guard = state.worker.lock().map_err(|e| {
        AppError::TranscriptionError(format!("Lock error: {e}"))
    })?;
    let worker = worker_guard.as_ref().ok_or(AppError::ModelNotLoaded)?;

    let wav_for_transcribe = wav_path.clone();
    let result = transcriber::transcribe_wav(worker, &wav_for_transcribe, mode, &progress)?;

    // Cleanup temp WAV
    let _ = std::fs::remove_file(&wav_path);

    Ok(result)
}

#[tauri::command]
pub fn export_srt(segments: Vec<crate::transcriber::TimedSegment>) -> String {
    srt::to_srt(&segments)
}
