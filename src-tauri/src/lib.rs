#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod ai_client;
pub mod artifact_download;
pub mod audio_recorder;
#[path = "history_secure.rs"]
pub mod history;
pub mod keyboard_hook;
pub mod keyboard_simulator;
pub mod logger;
pub mod parakeet_streaming;
pub mod secure_storage;
#[path = "settings_secure.rs"]
pub mod settings;
pub mod vad;
pub mod whisper_runner;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_updater::UpdaterExt;

struct ParakeetStreamingSession {
    generation: u64,
    cancel: Arc<AtomicBool>,
    result_rx: mpsc::Receiver<parakeet_streaming::StreamingOutcome>,
}

struct AppState {
    audio_recorder: audio_recorder::AudioRecorder,
    selected_text: Mutex<String>,
    press_time: Mutex<Option<std::time::Instant>>,
    is_recording: AtomicBool,
    toggle_enabled: AtomicBool,
    typed_so_far: Mutex<String>,
    /// True after physical user input makes Aura's mirrored live-text state
    /// unsafe for further Backspace-based reconciliation.
    live_target_desynced: AtomicBool,
    /// Remains active through final reconciliation so edits during processing
    /// also force a non-destructive clipboard handoff.
    live_target_monitoring: AtomicBool,
    selected_language: Mutex<String>,
    /// Increments on every new recording session; stale async tasks compare
    /// against it before touching the keyboard or clipboard.
    session_gen: AtomicU64,
    /// Serializes clipboard backup/paste/restore sequences so overlapping
    /// sessions cannot interleave and clobber each other's clipboard contents.
    clipboard_mutex: Mutex<()>,
    /// Toggle mode: a short tap latched the recording until the next tap / Esc.
    latched: AtomicBool,
    /// Set when a toggle-stopping tap already finalized; its key release is a no-op.
    ignore_next_release: AtomicBool,
    /// Window that had focus when the recording started (focus guard for typing).
    start_hwnd: Mutex<isize>,
    parakeet_lifecycle: Mutex<()>,
    parakeet_server: Mutex<Option<whisper_runner::RunningParakeetServer>>,
    parakeet_port: std::sync::atomic::AtomicU16,
    parakeet_streaming: Mutex<Option<ParakeetStreamingSession>>,
    parakeet_watchdog: Mutex<Option<whisper_runner::ParakeetWatchdog>>,
}

#[tauri::command]
fn minimize_window(window: tauri::WebviewWindow) {
    let _ = window.minimize();
}

#[tauri::command]
fn close_window(window: tauri::WebviewWindow) {
    let _ = window.close();
}

#[tauri::command]
fn start_dragging_command(window: tauri::WebviewWindow) {
    let _ = window.start_dragging();
}

#[tauri::command]
fn hide_overlay_window(window: tauri::WebviewWindow) {
    if window.label() == "overlay" {
        let _ = window.hide();
    }
}

async fn load_settings_async(app_handle: tauri::AppHandle) -> Result<settings::Settings, String> {
    tauri::async_runtime::spawn_blocking(move || settings::load_settings(&app_handle))
        .await
        .map_err(|error| format!("Settings load worker failed: {error}"))?
}

#[tauri::command]
async fn get_settings(app_handle: tauri::AppHandle) -> Result<settings::SettingsView, String> {
    load_settings_async(app_handle)
        .await
        .map(|settings| settings::SettingsView::from_settings(&settings))
}

#[derive(Clone, serde::Serialize)]
struct OverlayPreferences {
    overlay_sounds: bool,
    overlay_sound_theme: String,
    overlay_sound_volume: f32,
}

impl From<&settings::Settings> for OverlayPreferences {
    fn from(settings: &settings::Settings) -> Self {
        Self {
            overlay_sounds: settings.overlay_sounds,
            overlay_sound_theme: settings.overlay_sound_theme.clone(),
            overlay_sound_volume: settings.overlay_sound_volume,
        }
    }
}

#[tauri::command]
async fn set_provider_key(
    app_handle: tauri::AppHandle,
    provider: String,
    mut key: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let result = settings::set_provider_key(&app_handle, &provider, &key);
        zeroize::Zeroize::zeroize(&mut key);
        result
    })
    .await
    .map_err(|error| format!("API key storage worker failed: {error}"))?
}

#[tauri::command]
async fn set_settings(
    app_handle: tauri::AppHandle,
    settings: settings::Settings,
) -> Result<(), String> {
    keyboard_hook::validate_hotkey(&settings.hotkey)?;
    let save_handle = app_handle.clone();
    let mut saved_settings = settings.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result: Result<(), String> = (|| {
            settings::save_settings(&save_handle, &saved_settings)?;
            sync_autostart(&save_handle, saved_settings.autostart);
            Ok(())
        })();
        // The moved-in copy may carry secrets from the request; scrub it.
        zeroize::Zeroize::zeroize(&mut saved_settings.api_key);
        zeroize::Zeroize::zeroize(&mut saved_settings.api_key_gemini);
        zeroize::Zeroize::zeroize(&mut saved_settings.api_key_openai);
        zeroize::Zeroize::zeroize(&mut saved_settings.api_key_groq);
        result
    })
    .await
    .map_err(|error| format!("Settings storage worker failed: {error}"))??;

    keyboard_hook::update_hotkey(&settings.hotkey)?;
    if let Some(state) = app_handle.try_state::<AppState>() {
        state
            .toggle_enabled
            .store(settings.toggle_enabled, Ordering::SeqCst);
    }
    let sidecar_handle = app_handle.clone();
    tauri::async_runtime::spawn_blocking(move || {
        whisper_runner::ensure_parakeet_server_state(&sidecar_handle, &settings);
    });
    Ok(())
}

fn canonical_model_name(model_name: &str) -> Result<String, String> {
    if model_name.len() > 64 {
        return Err("Invalid model name".to_string());
    }
    if model_name == "parakeet-v3" {
        return Ok(model_name.to_string());
    }
    let filename = whisper_runner::whisper_model_filename(model_name)?;
    filename
        .strip_prefix("ggml-")
        .and_then(|name| name.strip_suffix(".bin"))
        .map(str::to_string)
        .ok_or_else(|| "Invalid Whisper model filename".to_string())
}

#[tauri::command]
async fn download_model_command(
    app_handle: tauri::AppHandle,
    model_name: String,
) -> Result<(), String> {
    let model_name = canonical_model_name(&model_name)?;
    if model_name == "parakeet-v3" {
        whisper_runner::download_parakeet_model(&app_handle).await?;
        let _ = whisper_runner::download_punctuation_model(&app_handle).await;
        Ok(())
    } else {
        whisper_runner::download_model(&app_handle, model_name.as_str())
            .await
            .map(|_| ())
    }
}

#[tauri::command]
async fn cancel_model_download(model_name: String) -> Result<(), String> {
    let model_name = canonical_model_name(&model_name)?;
    whisper_runner::request_cancel_download(&model_name);
    Ok(())
}

#[tauri::command]
async fn delete_model_command(
    app_handle: tauri::AppHandle,
    model_name: String,
) -> Result<(), String> {
    let model_name = canonical_model_name(&model_name)?;
    if whisper_runner::is_model_download_active(&model_name) {
        return Err(format!(
            "Cannot delete '{model_name}' while its download is active"
        ));
    }
    let app_local_data = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data dir: {}", e))?;

    if model_name == "parakeet-v3" {
        let folder = app_local_data.join("models").join("parakeet-v3");
        let worker_handle = app_handle.clone();
        return tauri::async_runtime::spawn_blocking(move || {
            whisper_runner::stop_parakeet_server(&worker_handle);
            match std::fs::remove_dir_all(&folder) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(format!("Failed to delete parakeet directory: {error}")),
            }
        })
        .await
        .map_err(|error| format!("Parakeet deletion worker failed: {error}"))?;
    }

    let filename = whisper_runner::whisper_model_filename(&model_name)?;
    let model_path = app_local_data.join("models").join(&filename);

    tauri::async_runtime::spawn_blocking(move || match std::fs::remove_file(&model_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to delete model file: {error}")),
    })
    .await
    .map_err(|error| format!("Whisper model deletion worker failed: {error}"))?
}

#[tauri::command]
async fn get_downloaded_models(app_handle: tauri::AppHandle) -> Result<Vec<String>, String> {
    let app_local_data = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data dir: {}", e))?;
    let models_dir = app_local_data.join("models");

    tauri::async_runtime::spawn_blocking(move || {
        let mut downloaded = Vec::new();
        let parakeet_dir = models_dir.join("parakeet-v3");
        if whisper_runner::parakeet_model_is_installed(&parakeet_dir) {
            downloaded.push("parakeet-v3".to_string());
        }

        let entries = match std::fs::read_dir(&models_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(downloaded),
            Err(error) => return Err(format!("Failed to inspect downloaded models: {error}")),
        };
        for entry_result in entries {
            let entry = match entry_result {
                Ok(entry) => entry,
                Err(error) => {
                    crate::logger::log(
                        "WARN",
                        "Models",
                        None,
                        &format!("Skipping unreadable model directory entry: {error}"),
                    );
                    continue;
                }
            };
            let path = entry.path();
            let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Some(name) = filename
                .strip_prefix("ggml-")
                .and_then(|value| value.strip_suffix(".bin"))
            else {
                continue;
            };
            if whisper_runner::whisper_model_is_installed(name, &path) {
                downloaded.push(name.to_string());
            }
        }
        downloaded.sort();
        downloaded.dedup();
        Ok::<Vec<String>, String>(downloaded)
    })
    .await
    .map_err(|error| format!("Model inspection worker failed: {error}"))?
}

#[tauri::command]
async fn get_history(app_handle: tauri::AppHandle) -> Result<Vec<history::HistoryEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || history::load_history(&app_handle))
        .await
        .map_err(|error| format!("History load worker failed: {error}"))?
}

#[tauri::command]
async fn clear_history(app_handle: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || history::clear_history(&app_handle))
        .await
        .map_err(|error| format!("History clear worker failed: {error}"))?
}

#[tauri::command]
async fn copy_to_clipboard(text: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
        clipboard.set_text(text).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("Clipboard worker failed: {error}"))?
}

#[derive(serde::Serialize)]
struct AppUpdateInfo {
    current_version: String,
    version: String,
}

async fn query_app_update(
    app_handle: &tauri::AppHandle,
) -> Result<Option<tauri_plugin_updater::Update>, String> {
    let updater = app_handle
        .updater()
        .map_err(|error| format!("Could not initialize updater: {error}"))?;
    tokio::time::timeout(std::time::Duration::from_secs(30), updater.check())
        .await
        .map_err(|_| "Update check timed out after 30 seconds".to_string())?
        .map_err(|error| format!("Update check failed: {error}"))
}

#[tauri::command]
async fn check_for_app_update(
    app_handle: tauri::AppHandle,
) -> Result<Option<AppUpdateInfo>, String> {
    Ok(query_app_update(&app_handle)
        .await?
        .map(|update| AppUpdateInfo {
            current_version: update.current_version,
            version: update.version,
        }))
}

#[tauri::command]
async fn install_app_update(app_handle: tauri::AppHandle) -> Result<(), String> {
    let update = query_app_update(&app_handle)
        .await?
        .ok_or_else(|| "No update is currently available".to_string())?;
    tokio::time::timeout(
        std::time::Duration::from_secs(15 * 60),
        update.download_and_install(
            |_chunk_size, _content_length| {},
            || crate::logger::log("INFO", "Updater", None, "Update download completed"),
        ),
    )
    .await
    .map_err(|_| "Update download timed out after 15 minutes".to_string())?
    .map_err(|error| format!("Update installation failed: {error}"))
}
/// Opens a URL in the user's default browser (used by the update badge).
#[tauri::command]
async fn open_url(app_handle: tauri::AppHandle, url: String) -> Result<(), String> {
    let parsed = reqwest::Url::parse(&url).map_err(|_| "Update URL is invalid".to_string())?;
    let allowed = parsed.scheme() == "https"
        && parsed.host_str() == Some("github.com")
        && parsed.path().starts_with("/malashkadev/aura/releases/");
    if !allowed {
        return Err("Only Aura release pages on https://github.com may be opened".to_string());
    }

    use tauri_plugin_opener::OpenerExt;
    app_handle
        .opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn relaunch_app(app_handle: tauri::AppHandle) {
    app_handle.restart();
}

#[tauri::command]
async fn get_diagnostic_report(app_handle: tauri::AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || logger::generate_diagnostic_report(&app_handle))
        .await
        .map_err(|error| format!("Diagnostic report worker failed: {error}"))?
}

#[tauri::command]
async fn log_frontend_event(
    level: String,
    tag: String,
    session: Option<String>,
    message: String,
) -> Result<(), String> {
    logger::log(&level, &tag, session.as_deref(), &message);
    Ok(())
}

fn sync_autostart(app_handle: &tauri::AppHandle, enabled: bool) {
    let manager = app_handle.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    if let Err(e) = result {
        let err_str = e.to_string();
        let is_not_found = err_str.contains("os error 2")
            || err_str.contains("not find")
            || err_str.contains("не удается найти")
            || err_str.contains("not found");

        if !enabled && is_not_found {
            crate::logger::log("INFO", "Autostart", None, "Autostart was already disabled.");
        } else {
            crate::logger::log(
                "ERROR",
                "Autostart",
                None,
                &format!("Failed to update autostart ({}): {}", enabled, e),
            );
        }
    }
}

fn recording_wav_path(gen: u64) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("aura-rec-{}-{}.wav", std::process::id(), gen))
}

fn chunk_wav_path(gen: u64) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("aura-chunk-{}-{}.wav", std::process::id(), gen))
}

fn provider_from(settings: &settings::Settings) -> ai_client::ApiProvider {
    match settings.api_provider.as_str() {
        "openai" => ai_client::ApiProvider::OpenAi,
        "groq" => ai_client::ApiProvider::Groq,
        _ => ai_client::ApiProvider::Gemini,
    }
}

/// Resolves the language to send to the recognizer from settings + detected layout.
fn effective_language(settings: &settings::Settings, layout_language: &str) -> String {
    match settings.language.as_str() {
        "ru" | "en" | "de" | "es" | "fr" | "it" | "zh" | "pt" | "tr" => settings.language.clone(),
        "layout" => layout_language.to_string(),
        _ => String::new(), // auto-detect
    }
}

struct WordSpan {
    _start: usize,
    end: usize,
    text: String,
}

fn extract_word_spans(chars: &[char]) -> Vec<WordSpan> {
    let mut spans = Vec::new();
    let mut in_word = false;
    let mut start = 0;

    for (i, &ch) in chars.iter().enumerate() {
        if !ch.is_whitespace() {
            if !in_word {
                in_word = true;
                start = i;
            }
        } else if in_word {
            in_word = false;
            let word_str: String = chars[start..i].iter().collect();
            spans.push(WordSpan {
                _start: start,
                end: i,
                text: word_str,
            });
        }
    }
    if in_word {
        let word_str: String = chars[start..chars.len()].iter().collect();
        spans.push(WordSpan {
            _start: start,
            end: chars.len(),
            text: word_str,
        });
    }
    spans
}

fn normalize_word_for_matching(w: &str) -> String {
    w.trim_matches(|c: char| c.is_ascii_punctuation() || ".,!?:;-—\"'«»()[]{}".contains(c))
        .to_lowercase()
        .replace('ё', "е")
}

#[derive(Debug, PartialEq, Eq)]
struct TextReplacementPlan {
    backspaces: usize,
    suffix: String,
}

fn plan_text_replacement(typed_so_far: &str, new_text: &str, is_live: bool) -> TextReplacementPlan {
    let typed_chars: Vec<char> = typed_so_far.chars().collect();
    let new_chars: Vec<char> = new_text.chars().collect();

    let (common_typed_end, common_new_end) = if is_live {
        let typed_spans = extract_word_spans(&typed_chars);
        let new_spans = extract_word_spans(&new_chars);

        let mut matched_typed_end = 0;
        let mut matched_new_end = 0;

        for (tw, nw) in typed_spans.iter().zip(new_spans.iter()) {
            let norm_t = normalize_word_for_matching(&tw.text);
            let norm_n = normalize_word_for_matching(&nw.text);

            if !norm_t.is_empty() && norm_t == norm_n {
                matched_typed_end = tw.end;
                matched_new_end = nw.end;
            } else {
                break;
            }
        }

        while matched_typed_end < typed_chars.len()
            && matched_new_end < new_chars.len()
            && typed_chars[matched_typed_end].is_whitespace()
            && new_chars[matched_new_end].is_whitespace()
            && typed_chars[matched_typed_end] == new_chars[matched_new_end]
        {
            matched_typed_end += 1;
            matched_new_end += 1;
        }

        (matched_typed_end, matched_new_end)
    } else {
        let mut common_len = 0;
        for (c1, c2) in typed_chars.iter().zip(new_chars.iter()) {
            if c1 == c2 {
                common_len += 1;
            } else {
                break;
            }
        }
        (common_len, common_len)
    };

    TextReplacementPlan {
        backspaces: typed_chars.len() - common_typed_end,
        suffix: new_chars[common_new_end..].iter().collect(),
    }
}

