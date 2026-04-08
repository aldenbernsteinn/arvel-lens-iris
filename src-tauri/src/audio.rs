use crate::error::AppError;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn find_ffmpeg() -> Result<PathBuf, AppError> {
    let output = Command::new("which")
        .arg("ffmpeg")
        .output()
        .map_err(|_| AppError::FfmpegNotFound)?;

    if !output.status.success() {
        return Err(AppError::FfmpegNotFound);
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return Err(AppError::FfmpegNotFound);
    }

    Ok(PathBuf::from(path))
}

pub fn extract_audio(input: &Path, output: &Path) -> Result<(), AppError> {
    let ffmpeg = find_ffmpeg()?;

    let status = Command::new(ffmpeg)
        .args([
            "-i",
            input.to_str().unwrap_or_default(),
            "-vn",
            "-acodec",
            "pcm_s16le",
            "-ar",
            "16000",
            "-ac",
            "1",
            "-y",
            output.to_str().unwrap_or_default(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .map_err(|e| AppError::FfmpegError(format!("Failed to run ffmpeg: {e}")))?;

    if !status.success() {
        return Err(AppError::FfmpegError(format!(
            "ffmpeg exited with code {}",
            status.code().unwrap_or(-1)
        )));
    }

    if !output.exists() {
        return Err(AppError::FfmpegError(
            "ffmpeg produced no output file".into(),
        ));
    }

    Ok(())
}
