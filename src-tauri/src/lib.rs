// VoiceType - Main Library
mod audio;
mod api;
mod autostart;
mod hotkey;

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::State;
use audio::AudioManager;

pub struct AppState {
    pub audio_manager: Arc<Mutex<AudioManager>>,
}

// History of previously corrected sentences (oldest first), max 5.
// Used as context so LLM correction stays coherent across sentences.
static HISTORY: std::sync::OnceLock<Mutex<Vec<String>>> = std::sync::OnceLock::new();

fn history() -> &'static Mutex<Vec<String>> {
    HISTORY.get_or_init(|| Mutex::new(Vec::new()))
}

fn get_context() -> Vec<String> {
    history().lock().map(|h| h.clone()).unwrap_or_default()
}

fn push_history(text: &str) {
    if text.trim().is_empty() { return; }
    if let Ok(mut h) = history().lock() {
        h.push(text.trim().to_string());
        while h.len() > 5 { h.remove(0); }
        eprintln!("[History] kept {} sentences", h.len());
    }
}

/// Correct text with conversation context, then record result into history.
fn correct_with_history(client: &api::GroqClient, text: &str, use_context: bool) -> String {
    if text.trim().is_empty() { return text.to_string(); }
    let ctx = if use_context { get_context() } else { Vec::new() };
    let result = match client.correct_with_context(text, &ctx) {
        Ok(c) => c,
        Err(e) => { eprintln!("[Correct] failed: {}, using original", e); text.to_string() }
    };
    push_history(&result);
    result
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub groq_api_key: String,
    pub auto_correct: bool,
    pub auto_start: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { groq_api_key: String::new(), auto_correct: true, auto_start: false }
    }
}

fn get_config_path() -> std::path::PathBuf {
    let mut p = std::env::current_dir().unwrap_or_default();
    p.push("voicetype-config.json");
    p
}

fn get_config() -> Result<AppConfig, String> {
    let p = get_config_path();
    if !p.exists() { return Ok(AppConfig::default()); }
    let c = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
    serde_json::from_str(&c).map_err(|e| e.to_string())
}

fn save_config(config: &AppConfig) -> Result<(), String> {
    let c = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(get_config_path(), c).map_err(|e| e.to_string())
}

#[tauri::command]
fn start_recording(state: State<'_, AppState>) -> Result<String, String> {
    eprintln!("[Cmd] start_recording");
    let manager = state.audio_manager.lock().map_err(|e| e.to_string())?;
    manager.start_recording(None)
}

#[tauri::command]
fn stop_recording(state: State<'_, AppState>) -> Result<audio::StopRecordingResult, String> {
    eprintln!("[Cmd] stop_recording");
    let manager = state.audio_manager.lock().map_err(|e| e.to_string())?;
    manager.stop_recording()
}