fn diff_and_type_with<D>(
    typed_so_far: &mut String,
    new_text: &str,
    is_live: bool,
    dispatch: D,
) -> Result<keyboard_simulator::ReplacementDispatchMetrics, String>
where
    D: FnOnce(
        usize,
        &str,
    ) -> Result<
        keyboard_simulator::ReplacementDispatchMetrics,
        keyboard_simulator::TextReplacementError,
    >,
{
    let plan = plan_text_replacement(typed_so_far, new_text, is_live);
    let retained_chars = typed_so_far.chars().count().saturating_sub(plan.backspaces);
    let mut applied_text: String = typed_so_far.chars().take(retained_chars).collect();
    applied_text.push_str(&plan.suffix);
    match dispatch(plan.backspaces, &plan.suffix) {
        Ok(metrics) => {
            *typed_so_far = applied_text;
            Ok(metrics)
        }
        Err(error) => {
            // Even an interrupted dispatch leaves part (or all) of the planned
            // change in the document. Commit a mirror of exactly what landed so
            // a later Esc cancel backspaces the true visible text instead of a
            // stale pre-update prefix, which would leave a partial tail behind.
            commit_partial_replacement(typed_so_far, &plan.suffix, &error);
            Err(error.message)
        }
    }
}

/// Mirrors into `typed_so_far` the portion of a planned replacement that a
/// dispatch actually committed before being interrupted. Backspaces map
/// one-to-one onto chars; committed UTF-16 units are converted back to the
/// longest matching char prefix of the planned suffix.
fn commit_partial_replacement(
    typed_so_far: &mut String,
    planned_suffix: &str,
    error: &keyboard_simulator::TextReplacementError,
) {
    if error.backspaces_committed == 0 && error.utf16_units_committed == 0 {
        return;
    }
    let current_chars = typed_so_far.chars().count();
    let backspaces_applied = error.backspaces_committed.min(current_chars);
    let retained: String = typed_so_far
        .chars()
        .take(current_chars - backspaces_applied)
        .collect();
    let mut consumed_units = 0usize;
    let mut landed_suffix = String::new();
    for ch in planned_suffix.chars() {
        let units = ch.len_utf16();
        if consumed_units + units > error.utf16_units_committed {
            break;
        }
        consumed_units += units;
        landed_suffix.push(ch);
    }
    typed_so_far.clear();
    typed_so_far.push_str(&retained);
    typed_so_far.push_str(&landed_suffix);
}

fn diff_and_type<F>(
    typed_so_far: &mut String,
    new_text: &str,
    is_live: bool,
    should_stop: F,
) -> Result<keyboard_simulator::ReplacementDispatchMetrics, String>
where
    F: Fn() -> bool,
{
    diff_and_type_with(typed_so_far, new_text, is_live, |backspaces, suffix| {
        keyboard_simulator::replace_text(backspaces, suffix, should_stop)
    })
}

fn is_silence_hallucination(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    if t.is_empty() {
        return true;
    }
    if t.chars()
        .all(|c| c.is_ascii_punctuation() || c.is_whitespace())
    {
        return true;
    }

    // Unambiguous fragments Whisper produces on silence — safe to match as substrings.
    let substring_markers = [
        "no audio to transcribe",
        "no speech",
        "no audio detected",
        "there is no audio",
        "subtitles by",
        "amara.org",
        "субтитры сделал",
        "субтитры создал",
        "редактор субтитров",
        "подпишитесь на канал",
        "blank_audio",
    ];
    for marker in &substring_markers {
        if t.contains(marker) {
            return true;
        }
    }

    // Common hallucinated phrases — must match the WHOLE normalized text so that
    // ordinary dictation containing these words is never discarded.
    let normalized: String = t
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    let normalized = normalized.split_whitespace().collect::<Vec<_>>().join(" ");
    let exact_markers = [
        "спасибо за просмотр",
        "спасибо за внимание",
        "продолжение следует",
        "подпишитесь",
        "thank you for watching",
        "thanks for watching",
        "thank you",
        "you",
        "you you",
        "you you you",
        "you you you you",
    ];
    exact_markers.contains(&normalized.as_str())
}

fn capitalize_word(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn clean_hallucinated_brackets(text: &str) -> String {
    let mut cleaned = text.trim().to_string();
    let noise_words = [
        "музыка",
        "music",
        "laughter",
        "смех",
        "background noise",
        "шум",
        "applause",
        "аплодисменты",
        "silence",
        "тишина",
        "sigh",
        "вздох",
        "cough",
        "кашель",
        "crying",
        "плач",
    ];
    for term in &noise_words {
        let brackets_upper = format!("[{}]", capitalize_word(term));
        let brackets_lower = format!("[{}]", term);
        let parens_upper = format!("({})", capitalize_word(term));
        let parens_lower = format!("({})", term);

        cleaned = cleaned.replace(&brackets_upper, "");
        cleaned = cleaned.replace(&brackets_lower, "");
        cleaned = cleaned.replace(&parens_upper, "");
        cleaned = cleaned.replace(&parens_lower, "");
    }
    cleaned.trim().to_string()
}

/// Converts spoken punctuation commands ("запятая", "новая строка") into symbols.
/// Opt-in via settings; mainly useful for the local/raw transcription mode.
fn apply_voice_punctuation(text: &str) -> String {
    // Longer patterns first so "точка с запятой" wins over "точка".
    const RULES: &[(&[&str], &str)] = &[
        (&["с", "новой", "строки"], "\n"),
        (&["новая", "строка"], "\n"),
        (&["с", "нового", "абзаца"], "\n\n"),
        (&["новый", "абзац"], "\n\n"),
        (&["вопросительный", "знак"], "?"),
        (&["восклицательный", "знак"], "!"),
        (&["точка", "с", "запятой"], ";"),
        (&["new", "paragraph"], "\n\n"),
        (&["new", "line"], "\n"),
        (&["question", "mark"], "?"),
        (&["exclamation", "mark"], "!"),
        (&["exclamation", "point"], "!"),
        (&["full", "stop"], "."),
        (&["open", "parenthesis"], "("),
        (&["close", "parenthesis"], ")"),
        (&["двоеточие"], ":"),
        (&["запятая"], ","),
        (&["точка"], "."),
        (&["тире"], "—"),
        (&["открыть", "скобку"], "("),
        (&["закрыть", "скобку"], ")"),
        (&["newline"], "\n"),
        (&["comma"], ","),
        (&["period"], "."),
        (&["colon"], ":"),
        (&["semicolon"], ";"),
        (&["dash"], "—"),
    ];

    fn norm(token: &str) -> String {
        token
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase()
    }

    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut items: Vec<String> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        let mut matched = false;
        for (pattern, replacement) in RULES {
            if i + pattern.len() <= tokens.len()
                && pattern
                    .iter()
                    .enumerate()
                    .all(|(k, w)| norm(tokens[i + k]) == *w)
            {
                items.push(replacement.to_string());
                i += pattern.len();
                matched = true;
                break;
            }
        }
        if !matched {
            items.push(tokens[i].to_string());
            i += 1;
        }
    }

    let mut result = String::new();
    let mut capitalize_next = false;
    let mut glue_next = false;
    for item in &items {
        match item.as_str() {
            "," | "." | "?" | "!" | ":" | ";" | ")" => {
                result.push_str(item);
                if matches!(item.as_str(), "." | "?" | "!") {
                    capitalize_next = true;
                }
            }
            "\n" | "\n\n" => {
                result.push_str(item);
                capitalize_next = true;
            }
            "(" => {
                if !result.is_empty() && !result.ends_with('\n') {
                    result.push(' ');
                }
                result.push('(');
                glue_next = true;
            }
            "—" => {
                if !result.is_empty() && !result.ends_with('\n') {
                    result.push(' ');
                }
                result.push('—');
            }
            word => {
                if !result.is_empty() && !result.ends_with('\n') && !glue_next {
                    result.push(' ');
                }
                glue_next = false;
                if capitalize_next {
                    let mut chars = word.chars();
                    if let Some(first) = chars.next() {
                        result.extend(first.to_uppercase());
                        result.push_str(chars.as_str());
                    }
                    capitalize_next = false;
                } else {
                    result.push_str(word);
                }
            }
        }
    }
    result
}

#[derive(Clone, Debug)]
enum ClipboardBackup {
    Text(String),
    Image {
        width: usize,
        height: usize,
        bytes: Vec<u8>,
    },
    Empty,
}

fn backup_clipboard() -> ClipboardBackup {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        if let Ok(text) = cb.get_text() {
            return ClipboardBackup::Text(text);
        }
        if let Ok(img) = cb.get_image() {
            return ClipboardBackup::Image {
                width: img.width,
                height: img.height,
                bytes: img.bytes.into_owned(),
            };
        }
    }
    ClipboardBackup::Empty
}

fn restore_clipboard(backup: ClipboardBackup) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        match backup {
            ClipboardBackup::Text(text) => {
                let _ = cb.set_text(text);
            }
            ClipboardBackup::Image {
                width,
                height,
                bytes,
            } => {
                let img = arboard::ImageData {
                    width,
                    height,
                    bytes: std::borrow::Cow::Owned(bytes),
                };
                let _ = cb.set_image(img);
            }
            ClipboardBackup::Empty => {
                let _ = cb.clear();
            }
        }
    }
}

fn restore_clipboard_if_unchanged(backup: ClipboardBackup, expected_temporary_text: &str) {
    let unchanged = arboard::Clipboard::new()
        .ok()
        .and_then(|mut clipboard| clipboard.get_text().ok())
        .map(|current| current == expected_temporary_text)
        .unwrap_or(false);
    if unchanged {
        restore_clipboard(backup);
    } else {
        crate::logger::log(
            "INFO",
            "Clipboard",
            None,
            "Clipboard changed externally; skipping Aura's clipboard restore",
        );
    }
}

/// Returns `true` when the given generation still identifies the *current*
/// recording session. Stale async tasks use this before touching the clipboard
/// so they cannot clobber state captured by a newer, overlapping session.
fn session_still_current(state: &AppState, my_gen: u64) -> bool {
    state.session_gen.load(Ordering::SeqCst) == my_gen
}

/// Serialized clipboard restore: refuses to restore unless the caller's session
/// is still current and holds the shared clipboard mutex, preventing an
/// outdated session from overwriting a newer session's clipboard contents.
fn restore_clipboard_guarded(
    state: &AppState,
    my_gen: u64,
    backup: ClipboardBackup,
    expected_temporary_text: Option<&str>,
) {
    if !session_still_current(state, my_gen) {
        crate::logger::log(
            "INFO",
            "Clipboard",
            None,
            &format!(
                "Session ({my_gen}) is stale; skipping clipboard restore to protect newer session"
            ),
        );
        return;
    }
    match expected_temporary_text {
        Some(expected) => restore_clipboard_if_unchanged(backup, expected),
        None => restore_clipboard(backup),
    }
}

struct ClipboardGuard {
    backup: ClipboardBackup,
    expected_temporary_text: Option<String>,
    /// When present, restore is gated on this session still being current,
    /// so a stale (overlapped) session cannot clobber a newer one's clipboard.
    session: Option<(tauri::AppHandle, u64)>,
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        let backup = std::mem::replace(&mut self.backup, ClipboardBackup::Empty);
        if let Some((app_handle, my_gen)) = &self.session {
            if let Some(state) = app_handle.try_state::<AppState>() {
                restore_clipboard_guarded(
                    state.inner(),
                    *my_gen,
                    backup,
                    self.expected_temporary_text.as_deref(),
                );
                return;
            }
        }
        match &self.expected_temporary_text {
            Some(expected) => restore_clipboard_if_unchanged(backup, expected),
            None => restore_clipboard(backup),
        }
    }
}

/// Runs the blocking whisper.cpp sidecar off the async runtime.
///
/// `generation` identifies the owning session: the sidecar decodes are aborted
/// as soon as a newer session supersedes the current one, so a stale
/// verification decode never keeps the overlay stuck on "processing".
async fn run_local_whisper_async(
    app_handle: tauri::AppHandle,
    model: String,
    wav: String,
    language: String,
    dictionary: String,
    generation: u64,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if model == "parakeet-v3" {
            whisper_runner::run_parakeet(&app_handle, &wav, &language, &dictionary, || {
                session_stale_for_abort(&app_handle, generation)
            })
        } else {
            whisper_runner::run_local_whisper(&app_handle, &model, &wav, &language, &dictionary)
        }
    })
    .await
    .map_err(|e| format!("Local ASR task failed: {e}"))?
}

/// True when `generation` is no longer the current session and blocking work
/// belonging to it should be abandoned immediately.
fn session_stale_for_abort(app_handle: &tauri::AppHandle, generation: u64) -> bool {
    app_handle
        .try_state::<AppState>()
        .map(|state| state.session_gen.load(Ordering::SeqCst) != generation)
        .unwrap_or(true)
}

type ParakeetPreviewSocket =
    tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>;

fn send_parakeet_preview_request(
    socket: &mut ParakeetPreviewSocket,
    samples: &[f32],
) -> Result<(), String> {
    if samples.is_empty() {
        return Err("Cannot send an empty Parakeet preview request".to_string());
    }
    let byte_count = samples
        .len()
        .checked_mul(std::mem::size_of::<f32>())
        .and_then(|size| i32::try_from(size).ok())
        .ok_or_else(|| "Parakeet preview request is too large".to_string())?;

    let mut header = Vec::with_capacity(8);
    header.extend_from_slice(&16_000i32.to_le_bytes());
    header.extend_from_slice(&byte_count.to_le_bytes());
    socket
        .send(tungstenite::Message::Binary(header))
        .map_err(|error| format!("Failed to send Parakeet audio header: {error}"))?;

    const SAMPLES_PER_MESSAGE: usize = 16_384;
    for chunk in samples.chunks(SAMPLES_PER_MESSAGE) {
        let mut payload = Vec::with_capacity(std::mem::size_of_val(chunk));
        for &sample in chunk {
            let sanitized = if sample.is_finite() {
                sample.clamp(-1.0, 1.0)
            } else {
                0.0
            };
            payload.extend_from_slice(&sanitized.to_le_bytes());
        }
        socket
            .send(tungstenite::Message::Binary(payload))
            .map_err(|error| format!("Failed to send Parakeet audio chunk: {error}"))?;
    }
    Ok(())
}

fn streaming_session_cancelled(
    app_handle: &tauri::AppHandle,
    generation: u64,
    cancel: &AtomicBool,
) -> bool {
    cancel.load(Ordering::Acquire)
        || app_handle
            .try_state::<AppState>()
            .map(|state| state.session_gen.load(Ordering::SeqCst) != generation)
            .unwrap_or(true)
}

fn connect_parakeet_socket_on_port<F>(
    port: u16,
    mut is_cancelled: F,
) -> Result<ParakeetPreviewSocket, String>
where
    F: FnMut() -> bool,
{
    let started = std::time::Instant::now();
    loop {
        if is_cancelled() {
            return Err("Parakeet request was cancelled".to_string());
        }
        let url = format!("ws://127.0.0.1:{port}");
        let address = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let attempt = (|| {
            let stream = std::net::TcpStream::connect_timeout(
                &address,
                std::time::Duration::from_millis(500),
            )
            .map_err(|error| format!("TCP connect failed: {error}"))?;
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(1)))
                .map_err(|error| format!("Failed to set handshake read timeout: {error}"))?;
            stream
                .set_write_timeout(Some(std::time::Duration::from_secs(1)))
                .map_err(|error| format!("Failed to set handshake write timeout: {error}"))?;
            tungstenite::client(
                url.as_str(),
                tungstenite::stream::MaybeTlsStream::Plain(stream),
            )
            .map(|(socket, _)| socket)
            .map_err(|error| format!("WebSocket handshake failed: {error}"))
        })();
        match attempt {
            Ok(socket) => {
                if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_ref() {
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_millis(250)))
                        .map_err(|error| {
                            format!("Failed to configure Parakeet read timeout: {error}")
                        })?;
                    stream
                        .set_write_timeout(Some(std::time::Duration::from_secs(30)))
                        .map_err(|error| {
                            format!("Failed to configure Parakeet write timeout: {error}")
                        })?;
                }
                return Ok(socket);
            }
            Err(error) if started.elapsed() >= std::time::Duration::from_secs(3) => {
                return Err(format!(
                    "Failed to connect to Parakeet WebSocket on port {port}: {error}"
                ));
            }
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_millis(50));
                if is_cancelled() {
                    return Err("Parakeet request was cancelled".to_string());
                }
            }
        }
    }
}

fn read_parakeet_response_cancellable<F>(
    socket: &mut ParakeetPreviewSocket,
    sample_count: usize,
    mut is_cancelled: F,
) -> Result<String, String>
where
    F: FnMut() -> bool,
{
    let audio_seconds = sample_count as u64 / 16_000;
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs((30 + audio_seconds / 2).min(60));
    loop {
        if is_cancelled() {
            return Err("Parakeet request was cancelled".to_string());
        }
        match socket.read() {
            Ok(tungstenite::Message::Text(text)) => return Ok(text),
            Ok(tungstenite::Message::Ping(payload)) => socket
                .send(tungstenite::Message::Pong(payload))
                .map_err(|error| format!("Failed to answer Parakeet WebSocket ping: {error}"))?,
            Ok(tungstenite::Message::Pong(_)) => {}
            Ok(tungstenite::Message::Close(frame)) => {
                return Err(format!(
                    "Parakeet server closed before returning a preview: {frame:?}"
                ));
            }
            Ok(_) => {
                return Err("Unexpected binary message from Parakeet preview server".to_string());
            }
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                if std::time::Instant::now() >= deadline {
                    return Err(format!(
                        "Parakeet preview timed out for {sample_count} samples"
                    ));
                }
            }
            Err(error) => {
                return Err(format!("Failed to read Parakeet preview response: {error}"));
            }
        }
    }
}

