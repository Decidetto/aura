use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};
use tauri::Manager;
use zeroize::Zeroize;

use crate::secure_storage;

const ALLOWED_MODES: &[&str] = &["cloud", "local"];
const ALLOWED_PROVIDERS: &[&str] = &["gemini", "openai", "groq"];
const ALLOWED_MODELS: &[&str] = &[
    "tiny",
    "base",
    "small",
    "medium",
    "large-v3-turbo-q5_0",
    "large-v3-turbo",
    "parakeet-v3",
];
const ALLOWED_ENGINES: &[&str] = &["whisper", "parakeet"];
const ALLOWED_ACCELERATION: &[&str] = &["cpu", "cuda"];
const ALLOWED_LANGUAGES: &[&str] = &[
    "auto", "layout", "ru", "en", "de", "es", "fr", "it", "zh", "pt", "tr",
];
const ALLOWED_SOUND_THEMES: &[&str] = &["zen", "rhodes", "scifi", "classic"];
const MAX_DICTIONARY_CHARS: usize = 4096;
const MAX_HOTKEY_CHARS: usize = 64;

static SETTINGS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_settings() -> MutexGuard<'static, ()> {
    let mutex = SETTINGS_LOCK.get_or_init(|| Mutex::new(()));
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            crate::logger::log(
                "ERROR",
                "Settings",
                None,
                "Recovering poisoned settings mutex",
            );
            poisoned.into_inner()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub transcription_mode: String,
    pub api_provider: String,
    #[serde(default, skip_serializing)]
    pub api_key: String,
    #[serde(default, skip_serializing)]
    pub api_key_gemini: String,
    #[serde(default, skip_serializing)]
    pub api_key_openai: String,
    #[serde(default, skip_serializing)]
    pub api_key_groq: String,
    pub model_name: String,
    pub hotkey: String,
    pub streaming_enabled: bool,
    pub toggle_enabled: bool,
    pub language: String,
    pub dictionary: String,
    pub voice_punctuation: bool,
    pub autostart: bool,
    pub automatic_update_checks: bool,
    pub overlay_sounds: bool,
    pub overlay_sound_theme: String,
    pub overlay_sound_volume: f32,
    pub cloud_fallback_enabled: bool,
    pub local_engine: String,
    pub copy_context_on_start: bool,
    pub local_acceleration: String,
    pub log_speech_text: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            transcription_mode: "cloud".to_string(),
            api_provider: "gemini".to_string(),
            api_key: String::new(),
            api_key_gemini: String::new(),
            api_key_openai: String::new(),
            api_key_groq: String::new(),
            model_name: "base".to_string(),
            hotkey: "Alt+V".to_string(),
            streaming_enabled: false,
            toggle_enabled: false,
            language: "auto".to_string(),
            dictionary: String::new(),
            voice_punctuation: false,
            autostart: false,
            automatic_update_checks: false,
            overlay_sounds: true,
            overlay_sound_theme: "zen".to_string(),
            overlay_sound_volume: 0.8,
            cloud_fallback_enabled: true,
            local_engine: "whisper".to_string(),
            copy_context_on_start: false,
            local_acceleration: "cpu".to_string(),
            log_speech_text: false,
        }
    }
}

impl Settings {
    pub fn active_api_key(&self) -> &str {
        match self.api_provider.as_str() {
            "gemini" => &self.api_key_gemini,
            "openai" => &self.api_key_openai,
            "groq" => &self.api_key_groq,
            _ => "",
        }
    }

