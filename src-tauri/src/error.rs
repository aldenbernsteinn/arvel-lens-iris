use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Model not found at {0}")]
    ModelNotFound(String),

    #[error("Model download failed: {0}")]
    DownloadError(String),

    #[error("ffmpeg not found. Install with: brew install ffmpeg")]
    FfmpegNotFound,

    #[error("Audio extraction failed: {0}")]
    FfmpegError(String),

    #[error("Transcription failed: {0}")]
    TranscriptionError(String),

    #[error("Model not loaded")]
    ModelNotLoaded,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