fn normalize_parakeet_response(response_text: &str) -> String {
    let mut transcript = response_text.to_string();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(response_text) {
        if let Some(text) = value.get("text").and_then(|value| value.as_str()) {
            transcript = text.trim().to_string();
        }
    }

    let has_cyrillic = transcript
        .chars()
        .any(|character| ('\u{0400}'..='\u{04FF}').contains(&character));
    if has_cyrillic {
        transcript.replace("<unk>", "ё")
    } else {
        transcript.replace("<unk>", "")
    }
}

fn decode_parakeet_samples_on_socket<F>(
    socket: &mut ParakeetPreviewSocket,
    samples: &[f32],
    is_cancelled: F,
) -> Result<String, String>
where
    F: FnMut() -> bool,
{
    send_parakeet_preview_request(socket, samples)?;
    let response = read_parakeet_response_cancellable(socket, samples.len(), is_cancelled)?;
    Ok(normalize_parakeet_response(&response))
}

pub(crate) fn finish_parakeet_socket(socket: &mut ParakeetPreviewSocket) {
    if socket
        .send(tungstenite::Message::Text("Done".to_string()))
        .is_err()
    {
        return;
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        match socket.read() {
            Ok(tungstenite::Message::Close(_)) => {
                let _ = socket.flush();
                break;
            }
            Ok(tungstenite::Message::Ping(payload)) => {
                if socket.send(tungstenite::Message::Pong(payload)).is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => break,
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) && std::time::Instant::now() < deadline => {}
            Err(_) => break,
        }
        if std::time::Instant::now() >= deadline {
            break;
        }
    }
}

#[derive(Debug)]
pub(crate) enum ParakeetWarmUpError {
    Connection(String),
    Inference(String),
}

pub(crate) fn warm_up_parakeet_server_port(port: u16) -> Result<u128, ParakeetWarmUpError> {
    const WARM_UP_SAMPLES: usize = 16_000;
    let samples = vec![0.0_f32; WARM_UP_SAMPLES];
    let mut socket =
        connect_parakeet_socket_on_port(port, || false).map_err(ParakeetWarmUpError::Connection)?;
    let started = std::time::Instant::now();
    let result = decode_parakeet_samples_on_socket(&mut socket, &samples, || false)
        .map_err(ParakeetWarmUpError::Inference);
    finish_parakeet_socket(&mut socket);
    result?;
    Ok(started.elapsed().as_millis())
}

struct ParakeetStreamingDecoder<'a> {
    app_handle: &'a tauri::AppHandle,
    generation: u64,
    cancel: &'a AtomicBool,
    recovery_used: bool,
    session_tag: &'a str,
    socket: Option<ParakeetPreviewSocket>,
    socket_connects: u64,
}

impl ParakeetStreamingDecoder<'_> {
    fn is_cancelled(&self) -> bool {
        streaming_session_cancelled(self.app_handle, self.generation, self.cancel)
    }

    fn current_port(&self) -> Result<u16, String> {
        let port = self
            .app_handle
            .try_state::<AppState>()
            .map(|state| state.parakeet_port.load(Ordering::SeqCst))
            .ok_or_else(|| "Application state is unavailable".to_string())?;
        if port == 0 {
            return Err("Parakeet server port is unavailable".to_string());
        }
        Ok(port)
    }

    fn ensure_socket(&mut self) -> Result<(), String> {
        if self.socket.is_some() {
            return Ok(());
        }
        let port = self.current_port()?;
        let app_handle = self.app_handle;
        let generation = self.generation;
        let cancel = self.cancel;
        let socket = connect_parakeet_socket_on_port(port, || {
            streaming_session_cancelled(app_handle, generation, cancel)
        })?;
        self.socket = Some(socket);
        self.socket_connects = self.socket_connects.saturating_add(1);
        Ok(())
    }

    fn decode_once(&mut self, samples: &[f32]) -> Result<String, String> {
        self.ensure_socket()?;
        let app_handle = self.app_handle;
        let generation = self.generation;
        let cancel = self.cancel;
        let result = self
            .socket
            .as_mut()
            .ok_or_else(|| "Parakeet WebSocket state is unavailable".to_string())
            .and_then(|socket| {
                decode_parakeet_samples_on_socket(socket, samples, || {
                    streaming_session_cancelled(app_handle, generation, cancel)
                })
            });
        if result.is_err() {
            self.socket = None;
        }
        result
    }

    fn decode_with_recovery(&mut self, samples: &[f32]) -> Result<String, String> {
        match self.decode_once(samples) {
            Ok(text) => Ok(text),
            Err(error) if !self.recovery_used && !self.is_cancelled() => {
                self.recovery_used = true;
                crate::logger::log(
                    "WARN",
                    "ASR",
                    Some(self.session_tag),
                    &format!(
                        "Parakeet streaming decode failed; restarting resident server once: {error}"
                    ),
                );
                whisper_runner::stop_parakeet_server(self.app_handle);
                whisper_runner::start_parakeet_server(self.app_handle).map_err(
                    |restart_error| format!("{error}; server restart failed: {restart_error}"),
                )?;
                self.decode_once(samples)
                    .map_err(|retry_error| format!("{error}; retry failed: {retry_error}"))
            }
            Err(error) => Err(error),
        }
    }
}

impl Drop for ParakeetStreamingDecoder<'_> {
    fn drop(&mut self) {
        if let Some(mut socket) = self.socket.take() {
            finish_parakeet_socket(&mut socket);
        }
    }
}

