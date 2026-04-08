use crate::error::AppError;
use futures_util::StreamExt;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::ipc::Channel;

const HF_BASE: &str =
    "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v2-onnx/resolve/main";

const MODEL_FILES: &[&str] = &[
    "encoder-model.onnx",
    "encoder-model.onnx.data",
    "decoder_joint-model.onnx",
    "vocab.txt",
];

#[derive(Clone, Serialize)]
pub struct DownloadProgress {
    pub file: String,
    pub file_index: usize,
    pub total_files: usize,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
    pub percent: f32,
}

pub struct ModelManager {
    model_dir: PathBuf,
}

impl ModelManager {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            model_dir: app_data_dir.join("models").join("parakeet-tdt-0.6b-v2"),
        }
    }

    pub fn model_dir(&self) -> &Path {
        &self.model_dir
    }

    pub fn is_ready(&self) -> bool {
        MODEL_FILES
            .iter()
            .all(|f| self.model_dir.join(f).exists())
    }

    pub async fn download(&self, progress: Channel<DownloadProgress>) -> Result<(), AppError> {
        std::fs::create_dir_all(&self.model_dir)
            .map_err(|e| AppError::DownloadError(format!("Cannot create model dir: {e}")))?;

        let client = reqwest::Client::new();

        for (i, filename) in MODEL_FILES.iter().enumerate() {
            let dest = self.model_dir.join(filename);
            if dest.exists() {
                let _ = progress.send(DownloadProgress {
                    file: filename.to_string(),
                    file_index: i,
                    total_files: MODEL_FILES.len(),
                    bytes_downloaded: 0,
                    total_bytes: Some(0),
                    percent: 100.0,
                });
                continue;
            }

            let url = format!("{HF_BASE}/{filename}");
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| AppError::DownloadError(format!("Request failed for {filename}: {e}")))?;

            if !resp.status().is_success() {
                return Err(AppError::DownloadError(format!(
                    "HTTP {} for {filename}",
                    resp.status()
                )));
            }

            let total = resp.content_length();
            let tmp = dest.with_extension("part");
            let mut file = tokio::fs::File::create(&tmp)
                .await
                .map_err(|e| AppError::DownloadError(format!("Cannot create file: {e}")))?;

            let mut downloaded: u64 = 0;
            let mut stream = resp.bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk
                    .map_err(|e| AppError::DownloadError(format!("Stream error: {e}")))?;
                tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                    .await
                    .map_err(|e| AppError::DownloadError(format!("Write error: {e}")))?;

                downloaded += chunk.len() as u64;
                let pct = total.map(|t| (downloaded as f32 / t as f32) * 100.0).unwrap_or(0.0);

                let _ = progress.send(DownloadProgress {
                    file: filename.to_string(),
                    file_index: i,
                    total_files: MODEL_FILES.len(),
                    bytes_downloaded: downloaded,
                    total_bytes: total,
                    percent: pct,
                });
            }

            drop(file);
            tokio::fs::rename(&tmp, &dest)
                .await
                .map_err(|e| AppError::DownloadError(format!("Rename error: {e}")))?;
        }

        Ok(())
    }
}
