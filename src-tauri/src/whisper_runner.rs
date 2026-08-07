use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Mutex, OnceLock};
use tauri::{Emitter, Manager, Runtime};

fn spawn_output_reader<R>(
    mut reader: R,
    thread_name: &str,
    stream_name: &'static str,
) -> Result<std::thread::JoinHandle<Result<Vec<u8>, String>>, String>
where
    R: std::io::Read + Send + 'static,
{
    std::thread::Builder::new()
        .name(thread_name.to_string())
        .spawn(move || {
            let mut output = Vec::new();
            reader
                .read_to_end(&mut output)
                .map_err(|error| format!("Failed to read {stream_name}: {error}"))?;
            Ok(output)
        })
        .map_err(|error| format!("Failed to start {stream_name} reader: {error}"))
}

fn collect_output_reader(
    reader: std::thread::JoinHandle<Result<Vec<u8>, String>>,
    stream_name: &str,
) -> Result<Vec<u8>, String> {
    reader
        .join()
        .map_err(|_| format!("{stream_name} reader panicked"))?
}

fn terminate_and_reap(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn run_command_with_timeout(
    command: &mut Command,
    timeout: std::time::Duration,
    label: &str,
) -> Result<Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("Failed to start {label}: {error}"))?;

    let Some(stdout) = child.stdout.take() else {
        terminate_and_reap(&mut child);
        return Err(format!("{label} stdout pipe is unavailable"));
    };
    let Some(stderr) = child.stderr.take() else {
        terminate_and_reap(&mut child);
        return Err(format!("{label} stderr pipe is unavailable"));
    };

    let stdout_reader = match spawn_output_reader(stdout, "aura-sidecar-stdout", "sidecar stdout") {
        Ok(reader) => reader,
        Err(error) => {
            terminate_and_reap(&mut child);
            return Err(error);
        }
    };
    let stderr_reader = match spawn_output_reader(stderr, "aura-sidecar-stderr", "sidecar stderr") {
        Ok(reader) => reader,
        Err(error) => {
            terminate_and_reap(&mut child);
            let _ = collect_output_reader(stdout_reader, "sidecar stdout");
            return Err(error);
        }
    };

    let started = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(None) => {
                terminate_and_reap(&mut child);
                let _ = collect_output_reader(stdout_reader, "sidecar stdout");
                let _ = collect_output_reader(stderr_reader, "sidecar stderr");
                return Err(format!(
                    "{label} timed out after {} seconds and was terminated",
                    timeout.as_secs()
                ));
            }
            Err(error) => {
                terminate_and_reap(&mut child);
                let _ = collect_output_reader(stdout_reader, "sidecar stdout");
                let _ = collect_output_reader(stderr_reader, "sidecar stderr");
                return Err(format!("Failed to inspect {label}: {error}"));
            }
        }
    };

    let stdout = collect_output_reader(stdout_reader, "sidecar stdout")?;
    let stderr = collect_output_reader(stderr_reader, "sidecar stderr")?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn whisper_timeout_for_duration(model_name: &str, audio_seconds: u64) -> std::time::Duration {
    let model = model_name.to_ascii_lowercase();
    let realtime_factor = if model.contains("tiny") || model.contains("base") {
        2
    } else if model.contains("small") {
        3
    } else {
        5
    };
    let seconds = 120u64
        .saturating_add(audio_seconds.saturating_mul(realtime_factor))
        .clamp(180, 3_600);
    std::time::Duration::from_secs(seconds)
}

fn whisper_timeout_for_wav(model_name: &str, wav_path: &str) -> std::time::Duration {
    let audio_seconds = hound::WavReader::open(wav_path)
        .ok()
        .map(|reader| {
            let sample_rate = u64::from(reader.spec().sample_rate.max(1));
            let sample_count = u64::from(reader.duration());
            sample_count.saturating_add(sample_rate.saturating_sub(1)) / sample_rate
        })
        .unwrap_or(0);
    whisper_timeout_for_duration(model_name, audio_seconds)
}
/// Models whose in-flight download the user asked to cancel (keyed by model name).
static DOWNLOAD_CANCEL: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn cancel_set() -> &'static Mutex<HashSet<String>> {
    DOWNLOAD_CANCEL.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Requests cancellation of the running download for `model_name`.
pub fn request_cancel_download(model_name: &str) {
    recover_lock(cancel_set(), "download cancellation").insert(model_name.to_string());
}

pub fn is_cancel_requested(model_name: &str) -> bool {
    recover_lock(cancel_set(), "download cancellation").contains(model_name)
}

pub fn clear_cancel(model_name: &str) {
    recover_lock(cancel_set(), "download cancellation").remove(model_name);
}

#[derive(Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub model: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percentage: f64,
    pub done: bool,
}

/// Helper to search recursively for the sidecar file if not in direct paths
fn find_file_recursive(dir: &Path, target_name: &str) -> Option<PathBuf> {
    if dir.is_dir() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(found) = find_file_recursive(&path, target_name) {
                        return Some(found);
                    }
                } else if path.file_name().is_some_and(|name| name == target_name) {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// Format the model filename correctly to prevent duplicates (e.g., ggml-ggml-tiny.bin)
fn format_model_filename(model_name: &str) -> String {
    let name_without_ggml = model_name.strip_prefix("ggml-").unwrap_or(model_name);
    let name_without_bin = name_without_ggml
        .strip_suffix(".bin")
        .unwrap_or(name_without_ggml);
    format!("ggml-{}.bin", name_without_bin)
}

/// Checks that the whisper runtime DLLs live next to the sidecar (or in its
/// `binaries/` subfolder) and are real files, not zero-byte placeholders.
fn dir_has_runtime_dlls(exe_path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        let _ = exe_path;
        true // macOS does not use DLLs
    }
    #[cfg(not(target_os = "macos"))]
    {
        let Some(dir) = exe_path.parent() else {
            return false;
        };
        let check = |d: &Path| {
            ["whisper.dll", "ggml.dll"].iter().all(|name| {
                fs::metadata(d.join(name))
                    .map(|m| m.len() > 1024)
                    .unwrap_or(false)
            })
        };
        check(dir) || check(&dir.join("binaries")) || check(&dir.join("resources").join("binaries"))
    }
}

/// Locate the whisper sidecar executable under the resources directory or fallback paths.
/// Candidates with the runtime DLLs beside them are preferred; broken copies
/// (e.g. a bare exe in target/debug without its DLLs) are used only as a last resort.
pub fn find_sidecar<R: Runtime>(app_handle: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    // In release bundles Tauri strips the target triple from externalBin names
    #[cfg(target_os = "windows")]
    let target_names = [
        "whisper-sidecar-x86_64-pc-windows-msvc.exe",
        "whisper-sidecar.exe",
    ];
    #[cfg(target_os = "macos")]
    let target_names = [
        "whisper-sidecar-x86_64-apple-darwin",
        "whisper-sidecar-aarch64-apple-darwin",
        "whisper-sidecar",
    ];
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let target_names = ["whisper-sidecar"];

    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let mut candidates: Vec<PathBuf> = Vec::new();

    // Dev builds: the source binaries folder always has the exe plus all DLLs
    #[cfg(debug_assertions)]
    for name in &target_names {
        candidates.push(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("binaries")
                .join(name),
        );
    }

    for name in &target_names {
        candidates.push(resource_dir.join("binaries").join(name));
        candidates.push(resource_dir.join("_up_").join("binaries").join(name));
        candidates.push(resource_dir.join(name));
        // CWD-relative fallbacks (dev mode launched from the workspace root)
        candidates.push(PathBuf::from("binaries").join(name));
        candidates.push(PathBuf::from("src-tauri").join("binaries").join(name));
    }

    // Bundled apps place externalBin next to the main executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            for name in &target_names {
                candidates.push(exe_dir.join(name));
            }
        }
    }

    let existing: Vec<PathBuf> = candidates.into_iter().filter(|p| p.exists()).collect();

    // Prefer a copy that has its runtime DLLs; fall back to any existing copy
    if let Some(path) = existing.iter().find(|p| dir_has_runtime_dlls(p)) {
        return Ok(path.clone());
    }
    if let Some(path) = existing.first() {
        #[cfg(target_os = "windows")]
        crate::logger::log(
            "WARN",
            "Sidecar",
            None,
            &format!(
                "sidecar found at {:?} but whisper.dll/ggml.dll are missing next to it.",
                path
            ),
        );
        return Ok(path.clone());
    }

    // Last resort: recursive search of the resource dir
    for name in &target_names {
        if let Some(path) = find_file_recursive(&resource_dir, name) {
            return Ok(path);
        }
    }

    Err(format!(
        "Could not find sidecar executable in resource dir ({:?}) or current working directory.",
        resource_dir
    ))
}

