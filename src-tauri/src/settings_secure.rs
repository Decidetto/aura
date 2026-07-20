use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

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
const ALLOWED_LANGUAGES: &[&str] = &[
    "auto", "layout", "ru", "en", "de", "es", "fr", "it", "zh", "pt", "tr",
];
const ALLOWED_SOUND_THEMES: &[&str] = &["zen", "rhodes", "scifi", "classic"];
const MAX_DICTIONARY_CHARS: usize = 4096;
const MAX_HOTKEY_CHARS: usize = 64;

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
    pub overlay_sounds: bool,
    pub overlay_sound_theme: String,
    pub overlay_sound_volume: f32,
    pub cloud_fallback_enabled: bool,
    pub local_engine: String,
    pub selection_edit_enabled: bool,
    pub automatic_update_checks: bool,
    pub history_enabled: bool,
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
            overlay_sounds: true,
            overlay_sound_theme: "zen".to_string(),
            overlay_sound_volume: 0.8,
            cloud_fallback_enabled: true,
            local_engine: "whisper".to_string(),
            selection_edit_enabled: false,
            automatic_update_checks: false,
            history_enabled: true,
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
        validate_choice("transcription mode", &self.transcription_mode, ALLOWED_MODES)?;
        validate_choice("API provider", &self.api_provider, ALLOWED_PROVIDERS)?;
        validate_choice("model", &self.model_name, ALLOWED_MODELS)?;
        validate_choice("local engine", &self.local_engine, ALLOWED_ENGINES)?;
        validate_choice("language", &self.language, ALLOWED_LANGUAGES)?;
        validate_choice("sound theme", &self.overlay_sound_theme, ALLOWED_SOUND_THEMES)?;

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

fn load_secrets(path: &Path) -> Result<ProviderSecrets, String> {
    if !path.exists() {
        return Ok(ProviderSecrets::default());
    }
    let encrypted = fs::read(path)
        .map_err(|error| format!("Failed to read encrypted API keys: {error}"))?;
    let plaintext = secure_storage::unprotect_for_current_user(&encrypted)?;
    serde_json::from_slice(&plaintext)
        .map_err(|error| format!("Encrypted API key store is invalid: {error}"))
}

fn save_secrets(path: &Path, secrets: &ProviderSecrets) -> Result<(), String> {
    let plaintext = serde_json::to_vec(secrets)
        .map_err(|error| format!("Failed to serialize API keys: {error}"))?;
    let encrypted = secure_storage::protect_for_current_user(&plaintext)?;
    secure_storage::atomic_write(path, &encrypted)
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

pub fn load_settings<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<Settings, String> {
    let config_path = get_settings_path(app_handle)?;
    let secrets_path = get_secrets_path(app_handle)?;
    let mut settings = load_config(&config_path)?;

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
        load_secrets(&secrets_path)?
    } else {
        if !legacy_secrets.is_empty() {
            save_secrets(&secrets_path, &legacy_secrets)?;
            save_config(&config_path, &settings)?;
        }
        legacy_secrets
    };
    secrets.apply_to(&mut settings);
    settings.validate()?;
    Ok(settings)
}

pub fn save_settings<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    settings: &Settings,
) -> Result<(), String> {
    let mut normalized = settings.clone();
    normalized.dictionary = normalized.dictionary.trim().to_string();
    normalized.hotkey = normalized.hotkey.trim().to_string();
    normalized.sync_active_api_key();
    normalized.validate()?;

    let secrets = ProviderSecrets::from_settings(&normalized);
    save_secrets(&get_secrets_path(app_handle)?, &secrets)?;
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

    let mut settings = load_settings(app_handle)?;
    let normalized = key.trim().to_string();
    match provider {
        "gemini" => settings.api_key_gemini = normalized,
        "openai" => settings.api_key_openai = normalized,
        "groq" => settings.api_key_groq = normalized,
        _ => unreachable!(),
    }
    settings.sync_active_api_key();
    save_settings(app_handle, &settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_private_and_safe() {
        let settings = Settings::default();
        assert!(!settings.selection_edit_enabled);
        assert!(!settings.automatic_update_checks);
        assert!(settings.history_enabled);
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
}