    pub fn sync_active_api_key(&mut self) {
        self.api_key = self.active_api_key().to_string();
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_choice(
            "transcription mode",
            &self.transcription_mode,
            ALLOWED_MODES,
        )?;
        validate_choice("API provider", &self.api_provider, ALLOWED_PROVIDERS)?;
        validate_choice("model", &self.model_name, ALLOWED_MODELS)?;
        validate_choice("local engine", &self.local_engine, ALLOWED_ENGINES)?;
        validate_choice(
            "local acceleration",
            &self.local_acceleration,
            ALLOWED_ACCELERATION,
        )?;
        validate_choice("language", &self.language, ALLOWED_LANGUAGES)?;
        validate_choice(
            "sound theme",
            &self.overlay_sound_theme,
            ALLOWED_SOUND_THEMES,
        )?;

        let hotkey_len = self.hotkey.chars().count();
        if hotkey_len == 0 || hotkey_len > MAX_HOTKEY_CHARS {
            return Err("Hotkey must contain between 1 and 64 characters".to_string());
        }
        if self.hotkey.chars().any(|ch| ch.is_control()) {
            return Err("Hotkey must not contain control characters".to_string());
        }
        if self.dictionary.chars().count() > MAX_DICTIONARY_CHARS {
            return Err(format!(
                "Dictionary exceeds the {MAX_DICTIONARY_CHARS}-character limit"
            ));
        }
        if !self.overlay_sound_volume.is_finite()
            || !(0.0..=1.0).contains(&self.overlay_sound_volume)
        {
            return Err("Overlay sound volume must be between 0 and 1".to_string());
        }
        if self.local_engine == "parakeet" && self.model_name != "parakeet-v3" {
            return Err("Parakeet engine requires the parakeet-v3 model".to_string());
        }
        if self.local_engine == "whisper" && self.model_name == "parakeet-v3" {
            return Err("Whisper engine cannot use the parakeet-v3 model".to_string());
        }
        Ok(())
    }
}

fn validate_choice(label: &str, value: &str, allowed: &[&str]) -> Result<(), String> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(format!("Unsupported {label}: {value}"))
    }
}

#[derive(Serialize)]
pub struct SettingsView {
    #[serde(flatten)]
    pub settings: Settings,
    pub has_api_key_gemini: bool,
    pub has_api_key_openai: bool,
    pub has_api_key_groq: bool,
}

impl SettingsView {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            settings: settings.clone(),
            has_api_key_gemini: !settings.api_key_gemini.is_empty(),
            has_api_key_openai: !settings.api_key_openai.is_empty(),
            has_api_key_groq: !settings.api_key_groq.is_empty(),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct ProviderSecrets {
    gemini: String,
    openai: String,
    groq: String,
}

impl ProviderSecrets {
    fn from_settings(settings: &Settings) -> Self {
        Self {
            gemini: settings.api_key_gemini.trim().to_string(),
            openai: settings.api_key_openai.trim().to_string(),
            groq: settings.api_key_groq.trim().to_string(),
        }
    }

    fn apply_to(self, settings: &mut Settings) {
        settings.api_key_gemini = self.gemini;
        settings.api_key_openai = self.openai;
        settings.api_key_groq = self.groq;
        settings.sync_active_api_key();
    }

    fn is_empty(&self) -> bool {
        self.gemini.is_empty() && self.openai.is_empty() && self.groq.is_empty()
    }
}

pub fn get_settings_path<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    let config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|error| format!("Failed to get app config directory: {error}"))?;
    Ok(config_dir.join("settings.json"))
}

fn get_secrets_path<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    let config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|error| format!("Failed to get app config directory: {error}"))?;
    Ok(config_dir.join("secrets.dpapi"))
}

fn load_config(path: &Path) -> Result<Settings, String> {
    if !path.exists() {
        return Ok(Settings::default());
    }
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read settings file: {error}"))?;
    serde_json::from_str(&content).map_err(|error| {
        let backup = path.with_extension(format!("corrupt-{}", timestamp_millis()));
        let _ = fs::rename(path, &backup);
        format!(
            "Settings file was invalid and moved to {}: {error}",
            backup.display()
        )
    })
}

fn unreadable_secrets_backup_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("secrets.dpapi");
    path.with_file_name(format!(
        "{file_name}.unreadable-{}-{}",
        timestamp_millis(),
        std::process::id()
    ))
}