struct DeleteOnDrop {
    path: PathBuf,
    active: bool,
}

impl Drop for DeleteOnDrop {
    fn drop(&mut self) {
        if self.active && self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

static ACTIVE_MODEL_DOWNLOADS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

struct DownloadLease {
    model: String,
}

impl Drop for DownloadLease {
    fn drop(&mut self) {
        let downloads = ACTIVE_MODEL_DOWNLOADS.get_or_init(|| Mutex::new(HashSet::new()));
        recover_lock(downloads, "active model downloads").remove(&self.model);
    }
}

pub fn is_model_download_active(model: &str) -> bool {
    let downloads = ACTIVE_MODEL_DOWNLOADS.get_or_init(|| Mutex::new(HashSet::new()));
    recover_lock(downloads, "active model downloads").contains(model)
}

fn begin_model_download(model: &str) -> Result<DownloadLease, String> {
    let downloads = ACTIVE_MODEL_DOWNLOADS.get_or_init(|| Mutex::new(HashSet::new()));
    let mut active = match downloads.lock() {
        Ok(active) => active,
        Err(poisoned) => {
            crate::logger::log(
                "ERROR",
                "Download",
                None,
                "Recovering poisoned active-download mutex",
            );
            poisoned.into_inner()
        }
    };
    if !active.insert(model.to_string()) {
        return Err(format!("A download for '{model}' is already running"));
    }
    Ok(DownloadLease {
        model: model.to_string(),
    })
}

#[derive(Clone, Copy)]
struct ArtifactSpec {
    filename: &'static str,
    url: &'static str,
    expected_size: u64,
    sha256: &'static str,
}

fn whisper_artifact(model_name: &str) -> Result<ArtifactSpec, String> {
    let filename = format_model_filename(model_name);
    let spec = match filename.as_str() {
        "ggml-tiny.bin" => ArtifactSpec {
            filename: "ggml-tiny.bin",
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-tiny.bin",
            expected_size: 77_691_713,
            sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
        },
        "ggml-base.bin" => ArtifactSpec {
            filename: "ggml-base.bin",
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-base.bin",
            expected_size: 147_951_465,
            sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
        },
        "ggml-small.bin" => ArtifactSpec {
            filename: "ggml-small.bin",
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-small.bin",
            expected_size: 487_601_967,
            sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
        },
        "ggml-medium.bin" => ArtifactSpec {
            filename: "ggml-medium.bin",
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-medium.bin",
            expected_size: 1_533_763_059,
            sha256: "6c14d5adee5f86394037b4e4e8b59f1673b6cee10e3cf0b11bbdbee79c156208",
        },
        "ggml-large-v3-turbo-q5_0.bin" => ArtifactSpec {
            filename: "ggml-large-v3-turbo-q5_0.bin",
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-large-v3-turbo-q5_0.bin",
            expected_size: 574_041_195,
            sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
        },
        "ggml-large-v3-turbo.bin" => ArtifactSpec {
            filename: "ggml-large-v3-turbo.bin",
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-large-v3-turbo.bin",
            expected_size: 1_624_555_275,
            sha256: "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69",
        },
        _ => {
            return Err(format!(
                "Unsupported Whisper model '{model_name}'. Select a model from Aura settings."
            ))
        }
    };
    Ok(spec)
}

pub(crate) fn whisper_model_filename(model_name: &str) -> Result<String, String> {
    whisper_artifact(model_name).map(|spec| spec.filename.to_string())
}

pub(crate) fn whisper_model_is_installed(model_name: &str, path: &Path) -> bool {
    let Ok(spec) = whisper_artifact(model_name) else {
        return false;
    };
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.len() == spec.expected_size)
        .unwrap_or(false)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    use sha2::Digest;
    use std::io::Read;

    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open '{}' for verification: {e}", path.display()))?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to verify '{}': {e}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

async fn verify_artifact(path: &Path, spec: ArtifactSpec) -> Result<bool, String> {
    let size = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "Failed to inspect model '{}': {error}",
                path.display()
            ))
        }
    };
    if size != spec.expected_size {
        return Ok(false);
    }
    let verify_path = path.to_path_buf();
    let digest = tauri::async_runtime::spawn_blocking(move || sha256_file(&verify_path))
        .await
        .map_err(|e| format!("Model verification worker failed: {e}"))??;
    Ok(digest.eq_ignore_ascii_case(spec.sha256))
}

fn replace_downloaded_file(temp_path: &Path, destination: &Path) -> Result<(), String> {
    let filename = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Downloaded model has an invalid filename".to_string())?;
    let backup = destination.with_file_name(format!("{filename}.previous"));
    if backup.exists() {
        std::fs::remove_file(&backup).map_err(|e| {
            format!(
                "Failed to remove stale model backup '{}': {e}",
                backup.display()
            )
        })?;
    }

    let had_previous = destination.exists();
    if had_previous {
        std::fs::rename(destination, &backup).map_err(|e| {
            format!(
                "Failed to stage previous model '{}': {e}",
                destination.display()
            )
        })?;
    }

    if let Err(error) = std::fs::rename(temp_path, destination) {
        if had_previous {
            let _ = std::fs::rename(&backup, destination);
        }
        return Err(format!(
            "Failed to install verified model '{}': {error}",
            destination.display()
        ));
    }

    if had_previous {
        let _ = std::fs::remove_file(backup);
    }
    Ok(())
}

async fn replace_downloaded_file_async(
    temp_path: PathBuf,
    destination: PathBuf,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || replace_downloaded_file(&temp_path, &destination))
        .await
        .map_err(|error| format!("Model installation worker failed: {error}"))?
}

