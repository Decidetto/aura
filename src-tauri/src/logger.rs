use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::thread;
use std::time::SystemTime;

pub const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024; // 10 MB
pub const LOG_FILE_NAME: &str = "aura_diagnostics.log";

static LOGGER_TX: Mutex<Option<Sender<String>>> = Mutex::new(None);
static LOGGER_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn anonymize_speech(text: &str, log_speech_text: bool) -> String {
    if log_speech_text {
        text.to_string()
    } else {
        let word_count = text.split_whitespace().count();
        let char_count = text.chars().count();
        format!("[REDACTED: {} words, {} chars]", word_count, char_count)
    }
}

pub fn format_session_tag(session_id: u64) -> String {
    format!("#session-{}", session_id)
}

pub fn init(log_dir: PathBuf) {
    if let Err(e) = fs::create_dir_all(&log_dir) {
        eprintln!("[LOGGER ERROR] Failed to create log directory: {}", e);
        return;
    }

    let (tx, rx) = mpsc::channel::<String>();
    let dir = log_dir.clone();

    thread::spawn(move || {
        while let Ok(msg) = rx.recv() {
            if let Err(e) = rotate_logs_if_needed(&dir, MAX_LOG_SIZE) {
                eprintln!("[LOGGER ERROR] Failed log rotation: {}", e);
            }
            let log_path = dir.join(LOG_FILE_NAME);
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
                let _ = writeln!(file, "{}", msg);
            }
        }
    });

    if let Ok(mut lock) = LOGGER_TX.lock() {
        *lock = Some(tx);
    }
    if let Ok(mut lock) = LOGGER_DIR.lock() {
        *lock = Some(log_dir);
    }
}

pub fn log(level: &str, tag: &str, session: Option<&str>, message: &str) {
    let timestamp = format_timestamp(SystemTime::now());
    let level_str = level.to_uppercase();
    let session_str = match session {
        Some(s) if s.starts_with('[') => format!(" {}", s),
        Some(s) => format!(" [{}]", s),
        None => String::new(),
    };

    let formatted_line = format!("{} [{}] [{}]{} {}", timestamp, level_str, tag, session_str, message);
    eprintln!("{}", formatted_line);

    if let Ok(lock) = LOGGER_TX.lock() {
        if let Some(tx) = lock.as_ref() {
            let _ = tx.send(formatted_line);
        }
    }
}

pub fn rotate_logs_if_needed(log_dir: &Path, max_bytes: u64) -> std::io::Result<()> {
    let log_path = log_dir.join(LOG_FILE_NAME);
    if log_path.exists() {
        if let Ok(meta) = fs::metadata(&log_path) {
            if meta.len() >= max_bytes {
                let log_1_path = log_dir.join(format!("{}.1", LOG_FILE_NAME));
                let log_2_path = log_dir.join(format!("{}.2", LOG_FILE_NAME));

                if log_2_path.exists() {
                    let _ = fs::remove_file(&log_2_path);
                }
                if log_1_path.exists() {
                    let _ = fs::rename(&log_1_path, &log_2_path);
                }
                let _ = fs::rename(&log_path, &log_1_path);
                File::create(&log_path)?;
            }
        }
    }
    Ok(())
}

pub fn get_recent_logs(max_lines: usize) -> Vec<String> {
    let log_dir = match LOGGER_DIR.lock() {
        Ok(lock) => match lock.as_ref() {
            Some(dir) => dir.clone(),
            None => return Vec::new(),
        },
        Err(_) => return Vec::new(),
    };

    let log_path = log_dir.join(LOG_FILE_NAME);
    if !log_path.exists() {
        return Vec::new();
    }

    let file = match File::open(&log_path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

    if lines.len() > max_lines {
        lines[lines.len() - max_lines..].to_vec()
    } else {
        lines
    }
}

fn format_timestamp(now: SystemTime) -> String {
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    let days = (secs / 86400) as i64;
    let rem_secs = (secs % 86400) as u32;
    let hours = rem_secs / 3600;
    let mins = (rem_secs % 3600) / 60;
    let seconds = rem_secs % 60;

    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}Z", year, m, d, hours, mins, seconds, millis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_anonymize_speech() {
        assert_eq!(
            anonymize_speech("hello world from aura", true),
            "hello world from aura"
        );
        assert_eq!(
            anonymize_speech("hello world from aura", false),
            "[REDACTED: 4 words, 21 chars]"
        );
        assert_eq!(
            anonymize_speech("", false),
            "[REDACTED: 0 words, 0 chars]"
        );
    }

    #[test]
    fn test_format_session_tag() {
        assert_eq!(format_session_tag(123456), "#session-123456");
        assert_eq!(format_session_tag(0), "#session-0");
    }

    #[test]
    fn test_log_rotation() {
        let unique_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("aura_log_test_{}", unique_id));
        fs::create_dir_all(&temp_dir).unwrap();

        let log_file = temp_dir.join(LOG_FILE_NAME);
        fs::write(&log_file, "A".repeat(100)).unwrap();

        // 1st rotation: log file size 100 >= threshold 50 -> rotated to log.1
        rotate_logs_if_needed(&temp_dir, 50).unwrap();
        assert!(temp_dir.join(LOG_FILE_NAME).exists());
        assert!(temp_dir.join(format!("{}.1", LOG_FILE_NAME)).exists());
        assert_eq!(fs::read_to_string(temp_dir.join(format!("{}.1", LOG_FILE_NAME))).unwrap(), "A".repeat(100));

        // Fill current log file again
        fs::write(&log_file, "B".repeat(100)).unwrap();

        // 2nd rotation: log file size 100 >= threshold 50 -> log.1 becomes log.2, current becomes log.1
        rotate_logs_if_needed(&temp_dir, 50).unwrap();
        assert!(temp_dir.join(LOG_FILE_NAME).exists());
        assert!(temp_dir.join(format!("{}.1", LOG_FILE_NAME)).exists());
        assert!(temp_dir.join(format!("{}.2", LOG_FILE_NAME)).exists());
        assert_eq!(fs::read_to_string(temp_dir.join(format!("{}.1", LOG_FILE_NAME))).unwrap(), "B".repeat(100));
        assert_eq!(fs::read_to_string(temp_dir.join(format!("{}.2", LOG_FILE_NAME))).unwrap(), "A".repeat(100));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
