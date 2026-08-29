use reqwest::header::{ACCEPT_ENCODING, CONTENT_RANGE, RANGE};
use reqwest::StatusCode;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

pub const DEFAULT_STALL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug)]
pub struct ArtifactSpec<'a> {
    pub label: &'a str,
    pub url: &'a str,
    pub expected_size: u64,
    pub sha256: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactProgress {
    pub downloaded: u64,
    pub total: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadOutcome {
    pub path: PathBuf,
    pub resumed_from: u64,
    pub reused_existing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ParsedContentRange {
    start: u64,
    end: u64,
    total: u64,
}

fn validate_spec(spec: ArtifactSpec<'_>) -> Result<(), String> {
    if spec.label.trim().is_empty() {
        return Err("Artifact label is empty".to_string());
    }
    if spec.expected_size == 0 {
        return Err(format!("Pinned size for '{}' is zero", spec.label));
    }
    if spec.sha256.len() != 64 || !spec.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("Pinned SHA-256 for '{}' is invalid", spec.label));
    }
    Ok(())
}

fn parse_content_range(value: &str) -> Result<ParsedContentRange, String> {
    let value = value.trim();
    let remainder = value
        .strip_prefix("bytes ")
        .ok_or_else(|| format!("Unsupported Content-Range '{value}'"))?;
    let (range, total) = remainder
        .split_once('/')
        .ok_or_else(|| format!("Malformed Content-Range '{value}'"))?;
    let (start, end) = range
        .split_once('-')
        .ok_or_else(|| format!("Malformed Content-Range '{value}'"))?;
    let start = start
        .parse::<u64>()
        .map_err(|_| format!("Invalid Content-Range start in '{value}'"))?;
    let end = end
        .parse::<u64>()
        .map_err(|_| format!("Invalid Content-Range end in '{value}'"))?;
    let total = total
        .parse::<u64>()
        .map_err(|_| format!("Invalid Content-Range total in '{value}'"))?;
    if start > end || end >= total {
        return Err(format!("Impossible Content-Range '{value}'"));
    }
    Ok(ParsedContentRange { start, end, total })
}

/// Returns bytes available to the caller on the volume hosting `path`.
/// `None` when the platform probe is unavailable (non-Windows builds).
#[cfg(target_os = "windows")]
fn available_bytes_on_volume(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut available: u64 = 0;
    let mut _total: u64 = 0;
    let mut _free: u64 = 0;
    let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut available, &mut _total, &mut _free) };
    if ok != 0 {
        Some(available)
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
fn available_bytes_on_volume(_path: &Path) -> Option<u64> {
    None
}

/// Fails the download up front when the volume hosting `dir` cannot hold
/// `required` bytes. A probe failure is not fatal (fail-open), matching the
/// rest of this module's tolerance for degraded environments.
pub fn ensure_disk_space(dir: &Path, required: u64, label: &str) -> Result<(), String> {
    let Some(available) = available_bytes_on_volume(dir) else {
        return Ok(());
    };
    if available < required {
        return Err(format!(
            "Not enough free disk space for {label}: need at least {required} bytes ({:.1} GB free required, {:.1} GB available)",
            required as f64 / 1_073_741_824.0,
            available as f64 / 1_073_741_824.0,
        ));
    }
    Ok(())
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|error| {
        format!(
            "Failed to open artifact '{}' for verification: {error}",
            path.display()
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Failed to verify artifact '{}': {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

async fn sha256_file_async(path: PathBuf) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || sha256_file(&path))
        .await
        .map_err(|error| format!("Artifact verification worker failed: {error}"))?
}

pub async fn artifact_is_verified(path: &Path, spec: ArtifactSpec<'_>) -> Result<bool, String> {
    validate_spec(spec)?;
    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "Failed to inspect artifact '{}': {error}",
                path.display()
            ))
        }
    };
    if !metadata.is_file() || metadata.len() != spec.expected_size {
        return Ok(false);
    }
    let digest = sha256_file_async(path.to_path_buf()).await?;
    Ok(digest.eq_ignore_ascii_case(spec.sha256))
}