/// Download a verified GGML model from a pinned repository revision.
pub async fn download_model<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
    model_name: &str,
) -> Result<PathBuf, String> {
    use sha2::Digest;
    use tokio::io::AsyncWriteExt;

    let spec = whisper_artifact(model_name)?;
    let app_local_data = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data dir: {e}"))?;
    let models_dir = app_local_data.join("models");
    tokio::fs::create_dir_all(&models_dir)
        .await
        .map_err(|e| format!("Failed to create models directory: {e}"))?;
    let destination = models_dir.join(spec.filename);

    if verify_artifact(&destination, spec).await? {
        let _ = app_handle.emit(
            "model-download-progress",
            DownloadProgress {
                model: model_name.to_string(),
                downloaded: spec.expected_size,
                total: Some(spec.expected_size),
                percentage: 100.0,
                done: true,
            },
        );
        return Ok(destination);
    }

    let _lease = begin_model_download(model_name)?;
    clear_cancel(model_name);

    let client = crate::ai_client::build_download_client();
    let mut response = client
        .get(spec.url)
        .send()
        .await
        .map_err(|e| format!("Failed to download model: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to download model: HTTP status {}",
            response.status()
        ));
    }
    if let Some(length) = response.content_length() {
        if length != spec.expected_size {
            return Err(format!(
                "Model server returned an unexpected size: {length} bytes (expected {})",
                spec.expected_size
            ));
        }
    }

    let temp_path = models_dir.join(format!("{}.tmp", spec.filename));
    match tokio::fs::remove_file(&temp_path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Failed to remove stale temp model: {error}")),
    }
    let mut delete_guard = DeleteOnDrop {
        path: temp_path.clone(),
        active: true,
    };
    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| format!("Failed to create temp model: {e}"))?;
    let mut downloaded = 0u64;
    let mut hasher = sha2::Sha256::new();

    while let Some(chunk) =
        tokio::time::timeout(std::time::Duration::from_secs(30), response.chunk())
            .await
            .map_err(|_| "Model download timed out".to_string())?
            .map_err(|e| format!("Error while downloading model: {e}"))?
    {
        if is_cancel_requested(model_name) {
            clear_cancel(model_name);
            return Err("Download cancelled".to_string());
        }
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        if downloaded > spec.expected_size {
            return Err("Model download exceeded its verified size".to_string());
        }
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write model chunk: {e}"))?;
        let _ = app_handle.emit(
            "model-download-progress",
            DownloadProgress {
                model: model_name.to_string(),
                downloaded,
                total: Some(spec.expected_size),
                percentage: downloaded as f64 / spec.expected_size as f64 * 100.0,
                done: false,
            },
        );
    }

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush temp model: {e}"))?;
    file.sync_all()
        .await
        .map_err(|e| format!("Failed to sync temp model: {e}"))?;
    drop(file);

    if downloaded != spec.expected_size {
        return Err(format!(
            "Incomplete model download: {downloaded} of {} bytes",
            spec.expected_size
        ));
    }
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if !digest.eq_ignore_ascii_case(spec.sha256) {
        return Err(format!(
            "SHA-256 verification failed for '{}'",
            spec.filename
        ));
    }

    replace_downloaded_file_async(temp_path.clone(), destination.clone()).await?;
    delete_guard.active = false;
    clear_cancel(model_name);

    let _ = app_handle.emit(
        "model-download-progress",
        DownloadProgress {
            model: model_name.to_string(),
            downloaded,
            total: Some(spec.expected_size),
            percentage: 100.0,
            done: true,
        },
    );
    Ok(destination)
}
/// Run transcription using the local Whisper sidecar binary and return the result.
/// `language` accepts "ru"/"en" to force a language; anything else auto-detects.
/// `dictionary` (comma-separated terms) is passed as the initial prompt to bias recognition.
pub fn run_local_whisper<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
    model_name: &str,
    wav_path: &str,
    language: &str,
    dictionary: &str,
) -> Result<String, String> {
    // 1. Locate sidecar binary
    let sidecar_path = find_sidecar(app_handle)?;
    let short_sidecar_path = get_short_path(&sidecar_path)?;
    let sidecar_dir = sidecar_path
        .parent()
        .ok_or_else(|| "Invalid sidecar path".to_string())?;
    let short_sidecar_dir = get_short_path(sidecar_dir)?;

    // Resolve short path of resource DLLs folder (under resource_dir/binaries/)
    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;
    let dlls_dir = resource_dir.join("binaries");
    let short_dlls_dir = get_short_path(&dlls_dir)?;

    // 2. Resolve model path
    let filename = format_model_filename(model_name);

    let app_local_data = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data dir: {}", e))?;
    let model_path = app_local_data.join("models").join(&filename);

    if !model_path.exists() {
        return Err(format!(
            "Model file not found at: {:?}. Please download it first.",
            model_path
        ));
    }

    // Convert model and wav paths to short 8.3 representations
    let short_model_path = get_short_path(&model_path)?;
    let short_wav_path = get_short_path(Path::new(wav_path))?;

    let lang = match language {
        "ru" | "en" | "de" | "es" | "fr" | "it" | "zh" | "pt" | "tr" => language,
        _ => "auto",
    };

    // whisper.cpp defaults to only 4 threads; on modern many-core CPUs that leaves
    // most of the machine idle (~20% usage). Use all available logical cores so
    // transcription runs several times faster.
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let mut args: Vec<String> = vec![
        "-m".to_string(),
        short_model_path
            .to_str()
            .ok_or("Invalid model path encoding")?
            .to_string(),
        "-f".to_string(),
        short_wav_path
            .to_str()
            .ok_or("Invalid wav path encoding")?
            .to_string(),
        "-l".to_string(),
        lang.to_string(),
        "-t".to_string(),
        n_threads.to_string(),
        "-nt".to_string(),
        "-np".to_string(),
        // Greedy decoding (beam_size=1): ~20-40% faster than default beam-search
        // with negligible quality loss for real-time dictation use cases.
        "--best-of".to_string(),
        "1".to_string(),
        "--beam-size".to_string(),
        "-1".to_string(),
    ];

    let dict = dictionary.trim();
    if !dict.is_empty() {
        args.push("--prompt".to_string());
        args.push(dict.to_string());
    }

    let mut cmd = Command::new(&short_sidecar_path);
    cmd.current_dir(&short_dlls_dir);
    cmd.args(&args);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW (hides the black console window)
    }

    #[cfg(target_os = "windows")]
    {
        // Prepend sidecar executable directory, resources, and dll paths to PATH for Windows DLL resolution
        let path_key = std::env::vars_os()
            .map(|(k, _)| k.to_string_lossy().into_owned())
            .find(|name| name.eq_ignore_ascii_case("path"))
            .unwrap_or_else(|| "PATH".to_string());

        let mut paths = if let Some(path_env) = std::env::var_os(&path_key) {
            std::env::split_paths(&path_env).collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // Add all possible location candidates of whisper.dll / ggml.dll to guarantee resolution
        if let Some(parent) = sidecar_path.parent() {
            paths.insert(0, parent.join("resources").join("binaries"));
            paths.insert(0, parent.to_path_buf());
        }
        paths.insert(0, dlls_dir.clone());
        paths.insert(0, short_sidecar_dir.to_path_buf());
        paths.insert(0, short_dlls_dir.to_path_buf());

        if let Ok(new_path) = std::env::join_paths(paths) {
            cmd.env(&path_key, new_path);
        }
    }

    let output = run_command_with_timeout(
        &mut cmd,
        whisper_timeout_for_wav(model_name, wav_path),
        "Whisper sidecar",
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // An empty stderr with an odd exit code usually means missing DLLs
        // (STATUS_DLL_NOT_FOUND = 0xC0000135) — include the code for diagnostics.
        return Err(format!(
            "Whisper sidecar exited with error (code {:?}, path {:?}): {}",
            output.status.code(),
            sidecar_path,
            stderr
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.trim().to_string())
}

pub fn find_sherpa_sidecar<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let target_names = [
        "sherpa-onnx-offline-x86_64-pc-windows-msvc.exe",
        "sherpa-onnx-offline.exe",
    ];
    #[cfg(target_os = "macos")]
    let target_names = [
        "sherpa-onnx-offline-aarch64-apple-darwin",
        "sherpa-onnx-offline-x86_64-apple-darwin",
        "sherpa-onnx-offline",
    ];
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let target_names = ["sherpa-onnx-offline"];

    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let mut candidates: Vec<PathBuf> = Vec::new();

    #[cfg(debug_assertions)]
    for name in &target_names {
        candidates.push(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("binaries")
                .join(name),
        );
    }

    for name in &target_names {
        candidates.push(resource_dir.join("binaries").join(name));
        candidates.push(resource_dir.join("_up_").join("binaries").join(name));
        candidates.push(resource_dir.join(name));
        candidates.push(PathBuf::from("binaries").join(name));
        candidates.push(PathBuf::from("src-tauri").join("binaries").join(name));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            for name in &target_names {
                candidates.push(exe_dir.join(name));
            }
        }
    }

    let existing: Vec<PathBuf> = candidates.into_iter().filter(|p| p.exists()).collect();

    if let Some(path) = existing.first() {
        return Ok(path.clone());
    }

    for name in &target_names {
        if let Some(path) = find_file_recursive(&resource_dir, name) {
            return Ok(path);
        }
    }

    Err(format!(
        "Could not find sherpa-onnx sidecar executable in resource dir ({:?}).",
        resource_dir
    ))
}

const PARAKEET_ARTIFACTS: [ArtifactSpec; 4] = [
    ArtifactSpec {
        filename: "encoder.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/resolve/2bda32ec70b097a55adaa07d9a7173915b43cc78/encoder.int8.onnx",
        expected_size: 652_184_281,
        sha256: "acfc2b4456377e15d04f0243af540b7fe7c992f8d898d751cf134c3a55fd2247",
    },
    ArtifactSpec {
        filename: "decoder.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/resolve/2bda32ec70b097a55adaa07d9a7173915b43cc78/decoder.int8.onnx",
        expected_size: 11_845_275,
        sha256: "179e50c43d1a9de79c8a24149a2f9bac6eb5981823f2a2ed88d655b24248db4e",
    },
    ArtifactSpec {
        filename: "joiner.onnx",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/resolve/2bda32ec70b097a55adaa07d9a7173915b43cc78/joiner.int8.onnx",
        expected_size: 6_355_277,
        sha256: "3164c13fc2821009440d20fcb5fdc78bff28b4db2f8d0f0b329101719c0948b3",
    },
    ArtifactSpec {
        filename: "tokens.txt",
        url: "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8/resolve/2bda32ec70b097a55adaa07d9a7173915b43cc78/tokens.txt",
        expected_size: 93_939,
        sha256: "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d",
    },
];