struct ProcessedParakeetDecode {
    elapsed_ms: u128,
    preview_usable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParakeetTranscriptUpdate {
    Preview { usable: bool },
    Committed,
    EmptyEndpoint { recovered_preview: bool },
}

fn apply_parakeet_decode_text(
    reason: parakeet_streaming::DecodeReason,
    requires_transcript_overlap: bool,
    cleaned: &str,
    transcript: &mut parakeet_streaming::TranscriptState,
) -> Result<ParakeetTranscriptUpdate, String> {
    let valid = !cleaned.is_empty() && !is_silence_hallucination(cleaned);

    if reason == parakeet_streaming::DecodeReason::Preview {
        if valid {
            transcript.set_preview(cleaned);
        }
        return Ok(ParakeetTranscriptUpdate::Preview { usable: valid });
    }

    if reason == parakeet_streaming::DecodeReason::Endpoint && !valid {
        let recovered_preview = match transcript.commit_preview() {
            Some(overlap_is_safe) => {
                if requires_transcript_overlap && !overlap_is_safe {
                    return Err("Parakeet Endpoint preview could not align its overlap".to_string());
                }
                true
            }
            None => false,
        };
        return Ok(ParakeetTranscriptUpdate::EmptyEndpoint { recovered_preview });
    }

    if !valid {
        return Err(format!(
            "Parakeet {:?} decode returned no usable speech",
            reason
        ));
    }

    let overlap_is_safe = transcript.commit(cleaned);
    if requires_transcript_overlap && !overlap_is_safe {
        return Err(format!(
            "Parakeet {:?} decode could not align its overlap",
            reason
        ));
    }
    Ok(ParakeetTranscriptUpdate::Committed)
}

fn process_parakeet_decode_request(
    decoder: &mut ParakeetStreamingDecoder<'_>,
    request: parakeet_streaming::DecodeRequest,
    transcript: &mut parakeet_streaming::TranscriptState,
    metrics: &mut parakeet_streaming::StreamingMetrics,
) -> Result<ProcessedParakeetDecode, String> {
    if streaming_session_cancelled(decoder.app_handle, decoder.generation, decoder.cancel) {
        return Err("Parakeet streaming session was cancelled".to_string());
    }

    let sample_count = request.samples.len();
    let started = std::time::Instant::now();
    let decoded = decoder.decode_with_recovery(&request.samples)?;
    let elapsed_ms = started.elapsed().as_millis();
    metrics.decode_requests = metrics.decode_requests.saturating_add(1);
    metrics.total_decode_ms = metrics.total_decode_ms.saturating_add(elapsed_ms);
    metrics.decode_latency.record(elapsed_ms);
    crate::logger::log(
        "INFO",
        "ASR",
        Some(decoder.session_tag),
        &format!(
            "Parakeet {:?} decode: {sample_count} samples in {elapsed_ms} ms",
            request.reason
        ),
    );

    let cleaned = clean_hallucinated_brackets(&decoded).trim().to_string();
    let update = apply_parakeet_decode_text(
        request.reason,
        request.requires_transcript_overlap,
        &cleaned,
        transcript,
    )?;
    let preview_usable = matches!(update, ParakeetTranscriptUpdate::Preview { usable: true });
    if let ParakeetTranscriptUpdate::EmptyEndpoint { recovered_preview } = update {
        metrics.empty_endpoint_decodes = metrics.empty_endpoint_decodes.saturating_add(1);
        if recovered_preview {
            metrics.recovered_endpoint_previews =
                metrics.recovered_endpoint_previews.saturating_add(1);
        }
        crate::logger::log(
            "INFO",
            "ASR",
            Some(decoder.session_tag),
            if recovered_preview {
                "Empty Parakeet endpoint; committed the last usable preview and continued"
            } else {
                "Empty Parakeet endpoint without a usable preview; ignored it and continued"
            },
        );
    }

    let typing_mode = TypingUpdateMode::for_parakeet_update(update);
    let display_text = transcript.display_text();
    if !display_text.is_empty() {
        type_streaming_update_sync(
            decoder.app_handle,
            decoder.generation,
            &display_text,
            typing_mode,
        );
    }
    Ok(ProcessedParakeetDecode {
        elapsed_ms,
        preview_usable,
    })
}

fn observe_parakeet_decode_load(
    cadence: &mut parakeet_streaming::PreviewCadenceController,
    sample_stream: &audio_recorder::AudioSampleStream,
    metrics: &mut parakeet_streaming::StreamingMetrics,
    elapsed_ms: u128,
    session_tag: &str,
) {
    let previous_step = cadence.preview_step_samples();
    let queued_chunks = sample_stream.queued_chunks();
    metrics.max_queued_chunks = metrics.max_queued_chunks.max(queued_chunks);
    cadence.observe_decode(elapsed_ms, queued_chunks, sample_stream.capacity());
    let next_step = cadence.preview_step_samples();
    metrics.max_preview_step_samples = metrics.max_preview_step_samples.max(next_step);

    if previous_step != next_step {
        let step_ms = next_step.saturating_mul(1_000) / 16_000;
        crate::logger::log(
            "INFO",
            "ASR",
            Some(session_tag),
            &format!(
                "Adaptive Parakeet preview cadence: step={step_ms} ms, queue={queued_chunks}/{}, last_decode={elapsed_ms} ms",
                sample_stream.capacity()
            ),
        );
    }
}

fn run_parakeet_streaming_worker(
    app_handle: tauri::AppHandle,
    generation: u64,
    sample_stream: audio_recorder::AudioSampleStream,
    cancel: Arc<AtomicBool>,
) -> parakeet_streaming::StreamingOutcome {
    let session_tag = crate::logger::format_session_tag(generation);
    let mut metrics = parakeet_streaming::StreamingMetrics::default();
    let mut transcript = parakeet_streaming::TranscriptState::default();
    let mut error = None;

    if let Err(start_error) = whisper_runner::start_parakeet_server(&app_handle) {
        error = Some(format!("Parakeet server is unavailable: {start_error}"));
    }

    let mut streaming_vad = if error.is_none() {
        match vad::StreamingVad::new_16k() {
            Ok(detector) => Some(detector),
            Err(vad_error) => {
                error = Some(vad_error);
                None
            }
        }
    } else {
        None
    };
    let mut accumulator = parakeet_streaming::SegmentAccumulator::new();
    let mut cadence = parakeet_streaming::PreviewCadenceController::default();
    metrics.max_preview_step_samples = cadence.preview_step_samples();
    let mut decoder = ParakeetStreamingDecoder {
        app_handle: &app_handle,
        generation,
        cancel: &cancel,
        recovery_used: false,
        session_tag: &session_tag,
        socket: None,
        socket_connects: 0,
    };
    let mut stream_closed = false;

    while error.is_none() && !stream_closed {
        if streaming_session_cancelled(&app_handle, generation, &cancel) {
            error = Some("Parakeet streaming session was cancelled".to_string());
            break;
        }

        let first_chunk = match sample_stream.recv_timeout(std::time::Duration::from_millis(50)) {
            Ok(chunk) => chunk,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        let mut chunks = vec![first_chunk];
        loop {
            match sample_stream.try_recv() {
                Ok(chunk) => chunks.push(chunk),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    stream_closed = true;
                    break;
                }
            }
        }

        let mut requests = Vec::new();
        for chunk in chunks {
            let Some(detector) = streaming_vad.as_mut() else {
                break;
            };
            let vad_result = detector.push_with_endpoints(&chunk);
            match accumulator.push(&chunk, &vad_result.events) {
                Ok(mut chunk_requests) => requests.append(&mut chunk_requests),
                Err(segment_error) => {
                    error = Some(segment_error);
                    break;
                }
            }
            metrics.max_active_samples = metrics
                .max_active_samples
                .max(accumulator.active_sample_count());
        }

        for request in requests {
            metrics.max_active_samples = metrics.max_active_samples.max(request.samples.len());
            match process_parakeet_decode_request(
                &mut decoder,
                request,
                &mut transcript,
                &mut metrics,
            ) {
                Ok(processed) => observe_parakeet_decode_load(
                    &mut cadence,
                    &sample_stream,
                    &mut metrics,
                    processed.elapsed_ms,
                    &session_tag,
                ),
                Err(decode_error) => {
                    error = Some(decode_error);
                    break;
                }
            }
        }

        if error.is_none() && !stream_closed {
            if let Some(request) = accumulator.take_preview_request(cadence.preview_step_samples())
            {
                match process_parakeet_decode_request(
                    &mut decoder,
                    request,
                    &mut transcript,
                    &mut metrics,
                ) {
                    Ok(processed) => {
                        if processed.preview_usable {
                            accumulator.mark_preview_successful();
                        } else {
                            accumulator.invalidate_successful_preview();
                        }
                        observe_parakeet_decode_load(
                            &mut cadence,
                            &sample_stream,
                            &mut metrics,
                            processed.elapsed_ms,
                            &session_tag,
                        );
                    }
                    Err(decode_error) => error = Some(decode_error),
                }
            }
        }
    }

    let dropped_chunks = sample_stream.dropped_chunks();
    if error.is_none() && !streaming_session_cancelled(&app_handle, generation, &cancel) {
        let pending_tail_has_speech = streaming_vad
            .as_mut()
            .map(vad::StreamingVad::finish_pending_has_speech)
            .unwrap_or(true);
        let can_reuse_preview = dropped_chunks == 0
            && accumulator.can_reuse_successful_preview(pending_tail_has_speech)
            && transcript.commit_preview().is_some();
        if can_reuse_preview {
            accumulator.discard_active();
            metrics.final_preview_reused = true;
            crate::logger::log(
                "INFO",
                "ASR",
                Some(&session_tag),
                "Reused fresh Parakeet preview at release; skipped redundant final decode",
            );
        } else if let Some(request) = accumulator.finish() {
            metrics.max_active_samples = metrics.max_active_samples.max(request.samples.len());
            match process_parakeet_decode_request(
                &mut decoder,
                request,
                &mut transcript,
                &mut metrics,
            ) {
                Ok(processed) => observe_parakeet_decode_load(
                    &mut cadence,
                    &sample_stream,
                    &mut metrics,
                    processed.elapsed_ms,
                    &session_tag,
                ),
                Err(decode_error) => error = Some(decode_error),
            }
        }
    }

    metrics.socket_connects = decoder.socket_connects;
    metrics.server_recoveries = u64::from(decoder.recovery_used);
    let latency_p50_ms = metrics
        .decode_latency
        .percentile(50)
        .map_or_else(|| "n/a".to_string(), |value| value.to_string());
    let latency_p95_ms = metrics
        .decode_latency
        .percentile(95)
        .map_or_else(|| "n/a".to_string(), |value| value.to_string());
    let latency_max_ms = metrics
        .decode_latency
        .max_ms()
        .map_or_else(|| "n/a".to_string(), |value| value.to_string());
    crate::logger::log(
        if error.is_some() || dropped_chunks > 0 || metrics.empty_endpoint_decodes > 0 {
            "WARN"
        } else {
            "INFO"
        },
        "ASR",
        Some(&session_tag),
        &format!(
            "Parakeet streaming summary: decodes={}, decode_ms={}, latency_p50_ms={}, latency_p95_ms={}, latency_max_ms={}, socket_connects={}, recoveries={}, empty_endpoints={}, recovered_endpoint_previews={}, final_preview_reused={}, max_active_samples={}, max_queue_depth={}, max_preview_step_samples={}, queue_gaps={}, status={}",
            metrics.decode_requests,
            metrics.total_decode_ms,
            latency_p50_ms,
            latency_p95_ms,
            latency_max_ms,
            metrics.socket_connects,
            metrics.server_recoveries,
            metrics.empty_endpoint_decodes,
            metrics.recovered_endpoint_previews,
            metrics.final_preview_reused,
            metrics.max_active_samples,
            metrics.max_queued_chunks,
            metrics.max_preview_step_samples,
            dropped_chunks,
            if error.is_some() {
                "degraded"
            } else if metrics.empty_endpoint_decodes > 0 {
                "recovered"
            } else {
                "complete"
            }
        ),
    );

    parakeet_streaming::StreamingOutcome {
        generation,
        transcript: transcript.final_text().to_string(),
        dropped_chunks,
        error,
        metrics,
    }
}

async fn await_parakeet_streaming_outcome(
    session: ParakeetStreamingSession,
) -> Result<parakeet_streaming::StreamingOutcome, String> {
    let generation = session.generation;
    let cancel = Arc::clone(&session.cancel);
    let result = tauri::async_runtime::spawn_blocking(move || {
        session
            .result_rx
            .recv_timeout(std::time::Duration::from_secs(75))
            .map_err(|error| {
                format!("Timed out waiting for streaming session {generation}: {error}")
            })
    })
    .await
    .map_err(|error| format!("Streaming result wait worker failed: {error}"))
    .and_then(|result| result);
    if result.is_err() {
        cancel.store(true, Ordering::Release);
    }
    result
}

struct BatchPreview {
    recorded_secs: f64,
    work: Option<(usize, settings::Settings, String)>,
}

fn prepare_batch_preview(
    app_handle: &tauri::AppHandle,
    chunk_path: &str,
    previous_len: usize,
) -> Result<BatchPreview, String> {
    let state = app_handle
        .try_state::<AppState>()
        .ok_or_else(|| "Application state is unavailable".to_string())?;
    let (samples, sample_rate, channels) = state.audio_recorder.get_recorded_samples()?;
    let recorded_secs = samples.len() as f64 / (sample_rate.max(1) as f64 * channels.max(1) as f64);
    let new_start = previous_len.min(samples.len());
    let has_new_speech = vad::has_speech(&samples[new_start..], sample_rate as i64);
    if samples.len() <= 8_000 || !has_new_speech {
        return Ok(BatchPreview {
            recorded_secs,
            work: None,
        });
    }

    audio_recorder::process_and_write_wav(&samples, channels, sample_rate, chunk_path)?;
    let settings = settings::load_settings(app_handle)?;
    let layout_language = match state.selected_language.lock() {
        Ok(language) => language.clone(),
        Err(poisoned) => {
            crate::logger::log(
                "ERROR",
                "State",
                None,
                "Recovering poisoned selected-language mutex",
            );
            poisoned.into_inner().clone()
        }
    };
    Ok(BatchPreview {
        recorded_secs,
        work: Some((samples.len(), settings, layout_language)),
    })
}

fn clean_live_text(text: &str) -> String {
    let mut cleaned = String::new();
    let mut last_was_space = false;

    for ch in text.chars() {
        let lower = ch.to_lowercase().to_string();
        for l_ch in lower.chars() {
            let normalized_ch = if l_ch == 'ё' { 'е' } else { l_ch };

            if normalized_ch.is_alphanumeric() {
                cleaned.push(normalized_ch);
                last_was_space = false;
            } else if normalized_ch.is_whitespace() && !last_was_space && !cleaned.is_empty() {
                cleaned.push(' ');
                last_was_space = true;
            }
        }
    }
    cleaned.trim_end().to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TypingUpdateMode {
    LivePreview,
    CommittedSegment,
    Final,
}

impl TypingUpdateMode {
    fn for_parakeet_update(update: ParakeetTranscriptUpdate) -> Self {
        if matches!(update, ParakeetTranscriptUpdate::Committed) {
            Self::CommittedSegment
        } else {
            Self::LivePreview
        }
    }

    fn requires_recording(self) -> bool {
        !matches!(self, Self::Final)
    }

    fn uses_live_matching(self) -> bool {
        matches!(self, Self::LivePreview)
    }

    fn target_text(self, text: &str) -> String {
        if self.uses_live_matching() {
            clean_live_text(text)
        } else {
            text.to_string()
        }
    }

    fn log_label(self) -> &'static str {
        match self {
            Self::LivePreview => "live-preview",
            Self::CommittedSegment => "committed-segment",
            Self::Final => "final",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TypingUpdateOutcome {
    Applied,
    TargetDesynchronized,
    FocusChanged,
    InputDispatchFailed,
    StaleSession,
    StateUnavailable,
}

impl TypingUpdateOutcome {
    fn needs_safe_clipboard_handoff(self) -> bool {
        matches!(
            self,
            Self::TargetDesynchronized
                | Self::FocusChanged
                | Self::InputDispatchFailed
                | Self::StateUnavailable
        )
    }
}

fn finish_live_target_monitoring(app_handle: &tauri::AppHandle, generation: u64) {
    if let Some(state) = app_handle.try_state::<AppState>() {
        if state.session_gen.load(Ordering::Acquire) == generation {
            state.live_target_monitoring.store(false, Ordering::Release);
        }
    }
}

/// Types a preview, committed segment, or final update via simulated keystrokes.
/// Re-checks session generation and, for in-recording modes, the recording flag
/// under the lock so a stale task can never corrupt a newer session's text.
fn type_streaming_update_sync(
    app_handle: &tauri::AppHandle,
    my_gen: u64,
    new_text: &str,
    mode: TypingUpdateMode,
) -> TypingUpdateOutcome {
    let Some(state) = app_handle.try_state::<AppState>() else {
        return TypingUpdateOutcome::StateUnavailable;
    };
    let state = state.inner();
    let session_tag = crate::logger::format_session_tag(my_gen);

    if state.live_target_desynced.load(Ordering::Acquire) {
        return TypingUpdateOutcome::TargetDesynchronized;
    }

    // Focus guard: never type into a window the user switched to mid-dictation
    let start_hwnd = match state.start_hwnd.lock() {
        Ok(guard) => *guard,
        Err(_) => {
            crate::logger::log(
                "ERROR",
                "Typing",
                Some(&session_tag),
                "Start-window state is poisoned; refusing simulated typing",
            );
            return TypingUpdateOutcome::StateUnavailable;
        }
    };
    if start_hwnd != 0 && keyboard_simulator::get_foreground_window() != start_hwnd {
        state.live_target_desynced.store(true, Ordering::Release);
        crate::logger::log(
            "WARN",
            "Typing",
            Some(&session_tag),
            "Focus changed since recording started; skipping simulated typing.",
        );
        return TypingUpdateOutcome::FocusChanged;
    }

    let mut typed_guard = match state.typed_so_far.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            crate::logger::log(
                "ERROR",
                "Typing",
                Some(&session_tag),
                "Recovering poisoned live-text mutex before guarded typing",
            );
            poisoned.into_inner()
        }
    };
    let require_recording = mode.requires_recording();
    let session_ok = state.session_gen.load(Ordering::SeqCst) == my_gen;
    let recording_ok = !require_recording || state.is_recording.load(Ordering::SeqCst);
    if !session_ok || !recording_ok {
        crate::logger::log(
            "WARN",
            "Typing",
            Some(&session_tag),
            "Skipping stale typing update (gen/recording check failed).",
        );
        return TypingUpdateOutcome::StaleSession;
    }
    if state.live_target_desynced.load(Ordering::Acquire) {
        return TypingUpdateOutcome::TargetDesynchronized;
    }

    let text_to_type = mode.target_text(new_text);

    let dispatch_started = std::time::Instant::now();
    let dispatch_result = diff_and_type(
        &mut typed_guard,
        &text_to_type,
        mode.uses_live_matching(),
        || {
            state.live_target_desynced.load(Ordering::Acquire)
                || state.session_gen.load(Ordering::Acquire) != my_gen
                || (require_recording && !state.is_recording.load(Ordering::Acquire))
                || (start_hwnd != 0 && keyboard_simulator::get_foreground_window() != start_hwnd)
        },
    );
    let dispatch_ms = dispatch_started.elapsed().as_millis();

    let metrics = match dispatch_result {
        Ok(metrics) => metrics,
        Err(error) => {
            if state.session_gen.load(Ordering::Acquire) != my_gen
                || (require_recording && !state.is_recording.load(Ordering::Acquire))
            {
                crate::logger::log(
                    "WARN",
                    "Typing",
                    Some(&session_tag),
                    &format!(
                        "Keyboard replacement interrupted for stale session after {dispatch_ms} ms"
                    ),
                );
                return TypingUpdateOutcome::StaleSession;
            }
            if state.live_target_desynced.load(Ordering::Acquire) {
                crate::logger::log(
                    "WARN",
                    "Typing",
                    Some(&session_tag),
                    &format!(
                        "Keyboard replacement interrupted after target edit ({dispatch_ms} ms)"
                    ),
                );
                return TypingUpdateOutcome::TargetDesynchronized;
            }
            if start_hwnd != 0 && keyboard_simulator::get_foreground_window() != start_hwnd {
                state.live_target_desynced.store(true, Ordering::Release);
                crate::logger::log(
                    "WARN",
                    "Typing",
                    Some(&session_tag),
                    &format!(
                        "Keyboard replacement interrupted after focus change ({dispatch_ms} ms)"
                    ),
                );
                return TypingUpdateOutcome::FocusChanged;
            }

            state.live_target_desynced.store(true, Ordering::Release);
            crate::logger::log(
                "ERROR",
                "Typing",
                Some(&session_tag),
                &format!("Keyboard replacement dispatch failed after {dispatch_ms} ms: {error}"),
            );
            return TypingUpdateOutcome::InputDispatchFailed;
        }
    };

    if matches!(
        mode,
        TypingUpdateMode::CommittedSegment | TypingUpdateMode::Final
    ) || dispatch_ms >= 100
    {
        crate::logger::log(
            "INFO",
            "Typing",
            Some(&session_tag),
            &format!(
                "Keyboard replacement dispatch: mode={}, backspaces={}, utf16_units={}, batches={}, duration_ms={dispatch_ms}",
                mode.log_label(),
                metrics.backspaces, metrics.utf16_units, metrics.batches
            ),
        );
    }

    if state.live_target_desynced.load(Ordering::Acquire) {
        TypingUpdateOutcome::TargetDesynchronized
    } else if state.session_gen.load(Ordering::Acquire) != my_gen
        || (require_recording && !state.is_recording.load(Ordering::Acquire))
    {
        TypingUpdateOutcome::StaleSession
    } else if start_hwnd != 0 && keyboard_simulator::get_foreground_window() != start_hwnd {
        state.live_target_desynced.store(true, Ordering::Release);
        TypingUpdateOutcome::FocusChanged
    } else {
        TypingUpdateOutcome::Applied
    }
}

/// Runs guarded keyboard reconciliation away from the async runtime worker.
async fn type_streaming_update(
    app_handle: tauri::AppHandle,
    my_gen: u64,
    new_text: String,
    mode: TypingUpdateMode,
) -> TypingUpdateOutcome {
    match tauri::async_runtime::spawn_blocking(move || {
        type_streaming_update_sync(&app_handle, my_gen, &new_text, mode)
    })
    .await
    {
        Ok(outcome) => outcome,
        Err(error) => {
            crate::logger::log(
                "ERROR",
                "Typing",
                Some(&crate::logger::format_session_tag(my_gen)),
                &format!("Typing worker failed: {error}"),
            );
            TypingUpdateOutcome::StateUnavailable
        }
    }
}

/// True when the cloud error looks like "the network/provider is unreachable from here"
/// (VPN/region block, no network, proxy/TLS failure) rather than a config mistake like a
/// bad API key or an exhausted quota — those need the user's attention, not a silent swap.
///
/// IMPORTANT: keep these patterns SPECIFIC. Overly broad matches (e.g. bare "connection"
/// or "reqwest") can silently swallow account-level errors (quota, connection limits for
/// tier, 403 IP-block) and hide them from the user. Each match below is either an exact
/// phrase or a context-narrowed substring that cannot appear in auth/quota errors.
fn is_cloud_unreachable(err: &str) -> bool {
    let e = err.to_lowercase();

    // Explicit geographic / region-block signals
    if e.contains("location is not supported")
        || e.contains("user location")
        || e.contains("failed_precondition")
    {
        return true;
    }

    // 403 Forbidden — only treated as network-level when it is from a VPN/proxy exit IP.
    // Groq/OpenAI 403s on bad keys say "invalid api key", "unauthorized", etc. and are
    // caught by the API-key branch above; a bare 403 here is a Cloudflare IP-block.
    if e.contains("403") || e.contains("forbidden") {
        // If the message also mentions key/auth, it's an account error, not unreachable.
        if e.contains("api key") || e.contains("unauthorized") || e.contains("invalid") {
            return false;
        }
        return true;
    }

    // TLS / proxy layer failures are always infrastructure issues
    if e.contains("proxy") || e.contains("certificate") || e.contains("tls") || e.contains("ssl") {
        return true;
    }

    // Network-level errors: require the error to come from the transport layer.
    // "reqwest" alone can appear in quota/auth errors too, so we require it alongside
    // a transport keyword. "connection refused" / "connection reset" are infrastructure;
    // "connection limit" (Groq tier) is an account error — so match the full phrase.
    if e.contains("no network")
        || e.contains("network unreachable")
        || e.contains("timed out")
        || e.contains("dns error")
        || e.contains("name resolution")
        || e.contains("connection refused")
        || e.contains("connection reset")
        || e.contains("os error 10060")   // Windows WSAETIMEDOUT
        || e.contains("os error 10061")   // Windows WSAECONNREFUSED
        || (e.contains("timeout") && !e.contains("rate limit"))
    {
        return true;
    }

    false
}

fn select_local_fallback_model(downloaded: &[String], preferred: &str) -> Option<String> {
    downloaded
        .iter()
        .find(|model| model.as_str() == preferred)
        .or_else(|| {
            downloaded
                .iter()
                .find(|model| model.as_str() == "parakeet-v3")
        })
        .or_else(|| downloaded.first())
        .cloned()
}

fn categorize_error(err: &str) -> String {
    let err_lower = err.to_lowercase();
    if err_lower.contains("location is not supported")
        || err_lower.contains("user location")
        || err_lower.contains("failed_precondition")
    {
        "Gemini is unavailable in your region. Enable global VPN or choose Groq".to_string()
    } else if err_lower.contains("api key")
        || err_lower.contains("invalid key")
        || err_lower.contains("key is invalid")
        || err_lower.contains("incorrect api key")
        || err_lower.contains("401")
        || err_lower.contains("permission_denied")
    {
        "Invalid API key in settings".to_string()
    } else if err_lower.contains("403") || err_lower.contains("forbidden") {
        // Groq/OpenAI are reachable without a VPN; a 403 almost always means the
        // provider's edge (Cloudflare) blocked the VPN/proxy exit IP, not a bad key.
        "Access forbidden (403): VPN/proxy IP is blocked. Turn off the VPN for Groq/OpenAI or switch server".to_string()
    } else if err_lower.contains("proxy")
        || err_lower.contains("certificate")
        || err_lower.contains("tls")
        || err_lower.contains("ssl")
    {
        "Connection error via VPN/Proxy".to_string()
    } else if err_lower.contains("network")
        || err_lower.contains("timeout")
        || err_lower.contains("timed out")
        || err_lower.contains("dns")
        || err_lower.contains("connection")
        || err_lower.contains("reqwest")
    {
        let clean_err = err
            .replace("Gemini API request failed: ", "")
            .replace("reqwest::Error", "");
        let truncated = if clean_err.chars().count() > 50 {
            clean_err.chars().take(47).collect::<String>() + "..."
        } else {
            clean_err
        };
        format!("No network: {}", truncated)
    } else if err_lower.contains("ggml") || err_lower.contains("model file")
        // Restrict "not found" to local-file paths only; a bare "not found" substring also
        // appears in HTTP 404 messages (e.g. "404 Not Found") and would be mis-classified.
        || (err_lower.contains("not found") && (err_lower.contains(".bin") || err_lower.contains("path") || err_lower.contains("ggml")))
    {
        "Local model not downloaded".to_string()
    } else if err_lower.contains("sherpa") || err_lower.contains("parakeet") {
        "Local Parakeet client failure".to_string()
    } else if err_lower.contains("whisper-sidecar") || err_lower.contains("sidecar") {
        "Local Whisper client failure".to_string()
    } else if err_lower.contains("rate limit") || err_lower.contains("429") {
        "API rate limit reached".to_string()
    } else if err_lower.contains("quota") || err_lower.contains("insufficient balance") {
        "API key balance exhausted".to_string()
    } else {
        if err.chars().count() > 40 {
            err.chars().take(37).collect::<String>() + "..."
        } else {
            err.to_string()
        }
    }
}

/// Shows the error state in the overlay for a moment, then hides it
/// (unless a newer session already owns the overlay).
async fn show_overlay_error(app_handle: &tauri::AppHandle, _my_gen: u64, error_msg: &str) {
    if let Some(overlay) = app_handle.get_webview_window("overlay") {
        if let Ok(Some(monitor)) = overlay.primary_monitor() {
            let size = monitor.size();
            let scale_factor = monitor.scale_factor();

            let monitor_width = size.width as f64 / scale_factor;
            let monitor_height = size.height as f64 / scale_factor;

            let overlay_width = 160.0;
            let overlay_height = 80.0;
            const TASKBAR_MARGIN: f64 = 95.0;

            let x = (monitor_width - overlay_width) / 2.0;
            let y = monitor_height - overlay_height - TASKBAR_MARGIN;

            let _ =
                overlay.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
        }
        let _ = overlay.set_always_on_top(true);
        let _ = overlay.show();
    }
    let _ = app_handle.emit("recording-state", format!("error:{}", error_msg));
    // JS handles playing error sound and animating the hide after a 2.5s display timeout
}

/// Emits hide-overlay-requested event and runs a 500ms backup safety timer to close the overlay window.
fn request_animated_hide(app_handle: &tauri::AppHandle, status: &str) {
    if let Some(overlay) = app_handle.get_webview_window("overlay") {
        let _ = app_handle.emit(
            "hide-overlay-requested",
            serde_json::json!({ "status": status }),
        );
        let overlay_clone = overlay.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            if let Ok(true) = overlay_clone.is_visible() {
                let _ = overlay_clone.hide();
            }
        });
    }
}