pub fn partial_path(destination: &Path, spec: ArtifactSpec<'_>) -> Result<PathBuf, String> {
    validate_spec(spec)?;
    let filename = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Artifact destination has an invalid filename".to_string())?;
    Ok(destination.with_file_name(format!("{filename}.{}.part", &spec.sha256[..16])))
}

async fn remove_file_if_exists(path: &Path) -> Result<(), String> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Failed to remove '{}': {error}", path.display())),
    }
}

fn replace_downloaded_file(temp_path: &Path, destination: &Path) -> Result<(), String> {
    let filename = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Artifact destination has an invalid filename".to_string())?;
    let backup = destination.with_file_name(format!("{filename}.previous"));
    if backup.exists() {
        std::fs::remove_file(&backup).map_err(|error| {
            format!(
                "Failed to remove stale artifact backup '{}': {error}",
                backup.display()
            )
        })?;
    }

    let had_previous = destination.exists();
    if had_previous {
        std::fs::rename(destination, &backup).map_err(|error| {
            format!(
                "Failed to stage previous artifact '{}': {error}",
                destination.display()
            )
        })?;
    }

    if let Err(error) = std::fs::rename(temp_path, destination) {
        if had_previous {
            let _ = std::fs::rename(&backup, destination);
        }
        return Err(format!(
            "Failed to install verified artifact '{}': {error}",
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
        .map_err(|error| format!("Artifact installation worker failed: {error}"))?
}

async fn cleanup_cancelled_partial(path: &Path, label: &str) -> Result<(), String> {
    remove_file_if_exists(path)
        .await
        .map_err(|error| format!("Download of '{label}' was cancelled; {error}"))
}

/// Downloads a pinned artifact with HTTP Range resume and exact size/SHA-256 verification.
///
/// A transient transport error deliberately retains the deterministic `.part` file. An explicit
/// cancellation or integrity failure removes it. The destination is replaced only after the
/// complete partial file has passed verification.
pub async fn download_verified_artifact<C, P>(
    client: &reqwest::Client,
    spec: ArtifactSpec<'_>,
    destination: &Path,
    stall_timeout: Duration,
    is_cancelled: C,
    mut on_progress: P,
) -> Result<DownloadOutcome, String>
where
    C: Fn() -> bool,
    P: FnMut(ArtifactProgress),
{
    validate_spec(spec)?;
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|error| {
            format!(
                "Failed to create artifact directory '{}': {error}",
                parent.display()
            )
        })?;
    }

    // The verified check comes FIRST: an already-downloaded artifact must
    // install even on an almost-full disk, where the download budget would
    // fail a preflight that no longer needs to run.
    if artifact_is_verified(destination, spec).await? {
        if is_cancelled() {
            return Err(format!("Download of '{}' cancelled", spec.label));
        }
        on_progress(ArtifactProgress {
            downloaded: spec.expected_size,
            total: spec.expected_size,
        });
        return Ok(DownloadOutcome {
            path: destination.to_path_buf(),
            resumed_from: spec.expected_size,
            reused_existing: true,
        });
    }

    if let Some(parent) = destination.parent() {
        // Room for the .part file AND the verified destination copy, with a
        // small fixed margin; a probe failure is ignored (fail-open).
        let required = spec
            .expected_size
            .saturating_mul(2)
            .saturating_add(64 * 1024 * 1024);
        ensure_disk_space(parent, required, spec.label)?;
    }

    let partial = partial_path(destination, spec)?;
    let mut offset = match tokio::fs::metadata(&partial).await {
        Ok(metadata) if metadata.is_file() && metadata.len() <= spec.expected_size => {
            metadata.len()
        }
        Ok(_) => {
            remove_file_if_exists(&partial).await?;
            0
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => {
            return Err(format!(
                "Failed to inspect partial artifact '{}': {error}",
                partial.display()
            ))
        }
    };

    if offset == spec.expected_size {
        if artifact_is_verified(&partial, spec).await? {
            if is_cancelled() {
                cleanup_cancelled_partial(&partial, spec.label).await?;
                return Err(format!("Download of '{}' cancelled", spec.label));
            }
            replace_downloaded_file_async(partial.clone(), destination.to_path_buf()).await?;
            on_progress(ArtifactProgress {
                downloaded: spec.expected_size,
                total: spec.expected_size,
            });
            return Ok(DownloadOutcome {
                path: destination.to_path_buf(),
                resumed_from: spec.expected_size,
                reused_existing: false,
            });
        }
        remove_file_if_exists(&partial).await?;
        offset = 0;
    }

    if is_cancelled() {
        cleanup_cancelled_partial(&partial, spec.label).await?;
        return Err(format!("Download of '{}' cancelled", spec.label));
    }

    let resumed_from = offset;
    let mut restarted_after_range_rejection = false;
    let mut downloaded;

    loop {
        on_progress(ArtifactProgress {
            downloaded: offset,
            total: spec.expected_size,
        });

        let mut request = client.get(spec.url).header(ACCEPT_ENCODING, "identity");
        if offset > 0 {
            request = request.header(RANGE, format!("bytes={offset}-"));
        }

        let mut response = request
            .send()
            .await
            .map_err(|error| format!("Failed to download '{}': {error}", spec.label))?;
        let status = response.status();

        if status == StatusCode::RANGE_NOT_SATISFIABLE
            && offset > 0
            && !restarted_after_range_rejection
        {
            remove_file_if_exists(&partial).await?;
            offset = 0;
            restarted_after_range_rejection = true;
            continue;
        }

        let append = if status == StatusCode::PARTIAL_CONTENT {
            let header = response
                .headers()
                .get(CONTENT_RANGE)
                .ok_or_else(|| {
                    format!(
                        "Server returned 206 for '{}' without Content-Range",
                        spec.label
                    )
                })?
                .to_str()
                .map_err(|_| format!("Invalid Content-Range for '{}'", spec.label))?;
            let range = parse_content_range(header)?;
            if range.start != offset || range.total != spec.expected_size {
                return Err(format!(
                    "Unexpected Content-Range for '{}': expected bytes {offset}-/{}, got {header}",
                    spec.label, spec.expected_size
                ));
            }
            let range_length = range
                .end
                .checked_sub(range.start)
                .and_then(|length| length.checked_add(1))
                .ok_or_else(|| format!("Invalid Content-Range for '{}'", spec.label))?;
            if let Some(length) = response.content_length() {
                if length != range_length {
                    return Err(format!(
                        "Partial response length mismatch for '{}': header range has {range_length} bytes, body advertises {length}",
                        spec.label
                    ));
                }
            }
            offset > 0
        } else if status == StatusCode::OK {
            if let Some(length) = response.content_length() {
                if length != spec.expected_size {
                    return Err(format!(
                        "Unexpected response size for '{}': expected {}, server advertised {length}",
                        spec.label, spec.expected_size
                    ));
                }
            }
            if offset > 0 {
                offset = 0;
                on_progress(ArtifactProgress {
                    downloaded: 0,
                    total: spec.expected_size,
                });
            }
            false
        } else {
            return Err(format!(
                "Failed to download '{}': HTTP {status}",
                spec.label
            ));
        };

        if append {
            let actual_offset = tokio::fs::metadata(&partial)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to re-check partial artifact '{}': {error}",
                        partial.display()
                    )
                })?
                .len();
            if actual_offset != offset {
                return Err(format!(
                    "Partial artifact changed during resume: expected {offset} bytes, found {actual_offset}"
                ));
            }
        }

        let mut options = tokio::fs::OpenOptions::new();
        options.create(true).write(true);
        if append {
            options.append(true);
        } else {
            options.truncate(true);
        }
        let mut file = options.open(&partial).await.map_err(|error| {
            format!(
                "Failed to open partial artifact '{}': {error}",
                partial.display()
            )
        })?;
        downloaded = offset;

        // The per-chunk stall timeout only catches a fully dead link; a server
        // dribbling one byte every few seconds would otherwise keep the
        // download (and its progress UI) alive forever. A size-proportional
        // wall-clock budget turns such a link into a bounded failure (C12).
        // The 100 KB/s implied floor was too tight for slow-but-healthy links
        // (same lesson as the cloud timeouts in ai_client.rs, where 300 s was
        // too tight): 50 KB/s minimum sustained throughput still bounds a
        // dribbling link while letting a real connection finish.
        let budget_secs = (spec.expected_size / 50_000).max(600);
        let overall_deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(budget_secs);

        loop {
            if std::time::Instant::now() >= overall_deadline {
                let _ = file.flush().await;
                return Err(format!(
                    "Download of '{}' exceeded its overall time budget ({budget_secs} seconds); \
                     the link is too slow or stalled; partial data was kept",
                    spec.label
                ));
            }
            if is_cancelled() {
                drop(file);
                cleanup_cancelled_partial(&partial, spec.label).await?;
                return Err(format!("Download of '{}' cancelled", spec.label));
            }

            let next_chunk = match tokio::time::timeout(stall_timeout, response.chunk()).await {
                Ok(Ok(chunk)) => chunk,
                Ok(Err(error)) => {
                    let _ = file.flush().await;
                    return Err(format!(
                        "Connection failed while downloading '{}': {error}; partial data was kept",
                        spec.label
                    ));
                }
                Err(_) => {
                    let _ = file.flush().await;
                    return Err(format!(
                        "Download of '{}' stalled for more than {} seconds; partial data was kept",
                        spec.label,
                        stall_timeout.as_secs()
                    ));
                }
            };
            let Some(chunk) = next_chunk else {
                break;
            };

            downloaded = downloaded
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| format!("Download byte count overflow for '{}'", spec.label))?;
            if downloaded > spec.expected_size {
                drop(file);
                remove_file_if_exists(&partial).await?;
                return Err(format!(
                    "Download of '{}' exceeded its pinned size",
                    spec.label
                ));
            }
            file.write_all(&chunk).await.map_err(|error| {
                format!(
                    "Failed to write partial artifact '{}': {error}",
                    partial.display()
                )
            })?;
            on_progress(ArtifactProgress {
                downloaded,
                total: spec.expected_size,
            });
        }

        file.flush().await.map_err(|error| {
            format!(
                "Failed to flush partial artifact '{}': {error}",
                partial.display()
            )
        })?;
        file.sync_all().await.map_err(|error| {
            format!(
                "Failed to persist partial artifact '{}': {error}",
                partial.display()
            )
        })?;
        drop(file);
        break;
    }

    if downloaded != spec.expected_size {
        return Err(format!(
            "Incomplete download of '{}': {downloaded} of {} bytes; partial data was kept",
            spec.label, spec.expected_size
        ));
    }
    if is_cancelled() {
        cleanup_cancelled_partial(&partial, spec.label).await?;
        return Err(format!("Download of '{}' cancelled", spec.label));
    }

    let digest = sha256_file_async(partial.clone()).await?;
    if !digest.eq_ignore_ascii_case(spec.sha256) {
        remove_file_if_exists(&partial).await?;
        return Err(format!(
            "SHA-256 verification failed for '{}': expected {}, got {digest}",
            spec.label, spec.sha256
        ));
    }
    if is_cancelled() {
        cleanup_cancelled_partial(&partial, spec.label).await?;
        return Err(format!("Download of '{}' cancelled", spec.label));
    }

    replace_downloaded_file_async(partial, destination.to_path_buf()).await?;
    on_progress(ArtifactProgress {
        downloaded: spec.expected_size,
        total: spec.expected_size,
    });
    Ok(DownloadOutcome {
        path: destination.to_path_buf(),
        resumed_from,
        reused_existing: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::net::{Shutdown, TcpListener, TcpStream};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "aura-artifact-download-{label}-{}-{suffix}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("create test directory");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn sha256_bytes(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn test_client() -> reqwest::Client {
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("build test client")
    }

    fn read_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set request timeout");
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).expect("read HTTP request");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        String::from_utf8(request).expect("request is UTF-8")
    }

    fn spawn_server<F>(handler: F) -> (String, thread::JoinHandle<()>)
    where
        F: FnOnce(TcpListener) + Send + 'static,
    {
        let listener =
            TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).expect("bind local HTTP server");
        let address = listener.local_addr().expect("HTTP server address");
        let handle = thread::spawn(move || handler(listener));
        (format!("http://{address}/artifact"), handle)
    }

    fn write_response(
        stream: &mut TcpStream,
        status: &str,
        headers: &[(&str, String)],
        body: &[u8],
    ) {
        let mut response = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
        for (name, value) in headers {
            response.push_str(name);
            response.push_str(": ");
            response.push_str(value);
            response.push_str("\r\n");
        }
        response.push_str("\r\n");
        stream
            .write_all(response.as_bytes())
            .expect("write response headers");
        stream.write_all(body).expect("write response body");
        stream.flush().expect("flush response");
    }

    #[test]
    fn content_range_parser_rejects_ambiguous_or_impossible_ranges() {
        assert_eq!(
            parse_content_range("bytes 10-19/20").expect("valid range"),
            ParsedContentRange {
                start: 10,
                end: 19,
                total: 20
            }
        );
        for invalid in ["bytes */20", "items 1-2/3", "bytes 4-2/5", "bytes 1-5/5"] {
            assert!(parse_content_range(invalid).is_err(), "{invalid}");
        }
    }

    #[tokio::test]
    async fn interrupted_transfer_keeps_partial_and_resumes_with_exact_range() {
        let bytes = b"aura-resumable-artifact".to_vec();
        let split = 9usize;
        let server_bytes = bytes.clone();
        let (url, server) = spawn_server(move |listener| {
            let (mut first, _) = listener.accept().expect("accept initial request");
            let first_request = read_request(&mut first);
            assert!(!first_request.to_ascii_lowercase().contains("range:"));
            let headers = [("Content-Length", server_bytes.len().to_string())];
            write_response(&mut first, "200 OK", &headers, &server_bytes[..split]);
            first
                .shutdown(Shutdown::Both)
                .expect("close truncated response");

            let (mut second, _) = listener.accept().expect("accept resumed request");
            let second_request = read_request(&mut second).to_ascii_lowercase();
            assert!(
                second_request.contains(&format!("range: bytes={split}-")),
                "{second_request}"
            );
            let remaining = &server_bytes[split..];
            let headers = [
                ("Content-Length", remaining.len().to_string()),
                (
                    "Content-Range",
                    format!(
                        "bytes {split}-{}/{total}",
                        server_bytes.len() - 1,
                        total = server_bytes.len()
                    ),
                ),
            ];
            write_response(&mut second, "206 Partial Content", &headers, remaining);
        });

        let hash = sha256_bytes(&bytes);
        let spec = ArtifactSpec {
            label: "resume-test",
            url: &url,
            expected_size: bytes.len() as u64,
            sha256: &hash,
        };
        let directory = TestDirectory::new("resume");
        let destination = directory.0.join("artifact.bin");
        let client = test_client();

        let first = download_verified_artifact(
            &client,
            spec,
            &destination,
            Duration::from_secs(2),
            || false,
            |_| {},
        )
        .await;
        assert!(first.is_err());
        let partial = partial_path(&destination, spec).expect("partial path");
        assert_eq!(
            std::fs::metadata(&partial).expect("partial metadata").len(),
            split as u64
        );

        let mut progress = Vec::new();
        let outcome = download_verified_artifact(
            &client,
            spec,
            &destination,
            Duration::from_secs(2),
            || false,
            |event| progress.push(event.downloaded),
        )
        .await
        .expect("resume download");
        server.join().expect("join HTTP server");

        assert_eq!(outcome.resumed_from, split as u64);
        assert_eq!(progress.first().copied(), Some(split as u64));
        assert_eq!(std::fs::read(&destination).expect("read result"), bytes);
        assert!(!partial.exists());
    }

    #[tokio::test]
    async fn full_200_response_to_range_request_restarts_without_mixing_bytes() {
        let bytes = b"complete-body-after-range-ignore".to_vec();
        let server_bytes = bytes.clone();
        let (url, server) = spawn_server(move |listener| {
            let (mut stream, _) = listener.accept().expect("accept request");
            let request = read_request(&mut stream).to_ascii_lowercase();
            assert!(request.contains("range: bytes=6-"), "{request}");
            let headers = [("Content-Length", server_bytes.len().to_string())];
            write_response(&mut stream, "200 OK", &headers, &server_bytes);
        });

        let hash = sha256_bytes(&bytes);
        let spec = ArtifactSpec {
            label: "range-ignore-test",
            url: &url,
            expected_size: bytes.len() as u64,
            sha256: &hash,
        };
        let directory = TestDirectory::new("range-ignore");
        let destination = directory.0.join("artifact.bin");
        let partial = partial_path(&destination, spec).expect("partial path");
        std::fs::write(&partial, &bytes[..6]).expect("seed partial");
        let mut progress = Vec::new();

        download_verified_artifact(
            &test_client(),
            spec,
            &destination,
            Duration::from_secs(2),
            || false,
            |event| progress.push(event.downloaded),
        )
        .await
        .expect("restart full response");
        server.join().expect("join HTTP server");

        assert!(progress.starts_with(&[6, 0]), "{progress:?}");
        assert_eq!(std::fs::read(&destination).expect("read result"), bytes);
    }

    #[tokio::test]
    async fn mismatched_content_range_is_rejected_before_writing() {
        let bytes = b"range-validation-body".to_vec();
        let server_bytes = bytes.clone();
        let (url, server) = spawn_server(move |listener| {
            let (mut stream, _) = listener.accept().expect("accept request");
            let request = read_request(&mut stream).to_ascii_lowercase();
            assert!(request.contains("range: bytes=5-"), "{request}");
            let body = &server_bytes[5..];
            let headers = [
                ("Content-Length", body.len().to_string()),
                (
                    "Content-Range",
                    format!(
                        "bytes 4-{}/{total}",
                        server_bytes.len() - 1,
                        total = server_bytes.len()
                    ),
                ),
            ];
            write_response(&mut stream, "206 Partial Content", &headers, body);
        });

        let hash = sha256_bytes(&bytes);
        let spec = ArtifactSpec {
            label: "bad-range-test",
            url: &url,
            expected_size: bytes.len() as u64,
            sha256: &hash,
        };
        let directory = TestDirectory::new("bad-range");
        let destination = directory.0.join("artifact.bin");
        let partial = partial_path(&destination, spec).expect("partial path");
        std::fs::write(&partial, &bytes[..5]).expect("seed partial");

        let error = download_verified_artifact(
            &test_client(),
            spec,
            &destination,
            Duration::from_secs(2),
            || false,
            |_| {},
        )
        .await
        .expect_err("reject mismatched range");
        server.join().expect("join HTTP server");

        assert!(error.contains("Unexpected Content-Range"), "{error}");
        assert_eq!(std::fs::read(&partial).expect("read partial"), &bytes[..5]);
        assert!(!destination.exists());
    }

    #[tokio::test]
    async fn integrity_failure_removes_partial() {
        let expected = b"expected-artifact".to_vec();
        let wrong = b"corruptd-artifact".to_vec();
        assert_eq!(expected.len(), wrong.len());
        let server_wrong = wrong.clone();
        let (url, server) = spawn_server(move |listener| {
            let (mut stream, _) = listener.accept().expect("accept request");
            let _ = read_request(&mut stream);
            let headers = [("Content-Length", server_wrong.len().to_string())];
            write_response(&mut stream, "200 OK", &headers, &server_wrong);
        });
        let hash = sha256_bytes(&expected);
        let spec = ArtifactSpec {
            label: "hash-test",
            url: &url,
            expected_size: expected.len() as u64,
            sha256: &hash,
        };
        let directory = TestDirectory::new("hash");
        let destination = directory.0.join("artifact.bin");
        let partial = partial_path(&destination, spec).expect("partial path");

        let error = download_verified_artifact(
            &test_client(),
            spec,
            &destination,
            Duration::from_secs(2),
            || false,
            |_| {},
        )
        .await
        .expect_err("reject corrupt body");
        server.join().expect("join HTTP server");

        assert!(error.contains("SHA-256 verification failed"), "{error}");
        assert!(!partial.exists());
        assert!(!destination.exists());
    }

    #[tokio::test]
    async fn cancellation_removes_partial_even_after_body_arrives() {
        let bytes = b"cancelled-artifact-body".to_vec();
        let server_bytes = bytes.clone();
        let (url, server) = spawn_server(move |listener| {
            let (mut stream, _) = listener.accept().expect("accept request");
            let _ = read_request(&mut stream);
            let headers = [("Content-Length", server_bytes.len().to_string())];
            write_response(&mut stream, "200 OK", &headers, &server_bytes);
        });
        let hash = sha256_bytes(&bytes);
        let spec = ArtifactSpec {
            label: "cancel-test",
            url: &url,
            expected_size: bytes.len() as u64,
            sha256: &hash,
        };
        let directory = TestDirectory::new("cancel");
        let destination = directory.0.join("artifact.bin");
        let partial = partial_path(&destination, spec).expect("partial path");
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancellation_check = Arc::clone(&cancelled);
        let cancellation_progress = Arc::clone(&cancelled);

        let error = download_verified_artifact(
            &test_client(),
            spec,
            &destination,
            Duration::from_secs(2),
            move || cancellation_check.load(Ordering::SeqCst),
            move |event| {
                if event.downloaded > 0 {
                    cancellation_progress.store(true, Ordering::SeqCst);
                }
            },
        )
        .await
        .expect_err("cancel download");
        server.join().expect("join HTTP server");

        assert!(error.contains("cancelled"), "{error}");
        assert!(!partial.exists());
        assert!(!destination.exists());
    }

    #[tokio::test]
    async fn oversized_advertised_body_is_rejected_without_partial_file() {
        let bytes = b"size-pinned".to_vec();
        let server_bytes = bytes.clone();
        let (url, server) = spawn_server(move |listener| {
            let (mut stream, _) = listener.accept().expect("accept request");
            let _ = read_request(&mut stream);
            let headers = [("Content-Length", (server_bytes.len() + 1).to_string())];
            write_response(&mut stream, "200 OK", &headers, &server_bytes);
        });
        let hash = sha256_bytes(&bytes);
        let spec = ArtifactSpec {
            label: "oversize-test",
            url: &url,
            expected_size: bytes.len() as u64,
            sha256: &hash,
        };
        let directory = TestDirectory::new("oversize");
        let destination = directory.0.join("artifact.bin");
        let partial = partial_path(&destination, spec).expect("partial path");

        let error = download_verified_artifact(
            &test_client(),
            spec,
            &destination,
            Duration::from_secs(2),
            || false,
            |_| {},
        )
        .await
        .expect_err("reject oversized response");
        server.join().expect("join HTTP server");

        assert!(error.contains("Unexpected response size"), "{error}");
        assert!(!partial.exists());
    }
}