pub(crate) fn parakeet_model_is_installed(directory: &Path) -> bool {
    PARAKEET_ARTIFACTS.iter().all(|spec| {
        std::fs::metadata(directory.join(spec.filename))
            .map(|metadata| metadata.is_file() && metadata.len() == spec.expected_size)
            .unwrap_or(false)
    })
}

pub async fn download_parakeet_model<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    use sha2::Digest;
    use tokio::io::AsyncWriteExt;

    let app_local_data = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data dir: {e}"))?;
    let parakeet_dir = app_local_data.join("models").join("parakeet-v3");
    tokio::fs::create_dir_all(&parakeet_dir)
        .await
        .map_err(|e| format!("Failed to create parakeet directory: {e}"))?;

    let total_size: u64 = PARAKEET_ARTIFACTS
        .iter()
        .map(|spec| spec.expected_size)
        .sum();
    let mut all_valid = true;
    for spec in PARAKEET_ARTIFACTS {
        if !verify_artifact(&parakeet_dir.join(spec.filename), spec).await? {
            all_valid = false;
            break;
        }
    }
    if all_valid {
        let _ = app_handle.emit(
            "model-download-progress",
            DownloadProgress {
                model: "parakeet-v3".to_string(),
                downloaded: total_size,
                total: Some(total_size),
                percentage: 100.0,
                done: true,
            },
        );
        return Ok(parakeet_dir);
    }

    let _lease = begin_model_download("parakeet-v3")?;
    clear_cancel("parakeet-v3");
    let client = crate::ai_client::build_download_client();
    let mut total_downloaded = 0u64;

    for spec in PARAKEET_ARTIFACTS {
        let destination = parakeet_dir.join(spec.filename);
        if verify_artifact(&destination, spec).await? {
            total_downloaded += spec.expected_size;
            continue;
        }

        let mut response = client
            .get(spec.url)
            .send()
            .await
            .map_err(|e| format!("Failed to download '{}': {e}", spec.filename))?;
        if !response.status().is_success() {
            return Err(format!(
                "Failed to download '{}': HTTP {}",
                spec.filename,
                response.status()
            ));
        }
        if let Some(length) = response.content_length() {
            if length != spec.expected_size {
                return Err(format!(
                    "Unexpected size for '{}': {length} bytes (expected {})",
                    spec.filename, spec.expected_size
                ));
            }
        }

        let temp_path = parakeet_dir.join(format!("{}.tmp", spec.filename));
        match tokio::fs::remove_file(&temp_path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Failed to remove stale temp file '{}': {error}",
                    temp_path.display()
                ))
            }
        }
        let mut delete_guard = DeleteOnDrop {
            path: temp_path.clone(),
            active: true,
        };
        let mut file = tokio::fs::File::create(&temp_path)
            .await
            .map_err(|e| format!("Failed to create temp file '{}': {e}", spec.filename))?;
        let mut file_downloaded = 0u64;
        let mut hasher = sha2::Sha256::new();

        while let Some(chunk) =
            tokio::time::timeout(std::time::Duration::from_secs(30), response.chunk())
                .await
                .map_err(|_| format!("Download of '{}' timed out", spec.filename))?
                .map_err(|e| format!("Error while downloading '{}': {e}", spec.filename))?
        {
            if is_cancel_requested("parakeet-v3") {
                clear_cancel("parakeet-v3");
                return Err("Download cancelled".to_string());
            }
            file_downloaded = file_downloaded.saturating_add(chunk.len() as u64);
            if file_downloaded > spec.expected_size {
                return Err(format!(
                    "Download of '{}' exceeded its verified size",
                    spec.filename
                ));
            }
            hasher.update(&chunk);
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write '{}': {e}", spec.filename))?;

            let downloaded = total_downloaded + file_downloaded;
            let _ = app_handle.emit(
                "model-download-progress",
                DownloadProgress {
                    model: "parakeet-v3".to_string(),
                    downloaded,
                    total: Some(total_size),
                    percentage: downloaded as f64 / total_size as f64 * 100.0,
                    done: false,
                },
            );
        }

        file.flush()
            .await
            .map_err(|e| format!("Failed to flush '{}': {e}", spec.filename))?;
        file.sync_all()
            .await
            .map_err(|e| format!("Failed to sync '{}': {e}", spec.filename))?;
        drop(file);

        if file_downloaded != spec.expected_size {
            return Err(format!(
                "Incomplete '{}': {file_downloaded} of {} bytes",
                spec.filename, spec.expected_size
            ));
        }
        let digest: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if !digest.eq_ignore_ascii_case(spec.sha256) {
            return Err(format!(
                "SHA-256 verification failed for '{}'",
                spec.filename
            ));
        }

        replace_downloaded_file_async(temp_path.clone(), destination.clone()).await?;
        delete_guard.active = false;
        total_downloaded += file_downloaded;
    }

    clear_cancel("parakeet-v3");
    let _ = app_handle.emit(
        "model-download-progress",
        DownloadProgress {
            model: "parakeet-v3".to_string(),
            downloaded: total_size,
            total: Some(total_size),
            percentage: 100.0,
            done: true,
        },
    );
    Ok(parakeet_dir)
}
pub fn find_cpu_sherpa_websocket_server<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let target_names = ["sherpa-onnx-offline-websocket-server.exe"];
    #[cfg(not(target_os = "windows"))]
    let target_names = ["sherpa-onnx-offline-websocket-server"];

    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let mut candidates: Vec<PathBuf> = Vec::new();

    #[cfg(debug_assertions)]
    for name in &target_names {
        candidates.push(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("binaries")
                .join(name),
        );
    }

    for name in &target_names {
        candidates.push(resource_dir.join("binaries").join(name));
        candidates.push(resource_dir.join("_up_").join("binaries").join(name));
        candidates.push(resource_dir.join(name));
        candidates.push(PathBuf::from("binaries").join(name));
        candidates.push(PathBuf::from("src-tauri").join("binaries").join(name));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            for name in &target_names {
                candidates.push(exe_dir.join(name));
            }
        }
    }

    let existing: Vec<PathBuf> = candidates.into_iter().filter(|p| p.exists()).collect();

    if let Some(path) = existing.first() {
        return Ok(path.clone());
    }

    for name in &target_names {
        if let Some(path) = find_file_recursive(&resource_dir, name) {
            return Ok(path);
        }
    }

    Err(format!(
        "Failed to find sidecar file '{}' or alternatives in candidates.",
        target_names[0]
    ))
}

pub fn find_sherpa_websocket_server<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    let settings = crate::settings::load_settings(app_handle).unwrap_or_default();

    let exe_name = if cfg!(target_os = "windows") {
        "sherpa-onnx-offline-websocket-server.exe"
    } else {
        "sherpa-onnx-offline-websocket-server"
    };

    if settings.local_acceleration != "cpu" {
        if let Ok(app_local_data) = app_handle.path().app_local_data_dir() {
            let gpu_exe = app_local_data
                .join("binaries")
                .join(&settings.local_acceleration)
                .join("bin")
                .join(exe_name);
            if gpu_exe.exists() {
                #[cfg(target_os = "windows")]
                {
                    if let Some(parent) = gpu_exe.parent() {
                        let has_ort = parent.join("onnxruntime.dll").exists();
                        let is_cuda_valid = if settings.local_acceleration == "cuda" {
                            parent.join("cudart64_110.dll").exists()
                                || parent.join("cublas64_11.dll").exists()
                        } else {
                            true
                        };
                        if has_ort && is_cuda_valid {
                            return Ok(gpu_exe);
                        }
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    return Ok(gpu_exe);
                }
            }
        }
    }

    find_cpu_sherpa_websocket_server(app_handle)
}

pub(crate) struct RunningParakeetServer {
    child: std::process::Child,
    provider: String,
    executable: PathBuf,
    port: u16,
    #[cfg(target_os = "windows")]
    _kill_on_close_job: Option<KillOnCloseJob>,
}