fn recover_unreadable_secrets(path: &Path, error: &str) -> ProviderSecrets {
    let secrets = ProviderSecrets::default();
    let backup = unreadable_secrets_backup_path(path);

    match fs::rename(path, &backup) {
        Ok(()) => {
            crate::logger::log(
                "WARN",
                "Settings",
                None,
                &format!(
                    "Encrypted API key store could not be read and was preserved at {}: {error}. API keys were reset and must be entered again.",
                    backup.display()
                ),
            );
            if let Err(reset_error) = save_secrets(path, &secrets) {
                crate::logger::log(
                    "ERROR",
                    "Settings",
                    None,
                    &format!(
                        "Could not initialize a replacement API key store after recovery: {reset_error}"
                    ),
                );
            }
        }
        Err(rename_error) => {
            crate::logger::log(
                "ERROR",
                "Settings",
                None,
                &format!(
                    "Encrypted API key store could not be read ({error}) or preserved as {} ({rename_error}). Continuing without API keys; the original file was left unchanged.",
                    backup.display()
                ),
            );
        }
    }

    secrets
}

fn load_secrets(path: &Path) -> ProviderSecrets {
    if !path.exists() {
        return ProviderSecrets::default();
    }

    let encrypted = match fs::read(path) {
        Ok(encrypted) => encrypted,
        Err(error) => {
            crate::logger::log(
                "ERROR",
                "Settings",
                None,
                &format!(
                    "Failed to read encrypted API keys: {error}. Continuing without API keys."
                ),
            );
            return ProviderSecrets::default();
        }
    };
    let mut plaintext = match secure_storage::unprotect_for_current_user(&encrypted) {
        Ok(plaintext) => plaintext,
        Err(error) => return recover_unreadable_secrets(path, &error),
    };
    let result = serde_json::from_slice(&plaintext);
    plaintext.zeroize();
    match result {
        Ok(secrets) => secrets,
        Err(error) => recover_unreadable_secrets(
            path,
            &format!("Encrypted API key store is invalid: {error}"),
        ),
    }
}
fn save_secrets(path: &Path, secrets: &ProviderSecrets) -> Result<(), String> {
    let mut plaintext = serde_json::to_vec(secrets)
        .map_err(|error| format!("Failed to serialize API keys: {error}"))?;
    let encrypted_result = secure_storage::protect_for_current_user(&plaintext);
    plaintext.zeroize();
    let mut encrypted = encrypted_result?;
    let result = secure_storage::atomic_write(path, &encrypted);
    encrypted.zeroize();
    result
}

fn save_config(path: &Path, settings: &Settings) -> Result<(), String> {
    let content = serde_json::to_vec_pretty(settings)
        .map_err(|error| format!("Failed to serialize settings: {error}"))?;
    secure_storage::atomic_write(path, &content)
}

fn timestamp_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn load_settings_unlocked<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<Settings, String> {
    let config_path = get_settings_path(app_handle)?;
    let secrets_path = get_secrets_path(app_handle)?;
    let mut settings = load_config(&config_path)?;

    let migrated_unsupported_acceleration = settings.local_acceleration == "directml";
    if migrated_unsupported_acceleration {
        settings.local_acceleration = "cpu".to_string();
    }
    let config_contained_plaintext = !settings.api_key.is_empty()
        || !settings.api_key_gemini.is_empty()
        || !settings.api_key_openai.is_empty()
        || !settings.api_key_groq.is_empty();
    let legacy_secrets = ProviderSecrets {
        gemini: if settings.api_key_gemini.is_empty() && settings.api_provider == "gemini" {
            settings.api_key.clone()
        } else {
            settings.api_key_gemini.clone()
        },
        openai: if settings.api_key_openai.is_empty() && settings.api_provider == "openai" {
            settings.api_key.clone()
        } else {
            settings.api_key_openai.clone()
        },
        groq: if settings.api_key_groq.is_empty() && settings.api_provider == "groq" {
            settings.api_key.clone()
        } else {
            settings.api_key_groq.clone()
        },
    };

    let secrets = if secrets_path.exists() {
        load_secrets(&secrets_path)
    } else {
        if !legacy_secrets.is_empty() {
            save_secrets(&secrets_path, &legacy_secrets)?;
        }
        legacy_secrets
    };
    if config_contained_plaintext || migrated_unsupported_acceleration {
        save_config(&config_path, &settings)?;
    }
    secrets.apply_to(&mut settings);
    settings.validate()?;
    Ok(settings)
}

pub fn load_settings<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<Settings, String> {
    let _guard = lock_settings();
    load_settings_unlocked(app_handle)
}