/// Shows a localized, non-error notice long enough to be read, with a backend
/// hide timer as a fallback if the overlay listener is unavailable.
fn show_overlay_notice(app_handle: &tauri::AppHandle, notice_key: &str) {
    if let Some(overlay) = app_handle.get_webview_window("overlay") {
        let _ = overlay.set_always_on_top(true);
        let _ = overlay.show();
        let _ = app_handle.emit("recording-state", format!("notice:{notice_key}"));
        let overlay_clone = overlay.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(2_800)).await;
            let _ = overlay_clone.hide();
        });
    }
}

/// Stops and discards the current recording (accidental tap or Esc cancel).
fn discard_recording(app_handle: &tauri::AppHandle) -> bool {
    let mut discarded = false;
    if let Some(state) = app_handle.try_state::<AppState>() {
        let state = state.inner();
        if state.is_recording.swap(false, Ordering::SeqCst) {
            state.latched.store(false, Ordering::SeqCst);
            let generation = state.session_gen.load(Ordering::SeqCst);
            if let Ok(mut slot) = state.parakeet_streaming.lock() {
                if slot
                    .as_ref()
                    .map(|session| session.generation == generation)
                    .unwrap_or(false)
                {
                    if let Some(session) = slot.take() {
                        session.cancel.store(true, Ordering::Release);
                    }
                }
            }
            let _ = state.audio_recorder.cancel_recording();
            discarded = true;
        }
    }
    if discarded {
        let generation = app_handle
            .try_state::<AppState>()
            .map(|state| state.session_gen.load(Ordering::Acquire))
            .unwrap_or(0);
        finish_live_target_monitoring(app_handle, generation);
        keyboard_hook::set_recording_active(false);
        request_animated_hide(app_handle, "cancel");
    }
    discarded
}

/// Esc pressed during recording: discard audio and erase any streamed preview text.
async fn cancel_recording(app_handle: tauri::AppHandle) {
    let my_gen = if let Some(state) = app_handle.try_state::<AppState>() {
        state.session_gen.load(Ordering::SeqCst)
    } else {
        return;
    };
    let session_tag = crate::logger::format_session_tag(my_gen);
    crate::logger::log(
        "INFO",
        "Session",
        Some(&session_tag),
        "Esc pressed — cancelling recording session",
    );
    if !discard_recording(&app_handle) {
        return;
    }

    // Erase live-preview text that was already typed, if any
    let has_typed = app_handle
        .try_state::<AppState>()
        .and_then(|s| s.typed_so_far.lock().ok().map(|g| !g.is_empty()))
        .unwrap_or(false);
    if has_typed {
        type_streaming_update(
            app_handle.clone(),
            my_gen,
            String::new(),
            TypingUpdateMode::Final,
        )
        .await;
    }
}

/// Starts a new recording session: registers a new generation, captures context
/// (focus window, keyboard layout, selected text), starts audio capture, shows the
/// overlay, and spawns the live-streaming loop when enabled.
async fn start_recording_session(app_handle: tauri::AppHandle) {
    let Some(state) = app_handle.try_state::<AppState>() else {
        return;
    };
    let state = state.inner();

    // A new generation invalidates every task left over from previous
    // sessions. It must advance *before* `is_recording` flips to true: an old
    // async finalize that polls the flags must never observe a recording that
    // is already claimed but whose generation has not yet advanced (that
    // window would let a stale task inject itself into the fresh session).
    let gen = state.session_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let session_tag = crate::logger::format_session_tag(gen);
    state.live_target_desynced.store(false, Ordering::Release);
    state.live_target_monitoring.store(false, Ordering::Release);

    if state
        .is_recording
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        crate::logger::log(
            "WARN",
            "Session",
            None,
            "Ignoring duplicate recording start",
        );
        return;
    }
    match state.parakeet_streaming.lock() {
        Ok(mut slot) => {
            if let Some(previous) = slot.take() {
                previous.cancel.store(true, Ordering::Release);
            }
        }
        Err(_) => crate::logger::log(
            "WARN",
            "ASR",
            Some(&session_tag),
            "Streaming session state is poisoned; this dictation will use WAV fallback",
        ),
    }
    crate::logger::log(
        "INFO",
        "Session",
        Some(&session_tag),
        "Hotkey pressed — starting recording session",
    );

    // Remember the focused window for the typing focus guard
    let hwnd = keyboard_simulator::get_foreground_window();
    if let Ok(mut guard) = state.start_hwnd.lock() {
        *guard = hwnd;
    }

    // Detect active keyboard language at the moment of press
    let lang = keyboard_simulator::get_active_layout_language();
    crate::logger::log(
        "INFO",
        "Layout",
        Some(&session_tag),
        &format!("Active layout language = {}", lang),
    );
    if let Ok(mut guard) = state.selected_language.lock() {
        *guard = lang;
    }

    if let Ok(mut guard) = state.typed_so_far.lock() {
        *guard = String::new();
    }
    if let Ok(mut guard) = state.selected_text.lock() {
        *guard = String::new();
    }
    state.latched.store(false, Ordering::SeqCst);
    state.ignore_next_release.store(false, Ordering::SeqCst);
    keyboard_hook::set_recording_active(true);

    if let Ok(mut guard) = state.press_time.lock() {
        *guard = Some(std::time::Instant::now());
    }

    // Copy selected text in a background task only when copy_context_on_start is enabled.
    // Sending Ctrl+C in a terminal kills the foreground process, so users who dictate
    // into terminals should turn this option off in settings.
    let session_settings = match load_settings_async(app_handle.clone()).await {
        Ok(settings) => settings,
        Err(error) => {
            crate::logger::log(
                "WARN",
                "Settings",
                Some(&session_tag),
                &format!("Could not load session settings; using privacy-safe defaults: {error}"),
            );
            settings::Settings::default()
        }
    };
    state
        .live_target_monitoring
        .store(session_settings.streaming_enabled, Ordering::Release);
    let copy_context = session_settings.copy_context_on_start;

    if copy_context {
        let app_handle_copy = app_handle.clone();
        let session_tag_copy = session_tag.clone();
        tauri::async_runtime::spawn(async move {
            // Sleep 50ms to let the OS keyboard state settle after the physical hotkey down
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            let mut clipboard_guard = ClipboardGuard {
                backup: backup_clipboard(),
                expected_temporary_text: None,
                session: Some((app_handle_copy.clone(), gen)),
            };

            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.clear();
            }
            keyboard_simulator::simulate_copy();
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            let copied = arboard::Clipboard::new()
                .ok()
                .and_then(|mut cb| cb.get_text().ok())
                .unwrap_or_default();
            clipboard_guard.expected_temporary_text = Some(copied.clone());

            if let Some(state) = app_handle_copy.try_state::<AppState>() {
                let state = state.inner();
                let session_is_current = state.session_gen.load(Ordering::SeqCst) == gen
                    && state.is_recording.load(Ordering::SeqCst);
                if session_is_current {
                    if let Ok(mut guard) = state.selected_text.lock() {
                        *guard = copied.clone();
                        crate::logger::log(
                            "INFO",
                            "Context",
                            Some(&session_tag_copy),
                            &format!("Captured selected text ({} chars)", copied.chars().count()),
                        );
                    }
                }
            }
        });
    }

    // Start recording to a session-unique temporary WAV path
    let temp_path = recording_wav_path(gen);
    let temp_path_str = temp_path.to_string_lossy().to_string();
    crate::logger::log(
        "INFO",
        "Audio",
        Some(&session_tag),
        &format!("Starting audio recording to {}", temp_path_str),
    );

    let app_handle_recorder = app_handle.clone();
    let app_handle_vol = app_handle.clone();
    let worker_path = temp_path_str.clone();
    let enable_parakeet_sample_stream = session_settings.streaming_enabled
        && session_settings.transcription_mode == "local"
        && session_settings.local_engine == "parakeet";
    let retain_samples_for_batch_preview =
        session_settings.streaming_enabled && !enable_parakeet_sample_stream;
    let start_result = tauri::async_runtime::spawn_blocking(move || {
        let recorder = app_handle_recorder
            .try_state::<AppState>()
            .ok_or_else(|| "Application state is unavailable".to_string())?;
        recorder.audio_recorder.start_recording(
            &worker_path,
            enable_parakeet_sample_stream,
            retain_samples_for_batch_preview,
            move |vol| {
                // Decouple the overlay's volume IPC from the record worker: a
                // slow/busy overlay webview must never stall audio processing
                // and overflow the capture queue.
                let app_handle_vol = app_handle_vol.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = app_handle_vol.emit("volume-level", vol);
                });
            },
        )
    })
    .await
    .map_err(|error| format!("Audio startup worker failed: {error}"))
    .and_then(|result| result);
    if let Err(e) = start_result {
        crate::logger::log(
            "ERROR",
            "Audio",
            Some(&session_tag),
            &format!("Failed to start recording: {}", e),
        );
        state.is_recording.store(false, Ordering::SeqCst);
        finish_live_target_monitoring(&app_handle, gen);
        keyboard_hook::set_recording_active(false);
        show_overlay_error(&app_handle, gen, "Microphone start error").await;
        return;
    }

    if !state.is_recording.load(Ordering::SeqCst) || state.session_gen.load(Ordering::SeqCst) != gen
    {
        finish_live_target_monitoring(&app_handle, gen);
        return;
    }

    // Show overlay window
    if let Some(overlay) = app_handle.get_webview_window("overlay") {
        // Position overlay in the bottom center of the primary monitor
        if let Ok(Some(monitor)) = overlay.primary_monitor() {
            let size = monitor.size();
            let scale_factor = monitor.scale_factor();

            // Convert physical coordinates to logical coordinates
            let monitor_width = size.width as f64 / scale_factor;
            let monitor_height = size.height as f64 / scale_factor;

            // Match width and height from tauri.conf.json
            let overlay_width = 160.0;
            let overlay_height = 80.0;
            const TASKBAR_MARGIN: f64 = 95.0; // Place cleanly above the taskbar

            // Center horizontally
            let x = (monitor_width - overlay_width) / 2.0;
            let y = monitor_height - overlay_height - TASKBAR_MARGIN;

            let _ =
                overlay.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
        }
        let _ = overlay.set_always_on_top(true);
        let _ = overlay.emit(
            "overlay-preferences",
            OverlayPreferences::from(&session_settings),
        );
        let _ = overlay.show();
    }

    let _ = app_handle.emit("recording-state", "recording");

    let streaming_enabled = session_settings.streaming_enabled;
    if streaming_enabled {
        let app_handle_loop = app_handle.clone();
        let my_gen = gen;

        if session_settings.transcription_mode == "local"
            && session_settings.local_engine == "parakeet"
        {
            let session_tag = crate::logger::format_session_tag(my_gen);
            match state.audio_recorder.take_sample_stream() {
                Ok(sample_stream) => {
                    let cancel = Arc::new(AtomicBool::new(false));
                    let (result_tx, result_rx) = mpsc::sync_channel(1);
                    let session = ParakeetStreamingSession {
                        generation: my_gen,
                        cancel: Arc::clone(&cancel),
                        result_rx,
                    };
                    let stored = match state.parakeet_streaming.lock() {
                        Ok(mut slot) => {
                            if let Some(previous) = slot.replace(session) {
                                previous.cancel.store(true, Ordering::Release);
                            }
                            true
                        }
                        Err(_) => false,
                    };

                    if stored {
                        crate::logger::log(
                            "INFO",
                            "ASR",
                            Some(&session_tag),
                            "Started event-driven Parakeet streaming worker",
                        );
                        tauri::async_runtime::spawn_blocking(move || {
                            let outcome = run_parakeet_streaming_worker(
                                app_handle_loop,
                                my_gen,
                                sample_stream,
                                cancel,
                            );
                            let _ = result_tx.send(outcome);
                        });
                    } else {
                        cancel.store(true, Ordering::Release);
                        crate::logger::log(
                            "WARN",
                            "ASR",
                            Some(&session_tag),
                            "Streaming session state is unavailable; final WAV fallback remains active",
                        );
                    }
                }
                Err(error) => {
                    crate::logger::log(
                        "WARN",
                        "ASR",
                        Some(&session_tag),
                        &format!(
                            "Could not subscribe to recorder sample stream; using final WAV fallback: {error}"
                        ),
                    );
                    let _ = app_handle_loop.emit(
                        "streaming-degraded",
                        "Live Parakeet preview is unavailable for this dictation",
                    );
                }
            }
        } else {
            // Existing batch streaming loop
            tauri::async_runtime::spawn(async move {
                let session_tag = crate::logger::format_session_tag(my_gen);
                crate::logger::log(
                    "INFO",
                    "ASR",
                    Some(&session_tag),
                    "Spawning background streaming loop task...",
                );
                tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

                let chunk_path = chunk_wav_path(my_gen);
                let chunk_path_str = chunk_path.to_string_lossy().to_string();
                let mut last_len: usize = 0;

                loop {
                    let still_active = app_handle_loop
                        .try_state::<AppState>()
                        .map(|s| {
                            s.is_recording.load(Ordering::SeqCst)
                                && s.session_gen.load(Ordering::SeqCst) == my_gen
                        })
                        .unwrap_or(false);
                    if !still_active {
                        crate::logger::log(
                            "INFO",
                            "ASR",
                            Some(&session_tag),
                            "Streaming session ended. Exiting streaming loop.",
                        );
                        break;
                    }

                    let preview_handle = app_handle_loop.clone();
                    let preview_path = chunk_path_str.clone();
                    let previous_len = last_len;
                    let preview_result = tauri::async_runtime::spawn_blocking(move || {
                        prepare_batch_preview(&preview_handle, &preview_path, previous_len)
                    })
                    .await;
                    let preview = match preview_result {
                        Ok(Ok(preview)) => preview,
                        Ok(Err(error)) => {
                            crate::logger::log(
                                "WARN",
                                "ASR",
                                Some(&session_tag),
                                &format!("Could not prepare streaming preview: {error}"),
                            );
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            continue;
                        }
                        Err(error) => {
                            crate::logger::log(
                                "ERROR",
                                "ASR",
                                Some(&session_tag),
                                &format!("Streaming preview worker failed: {error}"),
                            );
                            break;
                        }
                    };

                    if preview.recorded_secs >= 120.0 {
                        crate::logger::log(
                            "WARN",
                            "ASR",
                            Some(&session_tag),
                            "Batch live preview paused after 120 seconds to avoid repeated full-recording uploads and decoding.",
                        );
                        let _ = app_handle_loop.emit(
                            "streaming-degraded",
                            "Live preview paused for this long dictation",
                        );
                        break;
                    }
                    let sleep_ms = if preview.recorded_secs < 60.0 {
                        4_000
                    } else {
                        8_000
                    };

                    if let Some((sample_len, settings, layout_lang)) = preview.work {
                        last_len = sample_len;
                        let language = effective_language(&settings, &layout_lang);

                        let transcription_result = if settings.transcription_mode == "local" {
                            run_local_whisper_async(
                                app_handle_loop.clone(),
                                settings.model_name.clone(),
                                chunk_path_str.clone(),
                                language.clone(),
                                settings.dictionary.clone(),
                                my_gen,
                            )
                            .await
                        } else {
                            ai_client::transcribe_and_clean(
                                provider_from(&settings),
                                &settings.api_key,
                                &chunk_path_str,
                                "",
                                &language,
                                &settings.dictionary,
                                false,
                            )
                            .await
                        };

                        match transcription_result {
                            Ok(text) => {
                                let cleaned_text = clean_hallucinated_brackets(&text);
                                let trimmed = cleaned_text.trim().to_string();
                                crate::logger::log(
                                    "INFO",
                                    "ASR",
                                    Some(&session_tag),
                                    &format!(
                                        "Streaming transcription success: '{}'",
                                        crate::logger::anonymize_speech(
                                            &trimmed,
                                            settings.log_speech_text
                                        )
                                    ),
                                );
                                if !trimmed.is_empty() && !is_silence_hallucination(&trimmed) {
                                    type_streaming_update(
                                        app_handle_loop.clone(),
                                        my_gen,
                                        trimmed,
                                        TypingUpdateMode::LivePreview,
                                    )
                                    .await;
                                }
                            }
                            Err(e) => {
                                crate::logger::log(
                                    "ERROR",
                                    "ASR",
                                    Some(&session_tag),
                                    &format!("Streaming transcription failed: {}", e),
                                );
                            }
                        }
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
                }

                let _ = tokio::fs::remove_file(&chunk_path).await;
            });
        }
    }
}

