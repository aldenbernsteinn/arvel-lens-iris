mod audio;
mod chunker;
mod commands;
mod error;
mod model_manager;
mod srt;
mod transcriber;

use model_manager::ModelManager;
use std::path::PathBuf;
use std::sync::Mutex;
use transcriber::TranscriberWorker;

pub struct AppState {
    pub model_manager: ModelManager,
    pub worker: Mutex<Option<TranscriberWorker>>,
    pub temp_dir: PathBuf,
}

pub fn run() {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.mactranscribe.app");

    let temp_dir = data_dir.join("temp");
    let model_manager = ModelManager::new(&data_dir);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            model_manager,
            worker: Mutex::new(None),
            temp_dir,
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_model_status,
            commands::download_model,
            commands::check_ffmpeg,
            commands::transcribe_file,
            commands::export_srt,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MacTranscribe");
}