#[cfg(target_os = "windows")]
struct KillOnCloseJob {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(target_os = "windows")]
impl Drop for KillOnCloseJob {
    fn drop(&mut self) {
        if self.handle != 0 {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn attach_kill_on_close_job(child: &std::process::Child) -> Result<KillOnCloseJob, String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job == 0 {
            return Err(format!(
                "CreateJobObjectW failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if configured == 0 {
            let error = std::io::Error::last_os_error();
            windows_sys::Win32::Foundation::CloseHandle(job);
            return Err(format!("SetInformationJobObject failed: {error}"));
        }

        let process = child.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE;
        if AssignProcessToJobObject(job, process) == 0 {
            let error = std::io::Error::last_os_error();
            windows_sys::Win32::Foundation::CloseHandle(job);
            return Err(format!("AssignProcessToJobObject failed: {error}"));
        }

        Ok(KillOnCloseJob { handle: job })
    }
}

fn recover_lock<'a, T>(mutex: &'a std::sync::Mutex<T>, name: &str) -> std::sync::MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            crate::logger::log(
                "ERROR",
                "State",
                None,
                &format!("Recovering poisoned {name} mutex"),
            );
            poisoned.into_inner()
        }
    }
}

fn get_free_port() -> Result<u16, String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("Failed to reserve a local Parakeet port: {e}"))?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|e| format!("Failed to inspect the reserved Parakeet port: {e}"))
}

type SidecarDiagnostics = std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<String>>>;

fn remember_sidecar_line(diagnostics: &SidecarDiagnostics, line: &str) {
    let mut lines = recover_lock(diagnostics, "sidecar diagnostics");
    if lines.len() == 24 {
        lines.pop_front();
    }
    lines.push_back(line.chars().take(800).collect());
}

fn is_benign_websocket_shutdown_line(line: &str) -> bool {
    cfg!(target_os = "windows") && line.contains("handle_read_frame error: asio.system:10058")
}

fn is_routine_parakeet_status_line(line: &str) -> bool {
    if !line.contains("offline-websocket-server-impl.cc:") {
        return false;
    }

    (line.contains(":Decode:") && line.contains(" size: "))
        || (line.contains(":OnOpen:") && line.contains("Number of active connections:"))
        || (line.contains(":OnClose:") && line.contains("Number of active connections:"))
}

fn pipe_child_output(child: &mut std::process::Child) -> SidecarDiagnostics {
    let diagnostics = std::sync::Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new()));
    if let Some(stdout) = child.stdout.take() {
        let stdout_diagnostics = std::sync::Arc::clone(&diagnostics);
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if is_benign_websocket_shutdown_line(&line) {
                    continue;
                }
                remember_sidecar_line(&stdout_diagnostics, &line);
                crate::logger::log("INFO", "Sidecar", None, &line);
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let stderr_diagnostics = std::sync::Arc::clone(&diagnostics);
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if is_benign_websocket_shutdown_line(&line) {
                    continue;
                }
                remember_sidecar_line(&stderr_diagnostics, &line);
                if !is_routine_parakeet_status_line(&line) {
                    crate::logger::log("WARN", "Sidecar", None, &line);
                }
            }
        });
    }
    diagnostics
}

fn recent_sidecar_diagnostics(diagnostics: &SidecarDiagnostics) -> String {
    recover_lock(diagnostics, "sidecar diagnostics")
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join(" | ")
}

fn wait_for_server_warm_up(
    child: &mut std::process::Child,
    port: u16,
    timeout: std::time::Duration,
    diagnostics: &SidecarDiagnostics,
) -> Result<u128, String> {
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                std::thread::sleep(std::time::Duration::from_millis(75));
                let details = recent_sidecar_diagnostics(diagnostics);
                return Err(if details.is_empty() {
                    format!("sidecar exited before becoming ready ({status})")
                } else {
                    format!("sidecar exited before becoming ready ({status}): {details}")
                });
            }
            Ok(None) => {}
            Err(error) => {
                return Err(format!("Failed to inspect sidecar state: {error}"));
            }
        }

        let connection_error = match crate::warm_up_parakeet_server_port(port) {
            Ok(warm_up_ms) => return Ok(warm_up_ms),
            Err(crate::ParakeetWarmUpError::Connection(error)) => error,
            Err(crate::ParakeetWarmUpError::Inference(error)) => {
                std::thread::sleep(std::time::Duration::from_millis(75));
                let details = recent_sidecar_diagnostics(diagnostics);
                return Err(if details.is_empty() {
                    format!("sidecar failed functional inference warm-up: {error}")
                } else {
                    format!("sidecar failed functional inference warm-up: {error}: {details}")
                });
            }
        };
        if started.elapsed() >= timeout {
            let details = recent_sidecar_diagnostics(diagnostics);
            return Err(if details.is_empty() {
                format!(
                    "sidecar did not accept WebSocket connections on port {port} within {} seconds: {connection_error}",
                    timeout.as_secs(),
                )
            } else {
                format!(
                    "sidecar did not accept WebSocket connections on port {port} within {} seconds: {connection_error}: {details}",
                    timeout.as_secs(),
                )
            });
        }
        std::thread::sleep(std::time::Duration::from_millis(75));
    }
}

fn spawn_ready_server(
    executable: &Path,
    provider: &str,
    base_args: &[String],
    port: u16,
) -> Result<RunningParakeetServer, String> {
    let mut command = Command::new(executable);
    command
        .args(base_args)
        .arg(format!("--port={port}"))
        .arg(format!("--provider={provider}"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let mut child = command.spawn().map_err(|e| {
        format!(
            "Failed to spawn Parakeet server '{}' ({provider}): {e}",
            executable.display()
        )
    })?;

    #[cfg(target_os = "windows")]
    let kill_on_close_job = match attach_kill_on_close_job(&child) {
        Ok(job) => Some(job),
        Err(error) => {
            crate::logger::log(
                "WARN",
                "Sidecar",
                None,
                &format!("Could not attach Parakeet to a kill-on-close Job Object: {error}"),
            );
            None
        }
    };

    let diagnostics = pipe_child_output(&mut child);
    let timeout = if provider == "cpu" {
        std::time::Duration::from_secs(45)
    } else {
        std::time::Duration::from_secs(25)
    };
    let warm_up_ms = match wait_for_server_warm_up(&mut child, port, timeout, &diagnostics) {
        Ok(elapsed_ms) => elapsed_ms,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{provider} sidecar failed readiness: {error}"));
        }
    };
    crate::logger::log(
        "INFO",
        "Sidecar",
        None,
        &format!("Parakeet {provider} inference warm-up completed in {warm_up_ms} ms"),
    );

    Ok(RunningParakeetServer {
        child,
        provider: provider.to_string(),
        executable: executable.to_path_buf(),
        port,
        #[cfg(target_os = "windows")]
        _kill_on_close_job: kill_on_close_job,
    })
}

fn stop_owned_server(mut server: RunningParakeetServer) {
    let _ = server.child.kill();
    let _ = server.child.wait();
}

fn provider_for_server_path(path: &Path) -> &'static str {
    let normalized = path.to_string_lossy().replace('\\', "/").to_lowercase();
    if normalized.contains("/binaries/cuda/") {
        "cuda"
    } else if normalized.contains("/binaries/directml/") {
        "directml"
    } else {
        "cpu"
    }
}

