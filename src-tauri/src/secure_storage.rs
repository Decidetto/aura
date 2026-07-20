use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn temp_path_for(path: &Path) -> Result<PathBuf, String> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Secure storage path has no valid file name".to_string())?;
    Ok(path.with_file_name(format!(
        ".{file_name}.tmp-{}-{}",
        std::process::id(),
        unique_suffix()
    )))
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Secure storage path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create data directory: {error}"))?;

    let temp_path = temp_path_for(path)?;
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .map_err(|error| format!("Failed to create temporary data file: {error}"))?;
        file.write_all(bytes)
            .map_err(|error| format!("Failed to write temporary data file: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Failed to flush temporary data file: {error}"))?;
        drop(file);
        replace_file(&temp_path, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(target_os = "windows")]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination_wide: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
    let flags = MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH;
    let result = unsafe { MoveFileExW(source_wide.as_ptr(), destination_wide.as_ptr(), flags) };
    if result == 0 {
        return Err(format!(
            "Failed to atomically replace data file: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination)
        .map_err(|error| format!("Failed to atomically replace data file: {error}"))
}

#[cfg(target_os = "windows")]
pub fn protect_for_current_user(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: plaintext
            .len()
            .try_into()
            .map_err(|_| "Secret is too large for DPAPI".to_string())?,
        pbData: plaintext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        return Err(format!("DPAPI encryption failed: {}", std::io::Error::last_os_error()));
    }
    let encrypted = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        LocalFree(output.pbData.cast());
    }
    Ok(encrypted)
}

#[cfg(target_os = "windows")]
pub fn unprotect_for_current_user(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: ciphertext
            .len()
            .try_into()
            .map_err(|_| "Encrypted secret is too large for DPAPI".to_string())?,
        pbData: ciphertext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        return Err(format!("DPAPI decryption failed: {}", std::io::Error::last_os_error()));
    }
    let plaintext = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        LocalFree(output.pbData.cast());
    }
    Ok(plaintext)
}

#[cfg(not(target_os = "windows"))]
pub fn protect_for_current_user(_plaintext: &[u8]) -> Result<Vec<u8>, String> {
    Err("Secure secret storage is currently implemented for Windows only".to_string())
}

#[cfg(not(target_os = "windows"))]
pub fn unprotect_for_current_user(_ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    Err("Secure secret storage is currently implemented for Windows only".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "aura-{name}-{}-{}",
            std::process::id(),
            unique_suffix()
        ))
    }

    #[test]
    fn atomic_write_replaces_existing_contents() {
        let directory = test_dir("atomic-write");
        let path = directory.join("data.bin");

        atomic_write(&path, b"first").expect("initial write should succeed");
        atomic_write(&path, b"second").expect("replacement should succeed");

        assert_eq!(fs::read(&path).unwrap(), b"second");
        let leftovers = fs::read_dir(&directory)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp-"))
            .count();
        assert_eq!(leftovers, 0);
        let _ = fs::remove_dir_all(directory);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn dpapi_round_trip_is_not_plaintext() {
        let plaintext = b"aura-secret-test-value";
        let ciphertext = protect_for_current_user(plaintext).expect("DPAPI encryption should work");

        assert_ne!(ciphertext, plaintext);
        assert!(!ciphertext.windows(plaintext.len()).any(|window| window == plaintext));
        assert_eq!(
            unprotect_for_current_user(&ciphertext).expect("DPAPI decryption should work"),
            plaintext
        );
    }
}