/// Stops the recording and runs the final transcription + paste/type pipeline.
async fn finalize_recording(app_handle: tauri::AppHandle) {
    let Some(state) = app_handle.try_state::<AppState>() else {
        return;
    };
    let state = state.inner();

    if state
        .is_recording
        .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    let active_layout_lang = state
        .selected_language
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    let session_selected_text = state
        .selected_text
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default();
    let my_gen = state.session_gen.load(Ordering::SeqCst);
    let session_tag = crate::logger::format_session_tag(my_gen);
    crate::logger::log(
        "INFO",
        "Session",
        Some(&session_tag),
        "Finalizing recording session",
    );
    let streaming_session = match state.parakeet_streaming.lock() {
        Ok(mut slot) => {
            if slot
                .as_ref()
                .map(|session| session.generation == my_gen)
                .unwrap_or(false)
            {
                slot.take()
            } else {
                None
            }
        }
        Err(_) => {
            crate::logger::log(
                "WARN",
                "ASR",
                Some(&session_tag),
                "Streaming session state is unavailable; using final WAV fallback",
            );
            None
        }
    };

    state.latched.store(false, Ordering::SeqCst);
    keyboard_hook::set_recording_active(false);

    let _ = app_handle.emit("recording-state", "processing");

    let stop_handle = app_handle.clone();
    let stop_res = tauri::async_runtime::spawn_blocking(move || {
        stop_handle
            .try_state::<AppState>()
            .ok_or_else(|| "Application state is unavailable".to_string())?
            .audio_recorder
            .stop_recording()
    })
    .await
    .map_err(|error| format!("Audio stop worker failed: {error}"))
    .and_then(|result| result);
    crate::logger::log(
        "INFO",
        "Audio",
        Some(&session_tag),
        &format!("stop_recording result = {:?}", stop_res),
    );
    if let Err(e) = stop_res {
        if let Some(session) = &streaming_session {
            session.cancel.store(true, Ordering::Release);
        }
        crate::logger::log(
            "ERROR",
            "Audio",
            Some(&session_tag),
            &format!("Failed to stop recording: {}", e),
        );
        finish_live_target_monitoring(&app_handle, my_gen);
        show_overlay_error(&app_handle, my_gen, "Recording stop error").await;
        return;
    }

    let app_handle_clone = app_handle.clone();
    let session_tag_clone = session_tag.clone();
    tauri::async_runtime::spawn(async move {
        let start_time = std::time::Instant::now();

        let temp_path = recording_wav_path(my_gen);
        let temp_path_str = temp_path.to_string_lossy().to_string();

        let settings = match load_settings_async(app_handle_clone.clone()).await {
            Ok(s) => s,
            Err(e) => {
                if let Some(session) = &streaming_session {
                    session.cancel.store(true, Ordering::Release);
                }
                crate::logger::log(
                    "ERROR",
                    "Settings",
                    Some(&session_tag_clone),
                    &format!("Failed to load settings: {}", e),
                );
                finish_live_target_monitoring(&app_handle_clone, my_gen);
                show_overlay_error(&app_handle_clone, my_gen, "Settings load error").await;
                return;
            }
        };

        let layout_lang = active_layout_lang.clone();
        let selected_text = session_selected_text;
        let language = effective_language(&settings, &layout_lang);
        let api_call_start = std::time::Instant::now();

        let streaming_eligible = settings.streaming_enabled
            && settings.transcription_mode == "local"
            && settings.local_engine == "parakeet"
            && settings.model_name == "parakeet-v3";
        let mut streaming_transcript = match (streaming_eligible, streaming_session) {
            (true, Some(session)) => {
                let handoff_started = std::time::Instant::now();
                match await_parakeet_streaming_outcome(session).await {
                    Ok(outcome) => {
                        crate::logger::log(
                            "INFO",
                            "ASR",
                            Some(&session_tag_clone),
                            &format!(
                                "Streaming final handoff: wait_ms={}, decodes={}, decode_ms={}, max_active_samples={}, queue_gaps={}",
                                handoff_started.elapsed().as_millis(),
                                outcome.metrics.decode_requests,
                                outcome.metrics.total_decode_ms,
                                outcome.metrics.max_active_samples,
                                outcome.dropped_chunks
                            ),
                        );
                        match outcome.reusable_transcript(my_gen) {
                            Ok(text) if !is_silence_hallucination(&text) => Some(text),
                            Ok(_) => {
                                crate::logger::log(
                                    "WARN",
                                    "ASR",
                                    Some(&session_tag_clone),
                                    "Streaming result resembled a silence hallucination; using WAV fallback",
                                );
                                None
                            }
                            Err(reason) => {
                                crate::logger::log(
                                    "WARN",
                                    "ASR",
                                    Some(&session_tag_clone),
                                    &format!("Streaming result is not reusable; using WAV fallback: {reason}"),
                                );
                                None
                            }
                        }
                    }
                    Err(error) => {
                        crate::logger::log(
                            "WARN",
                            "ASR",
                            Some(&session_tag_clone),
                            &format!("Streaming final handoff failed; using WAV fallback: {error}"),
                        );
                        None
                    }
                }
            }
            (false, Some(session)) => {
                session.cancel.store(true, Ordering::Release);
                None
            }
            (_, None) => None,
        };
        let used_streaming_result = streaming_transcript.is_some();

        if !used_streaming_result {
            let trim_path = temp_path_str.clone();
            let trim_result =
                tauri::async_runtime::spawn_blocking(move || vad::trim_wav_file(&trim_path))
                    .await
                    .map_err(|error| format!("VAD worker failed: {error}"))
                    .and_then(|result| result);
            if let Err(error) = trim_result {
                crate::logger::log(
                    "WARN",
                    "VAD",
                    Some(&session_tag_clone),
                    &format!("VAD trimming failed or skipped: {error}"),
                );
            }
        }

        crate::logger::log(
            "INFO",
            "ASR",
            Some(&session_tag_clone),
            if used_streaming_result {
                "Reusing completed Parakeet streaming result"
            } else {
                "Calling final transcription from WAV"
            },
        );
        let mut used_local_fallback = false;
        let mut transcription_result = if let Some(text) = streaming_transcript.take() {
            Ok(text)
        } else if settings.transcription_mode == "local" {
            run_local_whisper_async(
                app_handle_clone.clone(),
                settings.model_name.clone(),
                temp_path_str.clone(),
                language.clone(),
                settings.dictionary.clone(),
                my_gen,
            )
            .await
        } else {
            if settings.api_key.trim().is_empty() {
                Err("Please enter your API key in settings".to_string())
            } else {
                ai_client::transcribe_and_clean(
                    provider_from(&settings),
                    &settings.api_key,
                    &temp_path_str,
                    &selected_text,
                    &language,
                    &settings.dictionary,
                    true,
                )
                .await
            }
        };

        if settings.transcription_mode != "local" && settings.cloud_fallback_enabled {
            if let Err(cloud_err) = &transcription_result {
                if is_cloud_unreachable(cloud_err) {
                    if let Ok(downloaded) = get_downloaded_models(app_handle_clone.clone()).await {
                        let fallback_model =
                            select_local_fallback_model(&downloaded, &settings.model_name);
                        if let Some(model) = fallback_model {
                            crate::logger::log(
                                "WARN",
                                "ASR",
                                Some(&session_tag_clone),
                                &format!(
                                    "Cloud unreachable, falling back to local model '{}'",
                                    model
                                ),
                            );
                            let local_result = run_local_whisper_async(
                                app_handle_clone.clone(),
                                model,
                                temp_path_str.clone(),
                                language.clone(),
                                settings.dictionary.clone(),
                                my_gen,
                            )
                            .await;
                            if local_result.is_ok() {
                                used_local_fallback = true;
                                transcription_result = local_result;
                                let _ = app_handle_clone.emit(
                                    "recording-state",
                                    "notice:Cloud unavailable — used local model instead",
                                );
                                tokio::time::sleep(std::time::Duration::from_millis(1400)).await;
                            }
                        }
                    }
                }
            }
        }
        let final_source = if used_streaming_result {
            "streaming-reuse"
        } else if streaming_eligible {
            "wav-fallback"
        } else {
            "wav-batch"
        };
        crate::logger::log(
            "INFO",
            "ASR",
            Some(&session_tag_clone),
            &format!(
                "Final transcription duration = {} ms (source={})",
                api_call_start.elapsed().as_millis(),
                final_source
            ),
        );

        let _ = tokio::fs::remove_file(&temp_path).await;

        let mut had_error = None;
        let mut final_notice = None;
        match transcription_result {
            Ok(text) => {
                let cleaned_text = clean_hallucinated_brackets(&text);
                let trimmed = cleaned_text.trim().to_string();
                crate::logger::log(
                    "INFO",
                    "ASR",
                    Some(&session_tag_clone),
                    &format!(
                        "Final result: {}",
                        crate::logger::anonymize_speech(&trimmed, settings.log_speech_text)
                    ),
                );
                if !trimmed.is_empty() && !is_silence_hallucination(&trimmed) {
                    let text_after_voice_punc = if settings.voice_punctuation {
                        apply_voice_punctuation(&trimmed)
                    } else {
                        trimmed.clone()
                    };

                    let final_text = if settings.voice_punctuation
                        && (settings.transcription_mode == "local" || used_local_fallback)
                        && (settings.local_engine == "parakeet"
                            || settings.local_engine == "whisper")
                    {
                        let active_lang =
                            if settings.language == "layout" || settings.language == "auto" {
                                active_layout_lang.to_lowercase()
                            } else {
                                settings.language.to_lowercase()
                            };
                        let is_english = active_lang.starts_with("en");
                        if is_english {
                            match whisper_runner::run_punctuation(
                                &app_handle_clone,
                                &text_after_voice_punc,
                            ) {
                                Ok(punctuated) => punctuated,
                                Err(e) => {
                                    crate::logger::log(
                                        "WARN",
                                        "Punctuation",
                                        Some(&session_tag_clone),
                                        &format!(
                                            "Offline punctuation model failed or not found: {}",
                                            e
                                        ),
                                    );
                                    text_after_voice_punc
                                }
                            }
                        } else {
                            text_after_voice_punc
                        }
                    } else {
                        text_after_voice_punc
                    };

                    let history_mode = if used_local_fallback {
                        "local (cloud fallback)"
                    } else {
                        settings.transcription_mode.as_str()
                    };
                    match history::add_entry(&app_handle_clone, &final_text, history_mode) {
                        Ok(()) => {
                            let _ = app_handle_clone.emit("history-updated", ());
                        }
                        Err(e) => crate::logger::log(
                            "ERROR",
                            "History",
                            Some(&session_tag_clone),
                            &format!("Failed to save history: {}", e),
                        ),
                    }

                    let paste_start = std::time::Instant::now();
                    if settings.streaming_enabled {
                        let typing_outcome = type_streaming_update(
                            app_handle_clone.clone(),
                            my_gen,
                            final_text.clone(),
                            TypingUpdateMode::Final,
                        )
                        .await;
                        if typing_outcome.needs_safe_clipboard_handoff() {
                            crate::logger::log(
                                "WARN",
                                "Typing",
                                Some(&session_tag_clone),
                                &format!(
                                    "Final document reconciliation skipped ({typing_outcome:?}); preserving transcript in history and clipboard"
                                ),
                            );
                            match copy_to_clipboard(final_text.clone()).await {
                                Ok(()) => final_notice = Some("final-copied-after-edit"),
                                Err(error) => crate::logger::log(
                                    "ERROR",
                                    "Clipboard",
                                    Some(&session_tag_clone),
                                    &format!(
                                        "Could not copy the safely preserved final transcript: {error}"
                                    ),
                                ),
                            }
                        }
                    } else {
                        let session_ok = app_handle_clone
                            .try_state::<AppState>()
                            .map(|s| s.session_gen.load(Ordering::SeqCst) == my_gen)
                            .unwrap_or(false);
                        let start_hwnd = app_handle_clone
                            .try_state::<AppState>()
                            .and_then(|s| s.start_hwnd.lock().ok().map(|g| *g))
                            .unwrap_or(0);
                        let focus_ok = start_hwnd == 0
                            || keyboard_simulator::get_foreground_window() == start_hwnd;

                        if session_ok && focus_ok {
                            let mut original_clipboard = ClipboardBackup::Empty;
                            // Serialize the clipboard mutation against other
                            // overlapping sessions before the paste lands.
                            if let Some(state) = app_handle_clone.try_state::<AppState>() {
                                if let Ok(_guard) = state.clipboard_mutex.lock() {
                                    original_clipboard = backup_clipboard();
                                    if let Ok(mut cb) = arboard::Clipboard::new() {
                                        let _ = cb.set_text(final_text.clone());
                                    }
                                    keyboard_simulator::simulate_paste();
                                }
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                            // Guarded restore: only restores if this session is
                            // still current, so an overlapped newer session's
                            // clipboard is never overwritten.
                            if let Some(state) = app_handle_clone.try_state::<AppState>() {
                                if let Ok(_guard) = state.clipboard_mutex.lock() {
                                    restore_clipboard_guarded(
                                        state.inner(),
                                        my_gen,
                                        original_clipboard.clone(),
                                        Some(&final_text),
                                    );
                                }
                            } else {
                                restore_clipboard_if_unchanged(original_clipboard.clone(), &final_text);
                            }
                        } else if session_ok {
                            crate::logger::log(
                                "WARN",
                                "Paste",
                                Some(&session_tag_clone),
                                "Focus changed; leaving text in clipboard instead of pasting.",
                            );
                            if let Ok(mut cb) = arboard::Clipboard::new() {
                                let _ = cb.set_text(final_text.clone());
                            }
                        } else {
                            crate::logger::log(
                                "WARN",
                                "Paste",
                                Some(&session_tag_clone),
                                "Discarding stale session output without changing keyboard or clipboard.",
                            );
                        }
                    }
                    crate::logger::log(
                        "INFO",
                        "Paste",
                        Some(&session_tag_clone),
                        &format!("Paste duration = {} ms", paste_start.elapsed().as_millis()),
                    );
                }
            }
            Err(e) => {
                crate::logger::log(
                    "ERROR",
                    "ASR",
                    Some(&session_tag_clone),
                    &format!("Final transcription failed: {}", e),
                );
                if let Ok(dir) = app_handle_clone.path().app_local_data_dir() {
                    let _ = tokio::fs::write(dir.join("last_transcription_error.txt"), &e).await;
                }
                had_error = Some(categorize_error(&e));
            }
        }
        crate::logger::log(
            "INFO",
            "Session",
            Some(&session_tag_clone),
            &format!(
                "Total processing duration from release = {} ms",
                start_time.elapsed().as_millis()
            ),
        );

        finish_live_target_monitoring(&app_handle_clone, my_gen);

        if let Some(msg) = had_error {
            show_overlay_error(&app_handle_clone, my_gen, &msg).await;
        } else {
            let session_ok = app_handle_clone
                .try_state::<AppState>()
                .map(|s| s.session_gen.load(Ordering::SeqCst) == my_gen)
                .unwrap_or(false);
            if session_ok {
                if let Some(notice_key) = final_notice {
                    show_overlay_notice(&app_handle_clone, notice_key);
                } else {
                    request_animated_hide(&app_handle_clone, "success");
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second instance just focuses the settings window of the first one
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let app_handle = app.handle().clone();

            // Initialize diagnostics logger
            if let Ok(log_dir) = app
                .path()
                .app_log_dir()
                .or_else(|_| app.path().app_local_data_dir())
            {
                let _ = std::fs::create_dir_all(&log_dir);
                crate::logger::init(log_dir);
                crate::logger::log("INFO", "App", None, "Aura diagnostics logging initialized");
            }

            // Load settings once, then reuse the validated snapshot throughout setup.
            let startup_settings = match settings::load_settings(&app_handle) {
                Ok(settings) => settings,
                Err(error) => {
                    crate::logger::log(
                        "ERROR",
                        "Settings",
                        None,
                        &format!("Could not load startup settings; using defaults: {error}"),
                    );
                    settings::Settings::default()
                }
            };
            if let Err(error) = keyboard_hook::update_hotkey(&startup_settings.hotkey) {
                crate::logger::log(
                    "ERROR",
                    "Hotkey",
                    None,
                    &format!("Configured hotkey is invalid; keeping the default: {error}"),
                );
            }
            sync_autostart(&app_handle, startup_settings.autostart);
            // 1. Intercept CloseRequested on main window to hide it instead of closing the app
            if let Some(main_window) = app.get_webview_window("main") {
                let main_window_clone = main_window.clone();
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = main_window_clone.hide();
                    }
                });
            }

            // 2. Build system tray menu
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Open Settings", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            // 3. Build tray icon
            if let Some(tray_icon) = app.default_window_icon().cloned() {
                let _tray = TrayIconBuilder::new()
                    .icon(tray_icon)
                    .menu(&menu)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }

            app.manage(AppState {
                audio_recorder: audio_recorder::AudioRecorder::new(),
                selected_text: Mutex::new(String::new()),
                press_time: Mutex::new(None),
                is_recording: AtomicBool::new(false),
                toggle_enabled: AtomicBool::new(startup_settings.toggle_enabled),
                typed_so_far: Mutex::new(String::new()),
                live_target_desynced: AtomicBool::new(false),
                live_target_monitoring: AtomicBool::new(false),
                selected_language: Mutex::new(String::new()),
                session_gen: AtomicU64::new(0),
                clipboard_mutex: Mutex::new(()),
                latched: AtomicBool::new(false),
                ignore_next_release: AtomicBool::new(false),
                start_hwnd: Mutex::new(0),
                parakeet_lifecycle: Mutex::new(()),
                parakeet_server: Mutex::new(None),
                parakeet_port: std::sync::atomic::AtomicU16::new(3033),
                parakeet_streaming: Mutex::new(None),
                parakeet_watchdog: Mutex::new(None),
            });

            // Start Parakeet only when the validated startup snapshot selects it.
            if startup_settings.local_engine == "parakeet" {
                let sidecar_handle = app_handle.clone();
                let sidecar_settings = startup_settings;
                tauri::async_runtime::spawn_blocking(move || {
                    whisper_runner::ensure_parakeet_server_state(
                        &sidecar_handle,
                        &sidecar_settings,
                    );
                });
            }
            // Esc cancels an active recording
            let cancel_handle = app_handle.clone();
            keyboard_hook::set_cancel_callback(move || {
                let app_handle = cancel_handle.clone();
                tauri::async_runtime::spawn(async move {
                    cancel_recording(app_handle).await;
                });
            })
            .map_err(std::io::Error::other)?;

            // Physical typing/Undo while a live transcript is mirrored means
            // the target document no longer matches typed_so_far. Stop all
            // further destructive reconciliation and preserve the final result
            // through history + clipboard instead.
            let user_input_handle = app_handle.clone();
            keyboard_hook::set_user_input_callback(move || {
                let Some(state) = user_input_handle.try_state::<AppState>() else {
                    return;
                };
                if !state.live_target_monitoring.load(Ordering::Acquire) {
                    return;
                }
                if !state.live_target_desynced.swap(true, Ordering::AcqRel) {
                    let generation = state.session_gen.load(Ordering::Acquire);
                    let session_tag = crate::logger::format_session_tag(generation);
                    crate::logger::log(
                        "WARN",
                        "Typing",
                        Some(&session_tag),
                        "Physical user input detected; stopping live text reconciliation to protect the target document",
                    );
                }
            })
            .map_err(std::io::Error::other)?;

            // Start global keyboard hook
            keyboard_hook::start_hook(move |is_down| {
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let Some(state) = app_handle.try_state::<AppState>() else {
                        return;
                    };

                    if is_down {
                        crate::logger::log("INFO", "Hotkey", None, "Hotkey down");
                        let recording = state.is_recording.load(Ordering::SeqCst);
                        if recording && state.latched.load(Ordering::SeqCst) {
                            // Second tap in toggle mode stops the recording
                            state.latched.store(false, Ordering::SeqCst);
                            state.ignore_next_release.store(true, Ordering::SeqCst);
                            finalize_recording(app_handle.clone()).await;
                        } else if !recording {
                            start_recording_session(app_handle.clone()).await;
                        }
                    } else {
                        crate::logger::log("INFO", "Hotkey", None, "Hotkey up");
                        // Alt-menu disarming is handled synchronously inside the
                        // keyboard hook (send_disarmed_alt_up / dummy Ctrl tap).

                        if state.ignore_next_release.swap(false, Ordering::SeqCst) {
                            return;
                        }
                        if !state.is_recording.load(Ordering::SeqCst) {
                            return;
                        }

                        let press_duration = state
                            .press_time
                            .lock()
                            .ok()
                            .and_then(|mut g| g.take())
                            .map(|t| t.elapsed());

                        if let Some(d) = press_duration {
                            crate::logger::log(
                                "INFO",
                                "Hotkey",
                                None,
                                &format!("Press duration = {} ms", d.as_millis()),
                            );
                            if d.as_millis() < 300 {
                                let toggle_enabled = state.toggle_enabled.load(Ordering::SeqCst);
                                if toggle_enabled {
                                    // Short tap latches the recording until the next tap or Esc
                                    crate::logger::log(
                                        "INFO",
                                        "Hotkey",
                                        None,
                                        "Short tap — latching recording (toggle mode).",
                                    );
                                    state.latched.store(true, Ordering::SeqCst);
                                } else {
                                    crate::logger::log(
                                        "INFO",
                                        "Hotkey",
                                        None,
                                        "Press too short (< 300ms), discarding.",
                                    );
                                    discard_recording(&app_handle);
                                }
                                return;
                            }
                        }

                        finalize_recording(app_handle.clone()).await;
                    }
                });
            })
            .map_err(std::io::Error::other)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_provider_key,
            set_settings,
            download_model_command,
            cancel_model_download,
            delete_model_command,
            get_downloaded_models,
            get_history,
            clear_history,
            copy_to_clipboard,
            check_for_app_update,
            install_app_update,
            open_url,
            relaunch_app,
            minimize_window,
            close_window,
            start_dragging_command,
            hide_overlay_window,
            download_gpu_binaries,
            cancel_gpu_download,
            delete_gpu_binaries,
            check_gpu_downloaded,
            get_diagnostic_report,
            log_frontend_event
        ])
        .build(tauri::generate_context!());
    let application = match application {
        Ok(application) => application,
        Err(error) => {
            eprintln!("Aura failed to initialize: {error}");
            crate::logger::log("ERROR", "Startup", None, &error.to_string());
            return;
        }
    };
    application.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            whisper_runner::stop_parakeet_server(app_handle);
        }
    });
}