pub fn start_parakeet_server<R: Runtime>(app_handle: &tauri::AppHandle<R>) -> Result<(), String> {
    let state = app_handle
        .try_state::<crate::AppState>()
        .ok_or_else(|| "Application state is not initialized".to_string())?;
    let _lifecycle = recover_lock(&state.parakeet_lifecycle, "Parakeet lifecycle");

    let server_path = find_sherpa_websocket_server(app_handle)?;
    let target_provider = provider_for_server_path(&server_path);
    let short_server_path = get_short_path(&server_path)?;

    let app_local_data = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data dir: {e}"))?;
    let model_dir = app_local_data.join("models").join("parakeet-v3");
    let encoder_path = model_dir.join("encoder.onnx");
    let decoder_path = model_dir.join("decoder.onnx");
    let joiner_path = model_dir.join("joiner.onnx");
    let tokens_path = model_dir.join("tokens.txt");
    for path in [&encoder_path, &decoder_path, &joiner_path, &tokens_path] {
        let metadata = std::fs::metadata(path).map_err(|_| {
            format!(
                "Parakeet model file '{}' is missing. Please redownload the model.",
                path.display()
            )
        })?;
        if metadata.len() == 0 {
            return Err(format!(
                "Parakeet model file '{}' is empty. Please redownload the model.",
                path.display()
            ));
        }
    }

    let short_encoder = get_short_path(&encoder_path)?;
    let short_decoder = get_short_path(&decoder_path)?;
    let short_joiner = get_short_path(&joiner_path)?;
    let short_tokens = get_short_path(&tokens_path)?;
    let threads = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4);
    let log_file_path = app_local_data.join("parakeet_server_log.txt");
    let _ = std::fs::remove_file(&log_file_path);
    let base_args = vec![
        format!("--encoder={}", short_encoder.to_string_lossy()),
        format!("--decoder={}", short_decoder.to_string_lossy()),
        format!("--joiner={}", short_joiner.to_string_lossy()),
        format!("--tokens={}", short_tokens.to_string_lossy()),
        "--feat-dim=128".to_string(),
        format!("--num-work-threads={threads}"),
        "--max-utterance-length=660".to_string(),
        format!("--log-file={}", log_file_path.to_string_lossy()),
    ];

    let previous = {
        let mut server = recover_lock(&state.parakeet_server, "Parakeet server");
        let keep_existing = if let Some(running) = server.as_mut() {
            match running.child.try_wait() {
                Ok(None) => {
                    running.provider == target_provider && running.executable == short_server_path
                }
                Ok(Some(status)) => {
                    crate::logger::log(
                        "WARN",
                        "Sidecar",
                        None,
                        &format!("Parakeet server exited unexpectedly ({status}); restarting"),
                    );
                    false
                }
                Err(error) => {
                    crate::logger::log(
                        "WARN",
                        "Sidecar",
                        None,
                        &format!("Could not inspect Parakeet server ({error}); restarting"),
                    );
                    false
                }
            }
        } else {
            false
        };
        if keep_existing {
            return Ok(());
        }
        server.take()
    };
    if let Some(previous) = previous {
        state
            .parakeet_port
            .store(0, std::sync::atomic::Ordering::SeqCst);
        stop_owned_server(previous);
    }

    crate::logger::log(
        "INFO",
        "Sidecar",
        None,
        &format!("Starting Parakeet WebSocket server ({target_provider})"),
    );

    let first_port = get_free_port()?;
    let running = match spawn_ready_server(
        &short_server_path,
        target_provider,
        &base_args,
        first_port,
    ) {
        Ok(server) => server,
        Err(gpu_error) if target_provider != "cpu" => {
            crate::logger::log(
                "WARN",
                "Sidecar",
                None,
                &format!(
                    "GPU Parakeet server ({target_provider}) failed: {gpu_error}. Falling back to CPU."
                ),
            );
            let cpu_path = get_short_path(&find_cpu_sherpa_websocket_server(app_handle).map_err(
                |error| {
                    format!("GPU start failed ({gpu_error}); CPU sidecar is unavailable: {error}")
                },
            )?)?;
            let cpu_port = get_free_port()?;
            spawn_ready_server(&cpu_path, "cpu", &base_args, cpu_port).map_err(|cpu_error| {
                format!("GPU start failed ({gpu_error}); CPU fallback also failed: {cpu_error}")
            })?
        }
        Err(error) => return Err(error),
    };

    let provider = running.provider.clone();
    let port = running.port;
    state
        .parakeet_port
        .store(port, std::sync::atomic::Ordering::SeqCst);
    *recover_lock(&state.parakeet_server, "Parakeet server") = Some(running);
    crate::logger::log(
        "INFO",
        "Sidecar",
        None,
        &format!("Parakeet WebSocket server is ready on port {port} ({provider})"),
    );
    Ok(())
}

pub fn stop_parakeet_server<R: Runtime>(app_handle: &tauri::AppHandle<R>) {
    let Some(state) = app_handle.try_state::<crate::AppState>() else {
        return;
    };
    let _lifecycle = recover_lock(&state.parakeet_lifecycle, "Parakeet lifecycle");
    let server = recover_lock(&state.parakeet_server, "Parakeet server").take();
    state
        .parakeet_port
        .store(0, std::sync::atomic::Ordering::SeqCst);
    if let Some(server) = server {
        crate::logger::log(
            "INFO",
            "Sidecar",
            None,
            "Stopping background Parakeet server",
        );
        stop_owned_server(server);
    }
}

pub fn ensure_parakeet_server_state<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
    settings: &crate::settings::Settings,
) {
    if settings.transcription_mode == "local" && settings.local_engine == "parakeet" {
        if let Err(error) = start_parakeet_server(app_handle) {
            crate::logger::log(
                "ERROR",
                "Sidecar",
                None,
                &format!("Failed to start Parakeet server: {error}"),
            );
        }
    } else {
        stop_parakeet_server(app_handle);
    }
}

/// Transcribes a 16 kHz mono WAV via the resident Parakeet WebSocket server.
///
/// Note: `language` and `dictionary` are intentionally unused. Parakeet v3 auto-detects the
/// language, and the custom dictionary (hotwords) can't be applied here because the server is a
/// long-lived daemon started once with fixed args — biasing would require a `--hotwords-file`
/// baked in at server start plus a restart whenever the dictionary changes. Left as a documented
/// limitation rather than shipping an unverified server flag that could stop the daemon from
/// starting. The dictionary still works on the Whisper engine and the cloud providers.
pub fn run_parakeet<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
    wav_path: &str,
    _language: &str,
    _dictionary: &str,
) -> Result<String, String> {
    if let Err(e) = start_parakeet_server(app_handle) {
        return Err(format!(
            "Parakeet server is not running and failed to start: {}",
            e
        ));
    }

    let mut reader =
        hound::WavReader::open(wav_path).map_err(|e| format!("Failed to open WAV file: {}", e))?;
    let spec = reader.spec();
    if spec.sample_rate != 16000 || spec.channels != 1 {
        return Err(format!(
            "Unsupported WAV format: channels={}, sample_rate={}",
            spec.channels, spec.sample_rate
        ));
    }
    if spec.sample_format != hound::SampleFormat::Int || spec.bits_per_sample != 16 {
        return Err(format!(
            "Unsupported WAV sample format: {:?}/{}-bit",
            spec.sample_format, spec.bits_per_sample
        ));
    }

    let sample_count = u64::from(reader.duration());
    if sample_count == 0 {
        return Ok(String::new());
    }
    let expected_byte_size_u64 = sample_count
        .checked_mul(std::mem::size_of::<f32>() as u64)
        .ok_or_else(|| "WAV sample count overflow".to_string())?;
    let expected_byte_size: i32 = expected_byte_size_u64
        .try_into()
        .map_err(|_| "WAV is too large for the Parakeet protocol".to_string())?;

    let port = if let Some(state) = app_handle.try_state::<crate::AppState>() {
        state
            .parakeet_port
            .load(std::sync::atomic::Ordering::SeqCst)
    } else {
        return Err("Application state is unavailable".to_string());
    };
    let url = format!("ws://127.0.0.1:{}", port);
    let mut socket = {
        let start_connect = std::time::Instant::now();
        let mut notified = false;
        loop {
            match tungstenite::connect(&url) {
                Ok((s, _)) => {
                    break s;
                }
                Err(e) => {
                    if start_connect.elapsed().as_secs() > 15 {
                        return Err(format!("Parakeet server connection timeout: {}", e));
                    }
                    // First failed connect = the server is still loading the model into RAM.
                    // Tell the user so the wait doesn't look like a freeze.
                    if !notified {
                        notified = true;
                        let _ = app_handle.emit("recording-state", "notice:Загрузка модели…");
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
            }
        }
    };

    // Guard against a hung/dead server: without a read timeout, socket.read() below
    // would block this dictation forever, leaving the overlay stuck on "processing".
    if let tungstenite::stream::MaybeTlsStream::Plain(stream) = socket.get_ref() {
        let audio_seconds = sample_count / u64::from(spec.sample_rate);
        let response_timeout = std::time::Duration::from_secs((30 + audio_seconds / 2).min(360));
        let _ = stream.set_read_timeout(Some(response_timeout));
        let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(30)));
    }

    let sample_rate = spec.sample_rate as i32;
    let mut header = Vec::with_capacity(8);
    header.extend_from_slice(&sample_rate.to_le_bytes());
    header.extend_from_slice(&expected_byte_size.to_le_bytes());
    socket
        .send(tungstenite::Message::Binary(header))
        .map_err(|e| format!("Failed to send Parakeet audio header: {e}"))?;

    // The Sherpa offline protocol explicitly permits audio in multiple WebSocket
    // messages. Convert directly from the WAV iterator so a ten-minute recording
    // never creates full-size i16, f32 and wire-format copies at the same time.
    const SAMPLES_PER_MESSAGE: usize = 16_384;
    let mut payload = Vec::with_capacity(SAMPLES_PER_MESSAGE * std::mem::size_of::<f32>());
    let mut sent_bytes = 0u64;
    for sample in reader.samples::<i16>() {
        let normalized =
            sample.map_err(|e| format!("Failed to read WAV samples: {e}"))? as f32 / 32768.0;
        payload.extend_from_slice(&normalized.to_le_bytes());
        if payload.len() >= SAMPLES_PER_MESSAGE * std::mem::size_of::<f32>() {
            sent_bytes += payload.len() as u64;
            socket
                .send(tungstenite::Message::Binary(payload))
                .map_err(|e| format!("Failed to send Parakeet audio chunk: {e}"))?;
            payload = Vec::with_capacity(SAMPLES_PER_MESSAGE * std::mem::size_of::<f32>());
        }
    }
    if !payload.is_empty() {
        sent_bytes += payload.len() as u64;
        socket
            .send(tungstenite::Message::Binary(payload))
            .map_err(|e| format!("Failed to send final Parakeet audio chunk: {e}"))?;
    }
    if sent_bytes != expected_byte_size_u64 {
        return Err(format!(
            "WAV length changed while reading: expected {expected_byte_size_u64} bytes, sent {sent_bytes}"
        ));
    }

    let response_text = loop {
        match socket
            .read()
            .map_err(|e| format!("Failed to read transcription response: {e}"))?
        {
            tungstenite::Message::Text(text) => break text,
            tungstenite::Message::Ping(_) | tungstenite::Message::Pong(_) => continue,
            tungstenite::Message::Close(frame) => {
                return Err(format!(
                    "Parakeet server closed before returning a result: {frame:?}"
                ));
            }
            _ => return Err("Unexpected binary message from Parakeet server".to_string()),
        }
    };

    crate::finish_parakeet_socket(&mut socket);

    let mut transcript = response_text.clone();
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&response_text) {
        if let Some(t) = val.get("text").and_then(|v| v.as_str()) {
            transcript = t.trim().to_string();
        }
    }

    // Workaround for Sherpa ONNX Nemo Parakeet model outputting <unk> instead of 'ё'.
    // Only replace with 'ё' when the transcript is in Cyrillic; otherwise just strip
    // the tag, since <unk> in Latin/Chinese text signals a genuinely unknown token.
    let has_cyrillic = transcript
        .chars()
        .any(|c| ('\u{0400}'..='\u{04FF}').contains(&c));
    transcript = if has_cyrillic {
        transcript.replace("<unk>", "ё")
    } else {
        transcript.replace("<unk>", "")
    };

    Ok(transcript)
}