#[tauri::command]
async fn transcribe(audio_path: String) -> Result<String, String> {
    eprintln!("[Cmd] transcribe: {}", audio_path);
    let config = get_config()?;
    if config.groq_api_key.is_empty() {
        return Err("請先設定 Groq API Key".to_string());
    }

    let client = api::GroqClient::new(config.groq_api_key.clone());
    let text = tokio::task::spawn_blocking(move || client.transcribe(std::path::Path::new(&audio_path)))
        .await.map_err(|e| e.to_string())??;

    let final_text = if config.auto_correct && !text.trim().is_empty() {
        let client2 = api::GroqClient::new(config.groq_api_key.clone());
        let t = text.clone();
        tokio::task::spawn_blocking(move || correct_with_history(&client2, &t, true))
            .await.map_err(|e| e.to_string())?
    } else {
        text
    };

    // 每句結尾加一個空格，連續貼上時句子之間有分隔
    let paste_text = format!("{} ", final_text.trim_end());
    {
        let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        cb.set_text(&paste_text).map_err(|e| e.to_string())?;
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    hotkey::simulate_paste()?;
    Ok(final_text)
}

#[tauri::command]
fn get_config_command() -> Result<AppConfig, String> { get_config() }

#[tauri::command]
fn set_config(config: AppConfig) -> Result<(), String> {
    // Apply auto-start setting to Windows Registry
    if let Err(e) = autostart::set_autostart(config.auto_start) {
        eprintln!("[Config] autostart apply failed: {}", e);
        return Err(e);
    }
    save_config(&config)
}

#[tauri::command]
fn get_autostart() -> bool { autostart::get_autostart() }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    eprintln!("[VoiceType] Starting...");

    // Ensure registry auto-start matches saved config (e.g. exe moved after update)
    if let Ok(cfg) = get_config() {
        if cfg.auto_start {
            if let Err(e) = autostart::set_autostart(true) {
                eprintln!("[VoiceType] autostart ensure failed: {}", e);
            }
        }
    }

    let audio_manager = AudioManager::new().expect("Cannot create AudioManager");
    let audio_arc = Arc::new(Mutex::new(audio_manager));

    // Start keyboard hook and get the channel
    let rx = hotkey::start_hook();

    // Clone Arc for the hotkey processing thread
    let am = audio_arc.clone();

    // Spawn a thread that processes hotkey events directly
    std::thread::spawn(move || {
        eprintln!("[Pipeline] Hotkey listener started");
        let mut is_recording = false;

        for pressed in rx.iter() {
            eprintln!("[Pipeline] F9 event: pressed={}", pressed);

            if pressed && !is_recording {
                // Start recording
                let manager = am.lock().map_err(|e| e.to_string());
                if let Ok(m) = manager {
                    match m.start_recording(None) {
                        Ok(id) => {
                            eprintln!("[Pipeline] Recording started: {}", id);
                            is_recording = true;
                        }
                        Err(e) => eprintln!("[Pipeline] Start failed: {}", e),
                    }
                }
            } else if !pressed && is_recording {
                // Stop recording
                let manager = am.lock().map_err(|e| e.to_string());
                if let Ok(m) = manager {
                    match m.stop_recording() {
                        Ok(result) => {
                            eprintln!("[Pipeline] Stopped: {}s, path: {}", result.duration, result.audio_path);
                            is_recording = false;

                            // Transcribe and paste
                            let path = result.audio_path;
                            let config = match get_config() {
                                Ok(c) => c,
                                Err(e) => { eprintln!("[Pipeline] Config error: {}", e); continue; }
                            };
                            if config.groq_api_key.is_empty() {
                                eprintln!("[Pipeline] No API key");
                                continue;
                            }

                            // Transcribe
                            let client = api::GroqClient::new(config.groq_api_key.clone());
                            let text = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                client.transcribe(std::path::Path::new(&path))
                            })) {
                                Ok(Ok(t)) => t,
                                Ok(Err(e)) => { eprintln!("[Pipeline] Transcribe error: {}", e); continue; }
                                Err(_) => { eprintln!("[Pipeline] Transcribe panicked"); continue; }
                            };
                            eprintln!("[Pipeline] Transcribed: {}", text);

                            // Correct with conversation context
                            let final_text = if config.auto_correct {
                                let client2 = api::GroqClient::new(config.groq_api_key.clone());
                                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    correct_with_history(&client2, &text, true)
                                })) {
                                    Ok(c) => c,
                                    Err(_) => { eprintln!("[Pipeline] Correct panicked"); text }
                                }
                            } else {
                                text
                            };

                            // Copy to clipboard (每句結尾加一個空格，連續貼上時有分隔)
                            let paste_text = format!("{} ", final_text.trim_end());
                            if let Ok(mut cb) = arboard::Clipboard::new() {
                                let _ = cb.set_text(&paste_text);
                            }

                            // Paste
                            if let Err(e) = hotkey::simulate_paste() {
                                eprintln!("[Pipeline] Paste error: {}", e);
                            } else {
                                eprintln!("[Pipeline] Pasted!");
                            }
                        }
                        Err(e) => {
                            eprintln!("[Pipeline] Stop error: {}", e);
                            is_recording = false;
                        }
                    }
                }
            }
        }
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            audio_manager: audio_arc,
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            transcribe,
            get_config_command,
            set_config,
            get_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
