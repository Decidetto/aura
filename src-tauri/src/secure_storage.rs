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
        // Harden the new file before it becomes visible at the final path:
        // only the current user and SYSTEM may read the contents. The DACL
        // travels with the rename below.
        if let Err(error) = apply_restrictive_acl(&temp_path) {
            crate::logger::log(
                "WARN",
                "Security",
                None,
                &format!("Could not restrict ACL on '{}': {error}", temp_path.display()),
            );
        }
        replace_file(&temp_path, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

#[cfg(target_os = "windows")]
fn owned_sid_copy(sid: windows_sys::Win32::Foundation::PSID) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Security::{CopySid, GetLengthSid};
    unsafe {
        let len = GetLengthSid(sid);
        let mut owned = vec![0u8; len as usize];
        if CopySid(len, owned.as_mut_ptr() as windows_sys::Win32::Foundation::PSID, sid) == 0 {
            return Err(format!(
                "CopySid failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(owned)
    }
}

#[cfg(target_os = "windows")]
fn well_known_sid_bytes(kind: windows_sys::Win32::Security::WELL_KNOWN_SID_TYPE) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Security::CreateWellKnownSid;
    unsafe {
        let mut len: u32 = 0;
        let _ = CreateWellKnownSid(kind, std::ptr::null_mut(), std::ptr::null_mut(), &mut len);
        let mut sid = vec![0u8; len as usize];
        if CreateWellKnownSid(
            kind,
            std::ptr::null_mut(),
            sid.as_mut_ptr() as windows_sys::Win32::Foundation::PSID,
            &mut len,
        ) == 0
        {
            return Err(format!(
                "CreateWellKnownSid failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(sid)
    }
}

/// Replaces the DACL of a file with two explicit ACEs: the current user (the
/// token owner, not the username) and SYSTEM, with inheritance protection so
/// nothing else can leak through parent directories. The caller must keep the
/// current-user SID derived from the process token: services and admin-wrapped
/// contexts should not inherit the admin's identity.
#[cfg(target_os = "windows")]
pub fn apply_restrictive_acl(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{SetEntriesInAclW, SetNamedSecurityInfoW, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER, WinLocalSystemSid,
        DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let user_sid = unsafe {
        let mut token: windows_sys::Win32::Foundation::HANDLE = 0;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(format!(
                "OpenProcessToken failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let mut needed: u32 = 0;
        let _ = GetTokenInformation(
            token,
            TokenUser,
            std::ptr::null_mut(),
            0,
            &mut needed,
        );
        let mut buffer = vec![0u8; needed as usize];
        let ok = GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr() as *mut std::ffi::c_void,
            needed,
            &mut needed,
        );
        windows_sys::Win32::Foundation::CloseHandle(token);
        if ok == 0 {
            return Err(format!(
                "GetTokenInformation failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        std::ptr::read(buffer.as_ptr() as *const TOKEN_USER).User.Sid
    };
    let user_sid = owned_sid_copy(user_sid)?;
    let system_sid = well_known_sid_bytes(WinLocalSystemSid)?;

    let entries = [explicit_grant_ace(&user_sid), explicit_grant_ace(&system_sid)];
    let mut new_acl: *mut windows_sys::Win32::Security::ACL = std::ptr::null_mut();
    let acl_result = unsafe {
        SetEntriesInAclW(
            entries.len() as u32,
            entries.as_ptr(),
            std::ptr::null(),
            &mut new_acl,
        )
    };
    if acl_result != 0 {
        return Err(format!(
            "SetEntriesInAclW failed with error {acl_result}"
        ));
    }

    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let named_result = unsafe {
        SetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            new_acl,
            std::ptr::null_mut(),
        )
    };
    if !new_acl.is_null() {
        unsafe {
            LocalFree(new_acl.cast());
        }
    }
    if named_result != 0 {
        return Err(format!(
            "SetNamedSecurityInfoW failed with error {named_result}"
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn explicit_grant_ace(sid: &[u8]) -> windows_sys::Win32::Security::Authorization::EXPLICIT_ACCESS_W {
    use windows_sys::Win32::Security::Authorization::{EXPLICIT_ACCESS_W, GRANT_ACCESS, TRUSTEE_IS_SID, TRUSTEE_W};
    EXPLICIT_ACCESS_W {
        grfAccessPermissions: 0x1F_01FF, // FILE_ALL_ACCESS
        grfAccessMode: GRANT_ACCESS,
        grfInheritance: 0,
        Trustee: TRUSTEE_W {
            pMultipleTrustee: std::ptr::null_mut(),
            MultipleTrusteeOperation: 0,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: 0,
            ptstrName: sid.as_ptr() as *mut u16,
        },
    }
}

#[cfg(not(target_os = "windows"))]
pub fn apply_restrictive_acl(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source_wide: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
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
        return Err(format!(
            "DPAPI encryption failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    let encrypted =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
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
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
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
        return Err(format!(
            "DPAPI decryption failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    let plaintext =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        // CryptUnprotectData allocates this buffer with LocalAlloc. Scrub the
        // decrypted bytes before returning the allocation to the OS.
        std::ptr::write_bytes(output.pbData, 0, output.cbData as usize);
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
        assert!(!ciphertext
            .windows(plaintext.len())
            .any(|window| window == plaintext));
        assert_eq!(
            unprotect_for_current_user(&ciphertext).expect("DPAPI decryption should work"),
            plaintext
        );
    }

    #[cfg(target_os = "windows")]
    fn file_dacl_ace_count(path: &Path) -> usize {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
        use windows_sys::Win32::Security::{
            GetAclInformation, AclSizeInformation, ACL_SIZE_INFORMATION,
            DACL_SECURITY_INFORMATION,
        };
        unsafe {
            let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let mut dacl: *mut windows_sys::Win32::Security::ACL = std::ptr::null_mut();
            let mut descriptor: *mut std::ffi::c_void = std::ptr::null_mut();
            let result = GetNamedSecurityInfoW(
                path_wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut dacl,
                std::ptr::null_mut(),
                &mut descriptor,
            );
            if result != 0 || dacl.is_null() {
                return usize::MAX;
            }
            let mut size_info: ACL_SIZE_INFORMATION = std::mem::zeroed();
            let ok = GetAclInformation(
                dacl,
                &mut size_info as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            );
            if ok == 0 {
                return usize::MAX;
            }
            size_info.AceCount as usize
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn atomic_write_restricts_the_dacl_to_current_user_and_system() {
        let directory = test_dir("restrictive-acl");
        let path = directory.join("settings.json");
        atomic_write(&path, b"{\"key\":\"secret\"}").expect("write should succeed");
        assert_eq!(
            file_dacl_ace_count(&path),
            2,
            "the DACL must contain exactly the current-user and SYSTEM ACEs"
        );
        let _ = fs::remove_dir_all(directory);
    }
}