pub fn find_sherpa_punctuation_exe<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    let settings = crate::settings::load_settings(app_handle).unwrap_or_default();
    let exe_name = if cfg!(target_os = "windows") {
        "sherpa-onnx-offline-punctuation.exe"
    } else {
        "sherpa-onnx-offline-punctuation"
    };

    if settings.local_acceleration != "cpu" {
        if let Ok(app_local_data) = app_handle.path().app_local_data_dir() {
            let gpu_exe = app_local_data
                .join("binaries")
                .join(&settings.local_acceleration)
                .join("bin")
                .join(exe_name);
            if gpu_exe.exists() {
                #[cfg(target_os = "windows")]
                {
                    if let Some(parent) = gpu_exe.parent() {
                        if parent.join("onnxruntime.dll").exists() {
                            return Ok(gpu_exe);
                        }
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    return Ok(gpu_exe);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    let target_names = ["sherpa-onnx-offline-punctuation.exe"];
    #[cfg(not(target_os = "windows"))]
    let target_names = ["sherpa-onnx-offline-punctuation"];

    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    let mut candidates: Vec<PathBuf> = Vec::new();

    #[cfg(debug_assertions)]
    for name in &target_names {
        candidates.push(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("binaries")
                .join(name),
        );
    }

    for name in &target_names {
        candidates.push(resource_dir.join("binaries").join(name));
        candidates.push(resource_dir.join("_up_").join("binaries").join(name));
        candidates.push(resource_dir.join(name));
        candidates.push(PathBuf::from("binaries").join(name));
        candidates.push(PathBuf::from("src-tauri").join("binaries").join(name));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            for name in &target_names {
                candidates.push(exe_dir.join(name));
            }
        }
    }

    let existing: Vec<PathBuf> = candidates.into_iter().filter(|p| p.exists()).collect();

    if let Some(path) = existing.first() {
        return Ok(path.clone());
    }

    for name in &target_names {
        if let Some(path) = find_file_recursive(&resource_dir, name) {
            return Ok(path);
        }
    }

    Err(format!(
        "Failed to find sidecar file '{}' or alternatives in candidates.",
        target_names[0]
    ))
}

const PUNCTUATION_ARCHIVE_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/punctuation-models/sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12-int8.tar.bz2";
const PUNCTUATION_ARCHIVE_SIZE: u64 = 64_717_756;
const PUNCTUATION_ARCHIVE_SHA256: &str =
    "c0d5aa5f8eeb686032345e180bedf39319dc2e0556781c6264bcadba8328a6e1";
const PUNCTUATION_ARCHIVE_ROOT: &str =
    "sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12-int8";

struct RemoveDirectoryOnDrop(PathBuf);

impl Drop for RemoveDirectoryOnDrop {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            if error.kind() != std::io::ErrorKind::NotFound {
                crate::logger::log(
                    "WARN",
                    "Punctuation",
                    None,
                    &format!(
                        "Failed to clean temporary punctuation directory {}: {error}",
                        self.0.display()
                    ),
                );
            }
        }
    }
}

fn punctuation_files_complete(directory: &Path) -> bool {
    ["model.int8.onnx", "tokens.txt"].into_iter().all(|name| {
        directory
            .join(name)
            .metadata()
            .map(|metadata| metadata.is_file() && metadata.len() > 0)
            .unwrap_or(false)
    })
}

fn unique_punctuation_staging_dir(models_dir: &Path) -> PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    models_dir.join(format!(
        ".punctuation.install-{}-{suffix}",
        std::process::id()
    ))
}