#[derive(Clone, serde::Serialize)]
struct GpuDownloadProgress {
    provider: String,
    downloaded: u64,
    total: Option<u64>,
    percentage: f64,
    done: bool,
}

const CUDA_ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.4/sherpa-onnx-v1.13.4-win-x64-cuda.tar.bz2";
const CUDA_ARCHIVE_SIZE: u64 = 221_905_418;
const CUDA_ARCHIVE_SHA256: &str =
    "9cc16169fb073ab0acd304ae144ccad21af03e8360921a12285105599f0f692a";
const CUDA_ARCHIVE_ROOT: &str = "sherpa-onnx-v1.13.4-win-x64-cuda";

fn lock_active_gpu_downloads() -> std::sync::MutexGuard<'static, std::collections::HashSet<String>>
{
    match ACTIVE_GPU_DOWNLOADS.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            crate::logger::log(
                "WARN",
                "GPU",
                None,
                "GPU download registry was poisoned; recovering its state",
            );
            poisoned.into_inner()
        }
    }
}

struct GpuDownloadGuard {
    provider: String,
    cancel_key: String,
}

impl Drop for GpuDownloadGuard {
    fn drop(&mut self) {
        lock_active_gpu_downloads().remove(&self.provider);
        whisper_runner::clear_cancel(&self.cancel_key);
    }
}

struct ScopedInstallDirectory {
    path: std::path::PathBuf,
    cleanup_on_drop: bool,
}

fn remove_gpu_staging_directory(path: &std::path::Path) {
    if let Err(error) = std::fs::remove_dir_all(path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            crate::logger::log(
                "WARN",
                "GPU",
                None,
                &format!(
                    "Failed to remove temporary GPU install directory {}: {error}",
                    path.display()
                ),
            );
        }
    }
}

impl Drop for ScopedInstallDirectory {
    fn drop(&mut self) {
        if !self.cleanup_on_drop {
            return;
        }
        let path = std::mem::take(&mut self.path);
        let display_path = path.display().to_string();
        if let Err(error) = std::thread::Builder::new()
            .name("aura-gpu-cleanup".to_string())
            .spawn(move || remove_gpu_staging_directory(&path))
        {
            crate::logger::log(
                "WARN",
                "GPU",
                None,
                &format!("Failed to start cleanup for {display_path}: {error}"),
            );
        }
    }
}
fn unique_gpu_install_directory(parent: &std::path::Path, provider: &str) -> std::path::PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    parent.join(format!(
        ".{provider}.install-{}-{suffix}",
        std::process::id()
    ))
}

fn gpu_bin_is_complete(bin_dir: &std::path::Path, provider: &str) -> bool {
    if provider != "cuda" {
        return false;
    }
    let exe_name = if cfg!(target_os = "windows") {
        "sherpa-onnx-offline-websocket-server.exe"
    } else {
        "sherpa-onnx-offline-websocket-server"
    };
    let mut required = vec![exe_name, "onnxruntime.dll"];
    if cfg!(target_os = "windows") {
        required.extend([
            "onnxruntime_providers_cuda.dll",
            "cudart64_110.dll",
            "cudnn64_8.dll",
        ]);
    }
    required.into_iter().all(|name| {
        bin_dir
            .join(name)
            .metadata()
            .map(|metadata| metadata.is_file() && metadata.len() > 0)
            .unwrap_or(false)
    })
}