pub fn save_settings<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    settings: &Settings,
) -> Result<(), String> {
    let mut normalized = settings.clone();
    normalized.dictionary = normalized.dictionary.trim().to_string();
    normalized.hotkey = normalized.hotkey.trim().to_string();
    normalized.validate()?;
    let _guard = lock_settings();

    // API keys are write-only and are updated solely by set_provider_key.
    // Saving a redacted SettingsView must never erase the encrypted store.
    save_config(&get_settings_path(app_handle)?, &normalized)
}

pub fn set_provider_key<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    provider: &str,
    key: &str,
) -> Result<(), String> {
    validate_choice("API provider", provider, ALLOWED_PROVIDERS)?;
    if key.chars().count() > 1024 || key.chars().any(|ch| ch.is_control() && !ch.is_whitespace()) {
        return Err("API key is invalid or too long".to_string());
    }

    let _guard = lock_settings();
    let mut settings = load_settings_unlocked(app_handle)?;
    let normalized = key.trim().to_string();
    match provider {
        "gemini" => settings.api_key_gemini = normalized,
        "openai" => settings.api_key_openai = normalized,
        "groq" => settings.api_key_groq = normalized,
        _ => return Err(format!("Unsupported API provider: {provider}")),
    }
    settings.sync_active_api_key();
    let secrets = ProviderSecrets::from_settings(&settings);
    save_secrets(&get_secrets_path(app_handle)?, &secrets)?;
    save_config(&get_settings_path(app_handle)?, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_private_and_safe() {
        let settings = Settings::default();
        assert!(!settings.copy_context_on_start);
        assert_eq!(settings.local_acceleration, "cpu");
        assert!(!settings.log_speech_text);
        assert_eq!(settings.active_api_key(), "");
        settings.validate().unwrap();
    }

    #[test]
    fn settings_view_never_serializes_secrets() {
        let settings = Settings {
            api_key: "legacy-secret".to_string(),
            api_key_gemini: "gemini-secret".to_string(),
            ..Settings::default()
        };
        let json = serde_json::to_string(&SettingsView::from_settings(&settings)).unwrap();
        assert!(!json.contains("legacy-secret"));
        assert!(!json.contains("gemini-secret"));
        assert!(json.contains("\"has_api_key_gemini\":true"));
    }

    #[test]
    fn validation_rejects_unknown_values_and_oversized_dictionary() {
        let invalid_provider = Settings {
            api_provider: "unknown".to_string(),
            ..Settings::default()
        };
        assert!(invalid_provider.validate().is_err());

        let invalid_dictionary = Settings {
            dictionary: "x".repeat(MAX_DICTIONARY_CHARS + 1),
            ..Settings::default()
        };
        assert!(invalid_dictionary.validate().is_err());
    }

    #[test]
    fn engine_and_model_must_match() {
        let invalid = Settings {
            local_engine: "parakeet".to_string(),
            model_name: "base".to_string(),
            ..Settings::default()
        };
        assert!(invalid.validate().is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn unreadable_secret_store_is_preserved_and_reinitialized() {
        let directory = std::env::temp_dir().join(format!(
            "aura-secrets-recovery-{}-{}",
            std::process::id(),
            timestamp_millis()
        ));
        let path = directory.join("secrets.dpapi");
        let corrupted = b"AURA-DPAPI-1\0invalid-dpapi-payload";
        fs::create_dir_all(&directory).unwrap();
        fs::write(&path, corrupted).unwrap();

        let secrets = load_secrets(&path);

        assert!(secrets.is_empty());
        let backups: Vec<PathBuf> = fs::read_dir(&directory)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|candidate| {
                candidate
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("secrets.dpapi.unreadable-"))
            })
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(&backups[0]).unwrap(), corrupted);

        let encrypted = fs::read(&path).unwrap();
        let mut plaintext = secure_storage::unprotect_for_current_user(&encrypted).unwrap();
        let replacement: ProviderSecrets = serde_json::from_slice(&plaintext).unwrap();
        plaintext.zeroize();
        assert!(replacement.is_empty());

        let _ = fs::remove_dir_all(directory);
    }
}