fn install_punctuation_archive(
    archive_path: &Path,
    staging_dir: &Path,
    destination: &Path,
) -> Result<(), String> {
    let extraction_dir = staging_dir.join("extract");
    std::fs::create_dir_all(&extraction_dir)
        .map_err(|error| format!("Failed to create punctuation extraction directory: {error}"))?;
    let archive_file = std::fs::File::open(archive_path)
        .map_err(|error| format!("Failed to open verified punctuation archive: {error}"))?;
    let decoder = bzip2::read::BzDecoder::new(archive_file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(&extraction_dir)
        .map_err(|error| format!("Failed to extract punctuation archive safely: {error}"))?;

    let source = extraction_dir.join(PUNCTUATION_ARCHIVE_ROOT);
    if !punctuation_files_complete(&source) {
        return Err(
            "Verified punctuation archive does not contain model.int8.onnx and tokens.txt"
                .to_string(),
        );
    }
    let parent = destination
        .parent()
        .ok_or_else(|| "Punctuation destination has no parent directory".to_string())?;
    let backup = parent.join(".punctuation.previous");
    if backup.exists() {
        std::fs::remove_dir_all(&backup)
            .map_err(|error| format!("Failed to clear stale punctuation backup: {error}"))?;
    }
    let had_previous = destination.exists();
    if had_previous {
        std::fs::rename(destination, &backup)
            .map_err(|error| format!("Failed to stage previous punctuation model: {error}"))?;
    }
    if let Err(error) = std::fs::rename(&source, destination) {
        if had_previous {
            let _ = std::fs::rename(&backup, destination);
        }
        return Err(format!("Failed to install punctuation model: {error}"));
    }
    if had_previous {
        if let Err(error) = std::fs::remove_dir_all(&backup) {
            crate::logger::log(
                "WARN",
                "Punctuation",
                None,
                &format!("Punctuation install succeeded but old backup cleanup failed: {error}"),
            );
        }
    }
    Ok(())
}

pub async fn download_punctuation_model<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
) -> Result<(), String> {
    let app_local_data = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data dir: {}", e))?;
    let models_dir = app_local_data.join("models");
    let punc_dir = models_dir.join("punctuation");
    if punctuation_files_complete(&punc_dir) {
        return Ok(());
    }
    let _lease = begin_model_download("punctuation")?;
    tokio::fs::create_dir_all(&models_dir)
        .await
        .map_err(|e| format!("Failed to create models directory: {e}"))?;
    let staging_dir = unique_punctuation_staging_dir(&models_dir);
    tokio::fs::create_dir(&staging_dir)
        .await
        .map_err(|e| format!("Failed to create punctuation staging directory: {e}"))?;
    let _staging_guard = RemoveDirectoryOnDrop(staging_dir.clone());
    let archive_path = staging_dir.join("punctuation.tar.bz2");

    crate::logger::log(
        "INFO",
        "Punctuation",
        None,
        "Downloading local punctuation model...",
    );
    let client = crate::ai_client::build_download_client();
    let mut response = client
        .get(PUNCTUATION_ARCHIVE_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to download punctuation model: {}", e))?;
    if !response.status().is_success() {
        return Err(format!(
            "Failed to download punctuation model: HTTP {}",
            response.status()
        ));
    }
    if let Some(advertised_size) = response.content_length() {
        if advertised_size != PUNCTUATION_ARCHIVE_SIZE {
            return Err(format!(
                "Punctuation archive size mismatch: expected {PUNCTUATION_ARCHIVE_SIZE}, server advertised {advertised_size}"
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
        .map_err(|e| format!("Failed to create punctuation archive: {e}"))?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;
    loop {
        let chunk = tokio::time::timeout(std::time::Duration::from_secs(30), response.chunk())
            .await
            .map_err(|_| "Punctuation download stalled for more than 30 seconds".to_string())?
            .map_err(|e| format!("Error reading punctuation chunk: {e}"))?;
        let Some(chunk) = chunk else {
            break;
        };
        if is_cancel_requested("parakeet-v3") {
            return Err("Punctuation download cancelled".to_string());
        }
        downloaded = downloaded
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| "Punctuation download byte count overflow".to_string())?;
        if downloaded > PUNCTUATION_ARCHIVE_SIZE {
            return Err("Punctuation download exceeded its pinned size".to_string());
        }
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Failed to write punctuation chunk: {e}"))?;
    }
    file.flush()
        .await
        .map_err(|e| format!("Failed to flush punctuation archive: {e}"))?;
    file.sync_all()
        .await
        .map_err(|e| format!("Failed to persist punctuation archive: {e}"))?;
    drop(file);

    if downloaded != PUNCTUATION_ARCHIVE_SIZE {
        return Err(format!(
            "Incomplete punctuation download: expected {PUNCTUATION_ARCHIVE_SIZE} bytes, received {downloaded}"
        ));
    }
    let actual_hash = format!("{:x}", hasher.finalize());
    if actual_hash != PUNCTUATION_ARCHIVE_SHA256 {
        return Err(format!(
            "Punctuation archive integrity check failed: expected {PUNCTUATION_ARCHIVE_SHA256}, got {actual_hash}"
        ));
    }

    let worker_archive = archive_path.clone();
    let worker_staging = staging_dir.clone();
    let worker_destination = punc_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        install_punctuation_archive(&worker_archive, &worker_staging, &worker_destination)
    })
    .await
    .map_err(|error| format!("Punctuation install worker failed: {error}"))??;

    crate::logger::log(
        "INFO",
        "Punctuation",
        None,
        "Punctuation model downloaded successfully.",
    );
    Ok(())
}

pub fn run_punctuation<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
    text: &str,
) -> Result<String, String> {
    // Windows CreateProcess limit is ~32 767 chars; long dictations would silently fail.
    // At >4 000 chars the marginal value of offline punctuation is also low (LLM/cloud
    // mode handles its own punctuation), so return the text as-is for safety.
    const MAX_CLI_CHARS: usize = 4_000;
    if text.chars().count() > MAX_CLI_CHARS {
        crate::logger::log(
            "WARN",
            "Punctuation",
            None,
            &format!(
                "run_punctuation skipped — text too long ({} chars > {})",
                text.chars().count(),
                MAX_CLI_CHARS
            ),
        );
        return Ok(text.to_string());
    }

    let app_local_data = app_handle
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app local data dir: {}", e))?;
    let punc_dir = app_local_data.join("models").join("punctuation");
    let model_path = punc_dir.join("model.int8.onnx");

    if !model_path.exists() {
        return Err("Punctuation model not found".to_string());
    }

    let punc_exe = find_sherpa_punctuation_exe(app_handle)?;
    let short_punc_exe = get_short_path(&punc_exe)?;
    let short_model_path = get_short_path(&model_path)?;

    let mut cmd = Command::new(&short_punc_exe);
    cmd.args(&[
        format!("--ct-transformer={}", short_model_path.to_string_lossy()),
        text.to_string(),
    ]);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = run_command_with_timeout(
        &mut cmd,
        std::time::Duration::from_secs(60),
        "Punctuation sidecar",
    )?;
    if !output.status.success() {
        return Err("Punctuation process failed".to_string());
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);

    for line in stdout_str.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Output text:") {
            let punc_text = trimmed.replace("Output text:", "").trim().to_string();
            let normalized = punc_text
                .replace("？", "? ")
                .replace("。", ". ")
                .replace("，", ", ")
                .replace("；", "; ")
                .replace("：", ": ")
                .replace("！", "! ")
                .replace(" ,", ",")
                .replace(" .", ".")
                .replace(" ?", "?")
                .replace(" !", "!")
                .replace("  ", " ");
            return Ok(normalized.trim().to_string());
        }
    }

    Err("Failed to parse punctuation output".to_string())
}

fn get_short_path(path: &Path) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::os::windows::ffi::OsStringExt;
        use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

        let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();

        unsafe {
            let size = GetShortPathNameW(wide_path.as_ptr(), std::ptr::null_mut(), 0);
            if size == 0 {
                return Ok(path.to_path_buf());
            }

            let mut buffer: Vec<u16> = vec![0; size as usize];
            let written = GetShortPathNameW(wide_path.as_ptr(), buffer.as_mut_ptr(), size);
            if written == 0 || written >= size {
                return Ok(path.to_path_buf());
            }

            let short_str = std::ffi::OsString::from_wide(&buffer[..written as usize]);
            Ok(PathBuf::from(short_str))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(path.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filename_parsing() {
        assert_eq!(format_model_filename("small"), "ggml-small.bin");
        assert_eq!(format_model_filename("ggml-base.bin"), "ggml-base.bin");
        assert_eq!(format_model_filename("base.bin"), "ggml-base.bin");
        assert_eq!(format_model_filename("ggml-tiny"), "ggml-tiny.bin");
    }

    #[test]
    fn whisper_timeout_is_model_aware_and_bounded() {
        assert_eq!(whisper_timeout_for_duration("base", 0).as_secs(), 180);
        assert_eq!(whisper_timeout_for_duration("base", 60).as_secs(), 240);
        assert_eq!(whisper_timeout_for_duration("small", 60).as_secs(), 300);
        assert_eq!(whisper_timeout_for_duration("medium", 600).as_secs(), 3_120);
        assert_eq!(
            whisper_timeout_for_duration("large", 10_000).as_secs(),
            3_600
        );
    }

    #[test]
    fn only_windows_post_close_10058_is_filtered_from_sidecar_diagnostics() {
        let benign = "handle_read_frame error: asio.system:10058 (socket already shut down)";
        assert_eq!(
            is_benign_websocket_shutdown_line(benign),
            cfg!(target_os = "windows")
        );
        assert!(!is_benign_websocket_shutdown_line(
            "handle_read_frame error: asio.system:10053 (connection aborted)"
        ));
        assert!(!is_benign_websocket_shutdown_line(
            "CUDA provider failed to initialize"
        ));
    }

    #[test]
    fn only_known_parakeet_status_lines_are_suppressed_from_unified_log() {
        for routine in [
            "offline-websocket-server-impl.cc:Decode:68 size: 1",
            "offline-websocket-server-impl.cc:OnOpen:172 Number of active connections: 1",
            "offline-websocket-server-impl.cc:OnClose:180 Number of active connections: 0",
        ] {
            assert!(is_routine_parakeet_status_line(routine), "{routine}");
        }

        for warning in [
            "offline-websocket-server-impl.cc:Decode:68 CUDA provider failed",
            "offline-websocket-server-impl.cc:OnOpen:172 bind failed",
            "CUDA provider failed to initialize",
        ] {
            assert!(!is_routine_parakeet_status_line(warning), "{warning}");
        }
    }
}