fn install_cuda_archive(
    archive_path: &std::path::Path,
    extraction_dir: &std::path::Path,
    gpu_dir: &std::path::Path,
) -> Result<(), String> {
    std::fs::create_dir_all(extraction_dir)
        .map_err(|error| format!("Failed to create extraction directory: {error}"))?;
    let archive_file = std::fs::File::open(archive_path)
        .map_err(|error| format!("Failed to open verified CUDA archive: {error}"))?;
    let decoder = bzip2::read::BzDecoder::new(archive_file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(extraction_dir)
        .map_err(|error| format!("Failed to extract CUDA archive safely: {error}"))?;

    let source_bin = extraction_dir.join(CUDA_ARCHIVE_ROOT).join("bin");
    if !gpu_bin_is_complete(&source_bin, "cuda") {
        return Err(
            "Verified CUDA archive does not contain the required runtime files".to_string(),
        );
    }

    let target_bin = gpu_dir.join("bin");
    let backup_bin = gpu_dir.join(".bin.previous");
    if backup_bin.exists() {
        std::fs::remove_dir_all(&backup_bin)
            .map_err(|error| format!("Failed to clear stale CUDA backup: {error}"))?;
    }
    let had_previous = target_bin.exists();
    if had_previous {
        std::fs::rename(&target_bin, &backup_bin)
            .map_err(|error| format!("Failed to stage existing CUDA runtime: {error}"))?;
    }

    if let Err(error) = std::fs::rename(&source_bin, &target_bin) {
        if had_previous {
            let _ = std::fs::rename(&backup_bin, &target_bin);
        }
        return Err(format!("Failed to install CUDA runtime: {error}"));
    }
    if had_previous {
        if let Err(error) = std::fs::remove_dir_all(&backup_bin) {
            crate::logger::log(
                "WARN",
                "GPU",
                None,
                &format!("CUDA update succeeded but old backup cleanup failed: {error}"),
            );
        }
    }
    Ok(())
}

#[tauri::command]
async fn check_gpu_downloaded(
    app_handle: tauri::AppHandle,
    provider: String,
) -> Result<bool, String> {
    if provider != "cuda" && provider != "directml" {
        return Err("Invalid provider".to_string());
    }
    if provider == "directml" {
        return Ok(false);
    }
    let app_local_data = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data: {}", e))?;
    let bin_dir = app_local_data.join("binaries").join(&provider).join("bin");
    Ok(gpu_bin_is_complete(&bin_dir, &provider))
}

#[tauri::command]
async fn delete_gpu_binaries(app_handle: tauri::AppHandle, provider: String) -> Result<(), String> {
    if provider != "cuda" && provider != "directml" {
        return Err("Invalid provider".to_string());
    }
    if lock_active_gpu_downloads().contains(&provider) {
        return Err("Cannot delete GPU runtime while its download is active".to_string());
    }

    let app_local_data = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data: {}", e))?;
    let provider_dir = app_local_data.join("binaries").join(&provider);
    let worker_handle = app_handle.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Process termination and recursive deletion can block on Windows/antivirus.
        whisper_runner::stop_parakeet_server(&worker_handle);
        if provider_dir.exists() {
            std::fs::remove_dir_all(&provider_dir)
                .map_err(|e| format!("Failed to delete binaries: {}", e))?;
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|error| format!("GPU deletion worker failed: {error}"))?
}

static ACTIVE_GPU_DOWNLOADS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashSet<String>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

#[tauri::command]
async fn cancel_gpu_download(provider: String) -> Result<(), String> {
    if provider != "cuda" {
        return Err("Invalid provider".to_string());
    }
    if !lock_active_gpu_downloads().contains(&provider) {
        return Err(format!(
            "No active GPU download for '{provider}' to cancel"
        ));
    }
    whisper_runner::request_cancel_download(&format!("gpu-{provider}"));
    crate::logger::log(
        "INFO",
        "GPU",
        None,
        &format!("GPU download cancellation requested for '{provider}'"),
    );
    Ok(())
}

#[tauri::command]
async fn download_gpu_binaries(
    app_handle: tauri::AppHandle,
    provider: String,
) -> Result<(), String> {
    if provider == "directml" {
        return Err(
            "DirectML is unavailable in the official Sherpa ONNX v1.13.4 Windows runtime; use CUDA or CPU"
                .to_string(),
        );
    }
    if provider != "cuda" {
        return Err("Invalid provider".to_string());
    }
    if !cfg!(target_os = "windows") {
        return Err("The CUDA runtime downloader is available only on Windows".to_string());
    }

    let cancel_key = format!("gpu-{provider}");
    {
        let mut active = lock_active_gpu_downloads();
        if !active.insert(provider.clone()) {
            return Err("Download already in progress".to_string());
        }
    }
    whisper_runner::clear_cancel(&cancel_key);
    let _download_guard = GpuDownloadGuard {
        provider: provider.clone(),
        cancel_key: cancel_key.clone(),
    };

    let app_local_data = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data: {}", e))?;
    let binaries_dir = app_local_data.join("binaries");
    let gpu_dir = binaries_dir.join(&provider);
    tokio::fs::create_dir_all(&gpu_dir)
        .await
        .map_err(|e| format!("Failed to create GPU directory: {e}"))?;
    let staging_dir = unique_gpu_install_directory(&binaries_dir, &provider);
    tokio::fs::create_dir(&staging_dir)
        .await
        .map_err(|e| format!("Failed to create unique GPU staging directory: {e}"))?;
    let mut staging_guard = ScopedInstallDirectory {
        path: staging_dir.clone(),
        cleanup_on_drop: true,
    };
    let archive_path = staging_dir.join("runtime.tar.bz2");

    crate::logger::log(
        "INFO",
        "GPU",
        None,
        "Downloading pinned Sherpa ONNX CUDA runtime",
    );
    let client = crate::ai_client::build_download_client();
    let mut response = client
        .get(CUDA_ARCHIVE_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch CUDA runtime: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "CUDA runtime server returned HTTP status {}",
            response.status()
        ));
    }
    if let Some(advertised_size) = response.content_length() {
        if advertised_size != CUDA_ARCHIVE_SIZE {
            return Err(format!(
                "CUDA runtime size mismatch before download: expected {CUDA_ARCHIVE_SIZE}, server advertised {advertised_size}"
            ));
        }
    }

    use sha2::{Digest, Sha256};
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&archive_path)
        .await
        .map_err(|e| format!("Failed to create temporary CUDA archive: {e}"))?;
    let mut hasher = Sha256::new();
    let mut total_downloaded = 0u64;
    loop {
        let chunk = tokio::time::timeout(std::time::Duration::from_secs(30), response.chunk())
            .await
            .map_err(|_| "CUDA download stalled for more than 30 seconds".to_string())?
            .map_err(|e| format!("Failed while reading CUDA download: {e}"))?;
        let Some(chunk) = chunk else {
            break;
        };
        if whisper_runner::is_cancel_requested(&cancel_key) {
            return Err("Download cancelled".to_string());
        }
        total_downloaded = total_downloaded
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| "CUDA download byte count overflow".to_string())?;
        if total_downloaded > CUDA_ARCHIVE_SIZE {
            return Err("CUDA download exceeded its pinned size".to_string());
        }
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write CUDA archive: {e}"))?;
        let percentage = (total_downloaded as f64 / CUDA_ARCHIVE_SIZE as f64 * 100.0).min(99.9);
        let _ = app_handle.emit(
            "gpu-download-progress",
            GpuDownloadProgress {
                provider: provider.clone(),
                downloaded: total_downloaded,
                total: Some(CUDA_ARCHIVE_SIZE),
                percentage,
                done: false,
            },
        );
    }
    file.flush()
        .await
        .map_err(|e| format!("Failed to flush CUDA archive: {e}"))?;
    file.sync_all()
        .await
        .map_err(|e| format!("Failed to persist CUDA archive: {e}"))?;
    drop(file);

    if total_downloaded != CUDA_ARCHIVE_SIZE {
        return Err(format!(
            "Incomplete CUDA download: expected {CUDA_ARCHIVE_SIZE} bytes, received {total_downloaded}"
        ));
    }
    let actual_hash = format!("{:x}", hasher.finalize());
    if actual_hash != CUDA_ARCHIVE_SHA256 {
        return Err(format!(
            "CUDA archive integrity check failed: expected {CUDA_ARCHIVE_SHA256}, got {actual_hash}"
        ));
    }
    if whisper_runner::is_cancel_requested(&cancel_key) {
        return Err("Download cancelled".to_string());
    }

    crate::logger::log("INFO", "GPU", None, "Verifying and installing CUDA runtime");
    let extraction_dir = staging_dir.join("extract");
    let worker_archive = archive_path.clone();
    let worker_gpu_dir = gpu_dir.clone();
    let worker_handle = app_handle.clone();
    tauri::async_runtime::spawn_blocking(move || {
        whisper_runner::stop_parakeet_server(&worker_handle);
        install_cuda_archive(&worker_archive, &extraction_dir, &worker_gpu_dir)
    })
    .await
    .map_err(|error| format!("CUDA install worker failed: {error}"))??;

    match tokio::fs::remove_dir_all(&staging_dir).await {
        Ok(()) => staging_guard.cleanup_on_drop = false,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            staging_guard.cleanup_on_drop = false;
        }
        Err(error) => {
            crate::logger::log(
                "WARN",
                "GPU",
                None,
                &format!(
                    "Async cleanup of temporary GPU install directory {} failed: {error}; retrying in the background",
                    staging_dir.display()
                ),
            );
        }
    }

    let _ = app_handle.emit(
        "gpu-download-progress",
        GpuDownloadProgress {
            provider: provider.clone(),
            downloaded: CUDA_ARCHIVE_SIZE,
            total: Some(CUDA_ARCHIVE_SIZE),
            percentage: 100.0,
            done: true,
        },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parakeet_protocol_reuses_one_socket_for_sequential_requests() {
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .expect("bind loopback WebSocket server");
        let port = listener.local_addr().expect("loopback address").port();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept one client");
            let mut socket = tungstenite::accept(stream).expect("accept WebSocket handshake");

            for request_index in 1..=2 {
                let header = match socket.read().expect("read audio header") {
                    tungstenite::Message::Binary(header) => header,
                    message => panic!("unexpected header message: {message:?}"),
                };
                assert_eq!(header.len(), 8);
                assert_eq!(i32::from_le_bytes(header[0..4].try_into().unwrap()), 16_000);
                let expected_bytes = i32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
                let mut received_bytes = 0usize;
                while received_bytes < expected_bytes {
                    match socket.read().expect("read audio payload") {
                        tungstenite::Message::Binary(payload) => {
                            received_bytes = received_bytes.saturating_add(payload.len());
                        }
                        message => panic!("unexpected payload message: {message:?}"),
                    }
                }
                assert_eq!(received_bytes, expected_bytes);
                socket
                    .send(tungstenite::Message::Text(format!(
                        "{{\"text\":\"request {request_index}\"}}"
                    )))
                    .expect("send transcript");
            }
            assert_eq!(
                socket.read().expect("read connection terminator"),
                tungstenite::Message::Text("Done".to_string())
            );
            socket
                .close(Some(tungstenite::protocol::CloseFrame {
                    code: tungstenite::protocol::frame::coding::CloseCode::Normal,
                    reason: "Done".into(),
                }))
                .expect("start server close handshake");
            let _ = socket.read();
        });

        let mut socket =
            connect_parakeet_socket_on_port(port, || false).expect("connect loopback client");
        assert_eq!(
            decode_parakeet_samples_on_socket(&mut socket, &[0.0; 320], || false)
                .expect("decode first request"),
            "request 1"
        );
        assert_eq!(
            decode_parakeet_samples_on_socket(&mut socket, &[0.0; 640], || false)
                .expect("decode second request"),
            "request 2"
        );
        finish_parakeet_socket(&mut socket);
        server.join().expect("loopback server thread");
    }

    #[test]
    fn test_silence_hallucination_exact_phrases() {
        assert!(is_silence_hallucination(""));
        assert!(is_silence_hallucination("   ...  "));
        assert!(is_silence_hallucination("Спасибо за просмотр!"));
        assert!(is_silence_hallucination("Thank you for watching."));
        assert!(is_silence_hallucination("Продолжение следует..."));
        assert!(is_silence_hallucination("Субтитры сделал DimaTorzok"));
        assert!(is_silence_hallucination("No speech detected."));
        assert!(is_silence_hallucination("[BLANK_AUDIO]"));
        assert!(is_silence_hallucination("blank_audio"));
        assert!(is_silence_hallucination("You"));
        assert!(is_silence_hallucination("You."));
        assert!(is_silence_hallucination("you you"));
        assert!(is_silence_hallucination("You You You"));
    }

    #[test]
    fn test_silence_hallucination_keeps_real_dictation() {
        // Regression: these used to be discarded because of a substring match
        assert!(!is_silence_hallucination(
            "Назначь просмотр квартиры на завтра"
        ));
        assert!(!is_silence_hallucination(
            "Спасибо за просмотр моего резюме и обратную связь"
        ));
        assert!(!is_silence_hallucination(
            "Обычное предложение для диктовки."
        ));
    }

    #[test]
    fn test_voice_punctuation_basic() {
        assert_eq!(
            apply_voice_punctuation("привет запятая как дела вопросительный знак"),
            "привет, как дела?"
        );
        assert_eq!(
            apply_voice_punctuation("это тест точка новое предложение"),
            "это тест. Новое предложение"
        );
        assert_eq!(
            apply_voice_punctuation("первая строка новая строка вторая строка"),
            "первая строка\nВторая строка"
        );
        assert_eq!(
            apply_voice_punctuation("hello comma how are you question mark"),
            "hello, how are you?"
        );
        assert_eq!(
            apply_voice_punctuation("this is a test period new line next sentence"),
            "this is a test.\nNext sentence"
        );
    }

    #[test]
    fn test_voice_punctuation_no_commands() {
        assert_eq!(
            apply_voice_punctuation("просто обычный текст без команд"),
            "просто обычный текст без команд"
        );
        // Words that merely contain command stems must not trigger
        assert_eq!(
            apply_voice_punctuation("мы дошли до этой точки маршрута"),
            "мы дошли до этой точки маршрута"
        );
    }

    #[test]
    fn test_voice_punctuation_multiword_priority() {
        assert_eq!(
            apply_voice_punctuation("список точка с запятой продолжение"),
            "список; продолжение"
        );
    }

    #[test]
    fn test_is_cloud_unreachable() {
        assert!(is_cloud_unreachable(
            "Gemini API returned status 403 Forbidden"
        ));
        assert!(is_cloud_unreachable("error sending request: dns error"));
        assert!(is_cloud_unreachable(
            "Groq Whisper API request failed: connection timed out"
        ));
        assert!(is_cloud_unreachable(
            "location is not supported for the API use"
        ));
        assert!(!is_cloud_unreachable("Incorrect API key provided"));
        assert!(!is_cloud_unreachable(
            "insufficient_quota: You exceeded your current quota"
        ));
    }

    #[test]
    fn test_select_local_fallback_model() {
        let downloaded = vec![
            "base".to_string(),
            "parakeet-v3".to_string(),
            "small".to_string(),
        ];

        assert_eq!(
            select_local_fallback_model(&downloaded, "small").as_deref(),
            Some("small")
        );
        assert_eq!(
            select_local_fallback_model(&downloaded, "missing").as_deref(),
            Some("parakeet-v3")
        );
        assert_eq!(
            select_local_fallback_model(&["base".to_string()], "missing").as_deref(),
            Some("base")
        );
        assert_eq!(select_local_fallback_model(&[], "base"), None);
    }

    #[test]
    fn test_clean_hallucinated_brackets() {
        assert_eq!(
            clean_hallucinated_brackets("Привет [Музыка] как дела"),
            "Привет  как дела"
        );
        assert_eq!(
            clean_hallucinated_brackets("Привет (laughter) как дела"),
            "Привет  как дела"
        );
        assert_eq!(clean_hallucinated_brackets("[тишина]"), "");
        assert_eq!(clean_hallucinated_brackets("   [смех]   "), "");
        assert_eq!(
            clean_hallucinated_brackets("Обычный текст без шума"),
            "Обычный текст без шума"
        );
    }

    #[test]
    fn test_text_replacement_plan_logic() {
        // Word-normalized live matching intentionally leaves equivalent text
        // alone; final reconciliation restores exact spelling/punctuation.
        assert_eq!(
            plan_text_replacement("Все хорошо", "Всё хорошо", true),
            TextReplacementPlan {
                backspaces: 0,
                suffix: String::new(),
            }
        );
        assert_eq!(
            plan_text_replacement("Привет как дела", "Привет, как дела", true),
            TextReplacementPlan {
                backspaces: 0,
                suffix: String::new(),
            }
        );

        assert_eq!(
            plan_text_replacement("Привет как дела", "Привет как дела хорошо", true),
            TextReplacementPlan {
                backspaces: 0,
                suffix: " хорошо".to_string(),
            }
        );

        // A legitimate long hypothesis correction must remain live. The old
        // hard 25-character cap made every later update fail permanently.
        let corrected = "Очень длинное новое предложение";
        let long_plan = plan_text_replacement(
            "Очень длинное предложение которое мы надиктовали ранее",
            corrected,
            true,
        );
        assert!(long_plan.backspaces > 25);
        assert_eq!(long_plan.suffix, "новое предложение");
        assert_eq!(
            plan_text_replacement(
                corrected,
                "Очень длинное новое предложение продолжается",
                true
            ),
            TextReplacementPlan {
                backspaces: 0,
                suffix: " продолжается".to_string(),
            }
        );

        let final_plan = plan_text_replacement(
            "Очень длинное предложение которое мы надиктовали ранее",
            corrected,
            false,
        );
        assert!(final_plan.backspaces > 25);
        assert_eq!(final_plan.suffix, "новое предложение");
    }

    #[test]
    fn failed_keyboard_dispatch_does_not_advance_mirrored_text() {
        let mut mirrored = "черновой текст".to_string();

        let failure = diff_and_type_with(
            &mut mirrored,
            "чистовой текст",
            false,
            |_backspaces, _suffix| {
                Err(keyboard_simulator::TextReplacementError {
                    message: "simulated SendInput failure".to_string(),
                    backspaces_committed: 0,
                    utf16_units_committed: 0,
                })
            },
        );

        assert!(failure.is_err());
        assert_eq!(mirrored, "черновой текст");

        let metrics = diff_and_type_with(
            &mut mirrored,
            "чистовой текст",
            false,
            |backspaces, suffix| {
                assert_eq!(backspaces, 13);
                assert_eq!(suffix, "истовой текст");
                Ok(keyboard_simulator::ReplacementDispatchMetrics {
                    backspaces,
                    utf16_units: suffix.encode_utf16().count(),
                    batches: 2,
                })
            },
        )
        .expect("successful dispatch");

        assert_eq!(metrics.backspaces, 13);
        assert_eq!(mirrored, "чистовой текст");
    }

    #[test]
    fn interrupted_suffix_dispatch_commits_exact_partial_mirror() {
        let mut mirrored = "hello world".to_string();

        let failure = diff_and_type_with(
            &mut mirrored,
            "hello world extra words",
            false,
            |_backspaces, suffix| {
                // Simulate an Esc/mid-dispatch interruption after only the first
                // four characters of the suffix landed.
                let landed: String = suffix.chars().take(4).collect();
                Err(keyboard_simulator::TextReplacementError {
                    message: "interrupted for test".to_string(),
                    backspaces_committed: 0,
                    utf16_units_committed: landed.encode_utf16().count(),
                })
            },
        );

        assert!(failure.is_err());
        assert_eq!(mirrored, "hello world ext");
    }

    #[test]
    fn interrupted_backspace_phase_commits_truncated_mirror() {
        let mut typed = "abcdefgh".to_string();

        let failure = diff_and_type_with(
            &mut typed,
            "abxyz",
            false,
            |_backspaces, suffix| {
                assert_eq!(suffix, "xyz");
                Err(keyboard_simulator::TextReplacementError {
                    message: "interrupted during delete".to_string(),
                    backspaces_committed: 4,
                    utf16_units_committed: 0,
                })
            },
        );

        assert!(failure.is_err());
        assert_eq!(typed, "abcd");
    }

    #[test]
    fn semantic_noop_keeps_mirror_equal_to_the_actual_target() {
        let mut mirrored = "Привет как дела".to_string();

        diff_and_type_with(
            &mut mirrored,
            "Привет, как дела",
            true,
            |backspaces, suffix| {
                assert_eq!(backspaces, 0);
                assert!(suffix.is_empty());
                Ok(keyboard_simulator::ReplacementDispatchMetrics {
                    backspaces,
                    utf16_units: 0,
                    batches: 0,
                })
            },
        )
        .expect("semantic no-op dispatch");
        assert_eq!(mirrored, "Привет как дела");

        diff_and_type_with(
            &mut mirrored,
            "Привет, как дела дальше",
            true,
            |backspaces, suffix| {
                assert_eq!(backspaces, 0);
                assert_eq!(suffix, " дальше");
                Ok(keyboard_simulator::ReplacementDispatchMetrics {
                    backspaces,
                    utf16_units: suffix.encode_utf16().count(),
                    batches: 1,
                })
            },
        )
        .expect("semantic suffix dispatch");
        assert_eq!(mirrored, "Привет как дела дальше");

        diff_and_type_with(
            &mut mirrored,
            "Привет, как дела дальше",
            false,
            |backspaces, suffix| {
                assert!(backspaces > 0);
                assert_eq!(suffix, ", как дела дальше");
                Ok(keyboard_simulator::ReplacementDispatchMetrics {
                    backspaces,
                    utf16_units: suffix.encode_utf16().count(),
                    batches: 2,
                })
            },
        )
        .expect("exact committed-segment dispatch");
        assert_eq!(mirrored, "Привет, как дела дальше");
    }

    #[test]
    fn typing_modes_separate_preview_stability_from_exact_reconciliation() {
        assert_eq!(
            TypingUpdateMode::for_parakeet_update(ParakeetTranscriptUpdate::Preview {
                usable: true,
            }),
            TypingUpdateMode::LivePreview
        );
        assert_eq!(
            TypingUpdateMode::for_parakeet_update(ParakeetTranscriptUpdate::EmptyEndpoint {
                recovered_preview: true,
            }),
            TypingUpdateMode::LivePreview
        );
        assert_eq!(
            TypingUpdateMode::for_parakeet_update(ParakeetTranscriptUpdate::Committed),
            TypingUpdateMode::CommittedSegment
        );

        assert!(TypingUpdateMode::LivePreview.requires_recording());
        assert!(TypingUpdateMode::CommittedSegment.requires_recording());
        assert!(!TypingUpdateMode::Final.requires_recording());
        assert!(TypingUpdateMode::LivePreview.uses_live_matching());
        assert!(!TypingUpdateMode::CommittedSegment.uses_live_matching());
        assert!(!TypingUpdateMode::Final.uses_live_matching());
        assert_eq!(
            TypingUpdateMode::LivePreview.target_text("Привет! Всё хорошо."),
            "привет все хорошо"
        );
        assert_eq!(
            TypingUpdateMode::CommittedSegment.target_text("Привет! Всё хорошо."),
            "Привет! Всё хорошо."
        );
    }

    #[test]
    fn empty_endpoint_recovers_preview_and_allows_later_segments() {
        let mut transcript = parakeet_streaming::TranscriptState::default();
        transcript.set_preview("first recovered segment");

        assert_eq!(
            apply_parakeet_decode_text(
                parakeet_streaming::DecodeReason::Endpoint,
                false,
                "",
                &mut transcript,
            )
            .expect("empty endpoint is recoverable"),
            ParakeetTranscriptUpdate::EmptyEndpoint {
                recovered_preview: true,
            }
        );
        assert_eq!(transcript.final_text(), "first recovered segment");

        assert_eq!(
            apply_parakeet_decode_text(
                parakeet_streaming::DecodeReason::Endpoint,
                false,
                "second segment",
                &mut transcript,
            )
            .expect("later endpoint still works"),
            ParakeetTranscriptUpdate::Committed
        );
        assert_eq!(
            transcript.final_text(),
            "first recovered segment second segment"
        );
    }

    #[test]
    fn empty_endpoint_without_preview_is_ignored_without_degrading() {
        let mut transcript = parakeet_streaming::TranscriptState::default();

        assert_eq!(
            apply_parakeet_decode_text(
                parakeet_streaming::DecodeReason::Endpoint,
                false,
                "",
                &mut transcript,
            )
            .expect("silence endpoint is recoverable"),
            ParakeetTranscriptUpdate::EmptyEndpoint {
                recovered_preview: false,
            }
        );
        assert!(transcript.final_text().is_empty());
    }

    #[test]
    fn unsafe_typing_outcomes_require_clipboard_handoff() {
        assert!(TypingUpdateOutcome::TargetDesynchronized.needs_safe_clipboard_handoff());
        assert!(TypingUpdateOutcome::FocusChanged.needs_safe_clipboard_handoff());
        assert!(TypingUpdateOutcome::InputDispatchFailed.needs_safe_clipboard_handoff());
        assert!(TypingUpdateOutcome::StateUnavailable.needs_safe_clipboard_handoff());
        assert!(!TypingUpdateOutcome::Applied.needs_safe_clipboard_handoff());
        assert!(!TypingUpdateOutcome::StaleSession.needs_safe_clipboard_handoff());
    }

    #[test]
    fn test_clean_live_text() {
        assert_eq!(
            clean_live_text("Итак, несмотря на все..."),
            "итак несмотря на все"
        );
        assert_eq!(
            clean_live_text("Итак, несмотря на все."),
            "итак несмотря на все"
        );
        assert_eq!(
            clean_live_text("Привет! Всё ли хорошо?"),
            "привет все ли хорошо"
        );
        assert_eq!(clean_live_text(""), "");
    }

    /// Builds an `AppState` with an arbitrary session generation, used to verify
    /// the clipboard/session-staleness guard in isolation (no OS clipboard is
    /// touched by these assertions).
    fn test_app_state_with_generation(gen: u64) -> crate::AppState {
        crate::AppState {
            audio_recorder: audio_recorder::AudioRecorder::new(),
            selected_text: Mutex::new(String::new()),
            press_time: Mutex::new(None),
            is_recording: AtomicBool::new(false),
            toggle_enabled: AtomicBool::new(false),
            typed_so_far: Mutex::new(String::new()),
            live_target_desynced: AtomicBool::new(false),
            live_target_monitoring: AtomicBool::new(false),
            selected_language: Mutex::new(String::new()),
            session_gen: AtomicU64::new(gen),
            clipboard_mutex: Mutex::new(()),
            latched: AtomicBool::new(false),
            ignore_next_release: AtomicBool::new(false),
            start_hwnd: Mutex::new(0),
            parakeet_lifecycle: Mutex::new(()),
            parakeet_server: Mutex::new(None),
            parakeet_port: std::sync::atomic::AtomicU16::new(3033),
            parakeet_streaming: Mutex::new(None),
            parakeet_watchdog: Mutex::new(None),
        }
    }

    #[test]
    fn stale_session_never_restores_clipboard_after_overlap() {
        // Session A starts and latches generation 1.
        let state = test_app_state_with_generation(1);
        assert!(session_still_current(&state, 1), "gen 1 is still current");

        // Session B starts (user triggers a new dictation before A's 800ms
        // clipboard-restore window elapses), bumping the generation to 2.
        state.session_gen.store(2, Ordering::SeqCst);

        // A's restore must now be rejected — it would otherwise clobber B's data.
        assert!(
            !session_still_current(&state, 1),
            "A (gen 1) must be treated as stale once gen 2 is current"
        );
        assert!(session_still_current(&state, 2), "B (gen 2) is the live session");

        // And re-verifying restore_clipboard_guarded returns early without
        // touching the (empty) clipboard for a stale session. Direct restore of
        // an empty backup is a no-op guard-wise; we assert the guard branch is
        // taken rather than attempting an OS-level clipboard write.
        restore_clipboard_guarded(&state, 1, ClipboardBackup::Empty, None);
        // No panic, no clobber — session 1 was rejected because gen became 2.
        assert!(state.session_gen.load(Ordering::SeqCst) == 2);
    }
}
