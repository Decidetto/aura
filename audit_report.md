# Aura Smart Voice Typing Application: Software Audit Report

**Date**: July 15, 2026  
**Auditor**: worker_m5  
**Audit Scope**: Build Validation, Security Design Review, Code Quality & Concurrency Analysis

---

## Executive Summary

This audit report compiles and synthesizes critical findings from the validation checks, security design review, and code quality/concurrency audit of the Aura smart voice typing application (v1.0.7). The audit evaluates the system's Rust-based backend (`src-tauri`), JavaScript frontend, and Tauri application configurations.

### Key Findings
1. **Compilation and Tests**: The Rust backend compiles successfully, and all 23 unit tests pass. However, the frontend static test suite fails due to missing privacy and data flow disclosures in the website documents (`index.html` and `index_ru.html`).
2. **Security Gaps**: 
   - Sensitive credentials and transcript history are stored in **plaintext JSON files** due to the insecure fallback modules being used in the active build.
   - The planned DPAPI secure storage features are uncompiled due to missing Windows API features in `Cargo.toml`. Furthermore, the DPAPI calls lack custom entropy, making them vulnerable to local decryption by other processes under the same user session.
   - A critical "key-wiping" logic bug exists where any general settings update from the UI clears existing API keys.
   - The Tauri capabilities configuration violates the principle of least privilege by granting browser-opening and app-update privileges to the recording status overlay window.
3. **Code Quality and Concurrency Issues**:
   - The anti-aliasing filter in `audio_recorder.rs` uses an incorrect cutoff calculation, causing frequencies between 8,000 Hz and 16,000 Hz to fold back (alias) into the decimated signal, which degrades transcription accuracy.
   - Audio processing lacks `NaN` float checks, exposing the app to process crashes when calling `f32::clamp`.
   - Trailing samples can be discarded at stream shutdown because of non-blocking channel checks.
   - The Parakeet daemon manager fails to recover or restart the backend server after a crash, and log path construction lacks space/Unicode escaping on Windows.
   - Stale asynchronous task responses can trigger clipboard race conditions that result in permanent user data loss.

---

## Section 1: Build & Compile Status

The build and compilation status of both the Rust backend and JavaScript frontend were audited. 

### 1. Backend Compilation (Rust / Tauri)
The backend uses Rust under the `src-tauri` directory. Running `cargo check` verifies compilation and dependency resolution.

- **Command Executed**: `cargo check` in `g:\1\2.0\src-tauri`
- **Results**: The backend compiles cleanly. No compile-time errors or warnings were reported.
  ```text
  Compiling aura-app v1.0.7 (G:\1\2.0\src-tauri)
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.66s
  ```

### 2. Frontend Compilation & Static Verification
The frontend is built with HTML/JS/CSS assets and is tested using a Node.js-based static test suite.

- **Command Executed**: `node tests/frontend-static.test.mjs` in `g:\1\2.0`
- **Status**: **Failed**
- **Error Summary**: The test suite fails during the execution of the test named `"website describes provider data flow and unsigned installer honestly"`.
- **Diagnostics**:
  ```text
  at TestContext.<anonymous> (file:///g:/1/2.0/tests/frontend-static.test.mjs:127:10)
  at Test.runInAsyncScope (node:async_hooks:227:14)
  ...
  code: 'ERR_ASSERTION',
  actual: '<!DOCTYPE html>\r\n<html lang="en">\r\n<head>\r\n  <meta charset="UTF-8" />\r\n  <meta name="viewport" content="width=device-width, initial-scale=1.0" />\r\n  \r\n  <!-- SEO Meta Tags -->\r\n  <title>Aura: Smart Voice Typing for Windows | Free and Unlimited</title>\r\n  <meta name="description" content="Aura: a free and open-source voice typing software for Windows. Works via cloud AI (Gemini, OpenAI, Groq) or fully locally and privately via Whisper." />\r\n...',
  expected: /cloud mode[^.]*audio[^.]*transcript[^.]*selected text[^.]*dictionary/is,
  operator: 'match',
  diff: 'simple'
  ```

---

## Section 2: Test Suite Execution & Diagnostics

### 1. Rust Unit Test Results
Running `cargo test` inside `src-tauri` executes the unit and doc tests. All tests execute successfully.

- **Command Executed**: `cargo test` in `g:\1\2.0\src-tauri`
- **Output**:
  ```text
  running 23 tests
  test ai_client::tests::test_language_normalization ... ok
  test keyboard_hook::windows_impl::tests::test_parse_hotkey_invalid ... ok
  test ai_client::tests::test_clean_instructions_conditional_sections ... ok
  test keyboard_hook::windows_impl::tests::test_parse_hotkey_combinations ... ok
  test ai_client::tests::test_openai_chat_deserialization ... ok
  test ai_client::tests::test_gemini_response_deserialization ... ok
  test ai_client::tests::test_gemini_request_serialization ... ok
  test settings::tests::test_legacy_settings_deserialization ... ok
  test settings::tests::test_save_settings_sync_logic ... ok
  test tests::test_is_cloud_unreachable ... ok
  test tests::test_clean_hallucinated_brackets ... ok
  test tests::test_silence_hallucination_keeps_real_dictation ... ok
  test tests::test_voice_punctuation_no_commands ... ok
  test tests::test_voice_punctuation_basic ... ok
  test settings::tests::test_settings_defaults ... ok
  test tests::test_is_newer_version ... ok
  test ai_client::tests::test_selected_text_is_delimited_as_data ... ok
  test tests::test_silence_hallucination_exact_phrases ... ok
  test tests::test_voice_punctuation_multiword_priority ... ok
  test whisper_runner::tests::test_filename_parsing ... ok
  test audio_recorder::tests::test_process_and_write_wav ... ok
  test vad::tests::test_trim_silence_no_speech ... ok
  test vad::tests::test_has_speech_silence ... ok

  test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
  ```

### 2. Frontend Static Test Failures & Root Cause
The static test failure in `tests/frontend-static.test.mjs` is due to a mismatch between the privacy disclosures expected by the tests and the text actually provided in the website files.

- **Failure Line**: `tests/frontend-static.test.mjs:127`
- **Assertion**:
  ```javascript
  assert.match(en, /cloud mode[^.]*audio[^.]*transcript[^.]*selected text[^.]*dictionary/is);
  ```
- **Root Cause**:
  - The regular expression `/cloud mode[^.]*audio[^.]*transcript[^.]*selected text[^.]*dictionary/is` expects the words `cloud mode`, `audio`, `transcript`, `selected text`, and `dictionary` to appear sequentially in `website/index.html` within a single sentence or block (without periods `.` separating them).
  - The website text at `website/index.html` only provides:
    ```html
    In cloud mode, audio is only sent directly to your chosen provider (Gemini, OpenAI, or Groq) over secure connections.
    ```
    This is missing disclosures indicating that the transcript, selected text, and custom dictionary are also transmitted to the cloud provider.
  - The corresponding Russian language test case for `website/index_ru.html` checks:
    ```javascript
    /облачн[^.]*аудио[^.]*транскрипт[^.]*выделенн[^.]*словар/is
    ```
    This test fails for the same reason.

---

## Section 3: Security Design Review

An audit of the application security patterns, credentials storage, and Tauri window capabilities revealed several design and implementation vulnerabilities.

### 1. Plaintext Storage & DPAPI Integration Status
- **Current Status**: The app is not currently using the secure storage implementation. In `src-tauri/src/lib.rs` (lines 7–10), the module imports are:
  ```rust
  pub mod settings;
  pub mod keyboard_simulator;
  pub mod history;
  ```
  Instead of utilizing `settings_secure.rs` and `history_secure.rs`, the application uses the insecure `settings.rs` and `history.rs` modules.
- **Insecure File Writes**:
  - In `settings.rs` (lines 139–143):
    ```rust
    let content = serde_json::to_string_pretty(&settings_clone)
        .map_err(|e| format!("Failed to format settings JSON: {}", e))?;
    fs::write(&path, content)
    ```
  - In `history.rs` (lines 45–48):
    ```rust
    let content = serde_json::to_string_pretty(&history_clone)
        .map_err(|e| format!("Failed to format history JSON: {}", e))?;
    fs::write(&path, content)
    ```
  These settings contain cloud API keys, and history files contain transcribed speech. They are saved on disk as plaintext JSON in `%APPDATA%` and `%LOCALAPPDATA%`, leaving them accessible to any process.

- **Cargo Dependency Configuration Issue**:
  The secure storage backend (`secure_storage.rs`) cannot compile because `Cargo.toml` lacks the required Win32 cryptography features for the `windows-sys` crate.
  - Location: `src-tauri/Cargo.toml` (lines 39–41)
  - Current configuration:
    ```toml
    [target.'cfg(target_os = "windows")'.dependencies]
    windows-sys = { version = "0.52", features = ["Win32_UI_WindowsAndMessaging", "Win32_Foundation", "Win32_System_LibraryLoader", "Win32_UI_Input_KeyboardAndMouse", "Win32_Storage_FileSystem"] }
    ```
  - **Vulnerability**: It is missing the `"Win32_Security_Cryptography"` feature flag required to link `CryptProtectData` and `CryptUnprotectData`.

### 2. DPAPI Entropy Vulnerability
If secure storage is enabled, it uses DPAPI to encrypt keys. However, the implementation does not supply custom entropy.
- **Location**: `src-tauri/src/secure_storage.rs` (lines 96–106 and 132–142)
- **Encryption Call**:
  ```rust
  let ok = unsafe {
      CryptProtectData(
          &input,
          std::ptr::null(), // Description
          std::ptr::null(), // pOptionalEntropy
          std::ptr::null(), // Reserved
          std::ptr::null(), // pPromptStruct
          CRYPTPROTECT_UI_FORBIDDEN,
          &mut output,
      )
  };
  ```
- **Decryption Call**:
  ```rust
  let ok = unsafe {
      CryptUnprotectData(
          &input,
          std::ptr::null_mut(), // pConnectionDescription
          std::ptr::null(),     // pOptionalEntropy
          std::ptr::null(),     // Reserved
          std::ptr::null(),     // pPromptStruct
          CRYPTPROTECT_UI_FORBIDDEN,
          &mut output,
      )
  };
  ```
- **Vulnerability**: Setting `pOptionalEntropy` to `null` means DPAPI relies solely on the user's OS login credentials. Consequently, any other application running under the same user session can invoke `CryptUnprotectData` on the saved ciphertext and steal the keys.

### 3. Key Wiping Logic Bug & Unregistered Commands
- **Serialization Behavior**:
  In `src-tauri/src/settings_secure.rs` (lines 32–39), the API keys are annotated with `#[serde(default, skip_serializing)]`:
  ```rust
  #[serde(default, skip_serializing)]
  pub api_key: String,
  #[serde(default, skip_serializing)]
  pub api_key_gemini: String,
  #[serde(default, skip_serializing)]
  pub api_key_openai: String,
  #[serde(default, skip_serializing)]
  pub api_key_groq: String,
  ```
  This is intended to prevent keys from leaking to the frontend during `get_settings`.
- **UI State Update Bug**:
  In the frontend JavaScript (`src/main.js`, lines 1523–1529), settings updates are sent in bulk via the `set_settings` command:
  ```javascript
  const settings = {
    transcription_mode: radioLocal.checked ? "local" : "cloud",
    api_provider: selectProvider.value,
    api_key: apiKeyInput.value.trim(),
    api_key_gemini: apiKeys.gemini.trim(),
    api_key_openai: apiKeys.openai.trim(),
    api_key_groq: apiKeys.groq.trim(),
  ```
  Since `get_settings` returned empty strings for keys, the cache in `apiKeys` is also empty. When the user saves any setting (such as changing the UI language), the frontend transmits empty strings for all API key fields. The backend deserializes these empty strings and writes them to the secure storage, wiping all API keys.
- **Unregistered Command Handler**:
  To address this, a dedicated command `set_provider_key` was written to update keys individually, and it was permitted in the capabilities configuration:
  - `src-tauri/capabilities/main.json` (line 13): `"allow-set-provider-key"`
  - **Bug**: The command was never registered in `src-tauri/src/lib.rs` (lines 1451–1468) under `tauri::generate_handler![]`, rendering it unreachable dead code.

### 4. Memory Sanitization & Secure Deletion Issues
- **Memory Leakage**:
  In `src-tauri/src/secure_storage.rs` (lines 110–113 & 146–149), plaintext decrypted secrets are copied directly to the heap:
  ```rust
  let plaintext = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
  unsafe {
      LocalFree(output.pbData.cast());
  }
  ```
  Standard Rust `Vec<u8>` and `String` allocations do not clear their memory blocks when they are dropped. Additionally, `LocalFree` is called without zeroing out the buffer allocated by DPAPI. Plaintext keys and transcriptions can thus linger in RAM, vulnerable to local memory scraping.
- **Insecure File Migration**:
  When migrating from legacy plaintext storage, `history_secure.rs` (line 87) calls:
  ```rust
  fs::remove_file(legacy_path)
  ```
  `fs::remove_file` only deletes the directory index pointer. The plaintext data remains on disk blocks and can be recovered by file reconstruction tools.

### 5. Tauri Capabilities Privilege Violations
- **Configuration**:
  In `src-tauri/capabilities/default.json` (lines 5–13), both the `"main"` window and `"overlay"` window are granted default permissions:
  ```json
  "windows": [
    "main",
    "overlay"
  ],
  "permissions": [
    "core:default",
    "opener:default",
    "updater:default"
  ]
  ```
- **Vulnerability**:
  The `overlay` window is a HUD showing volume and speech status. By default, it inherits `opener:default` (allowing it to open arbitrary URLs) and `updater:default` (allowing it to trigger software updates). This violates the principle of least privilege. If the overlay webview is compromised, the attacker can access these sensitive capabilities.

---

## Section 4: Code Quality & Bug Detection

An audit of the audio recording pipeline, daemon control threads, and platform clipboard handlers highlighted several functional and concurrency issues.

### 1. Cutoff Frequency Calculation in Anti-Aliasing Filter
- **Location**: `src-tauri/src/audio_recorder.rs` (lines 195–225)
- **Math Defect**:
  ```rust
  let cutoff = output_rate as f64 / input_rate as f64; // e.g. 16000 / 48000 = 1/3
  ...
  let sinc = if n.abs() < 1e-9 {
      2.0 * cutoff
  } else {
      (2.0 * PI * cutoff * n).sin() / (PI * n)
  };
  ```
  The equation used inside the sinc filter is:
  $$\frac{\sin(2\pi \cdot \text{cutoff} \cdot n)}{\pi \cdot n}$$
  In signal processing, the digital radian frequency cutoff is $\omega_c = 2\pi \cdot \text{cutoff}$.
  Substituting $\text{cutoff} = \frac{\text{output\_rate}}{\text{input\_rate}}$ yields:
  $$\omega_c = 2\pi \frac{\text{output\_rate}}{\text{input\_rate}}$$
  Since $\omega_c = \frac{2\pi \cdot f_c}{\text{input\_rate}}$, solving for the physical cutoff frequency $f_c$ gives:
  $$f_c = \text{output\_rate} \quad (16,000 \text{ Hz})$$
  To prevent aliasing, the cutoff frequency must be the Nyquist limit of the output sample rate:
  $$f_c = \frac{\text{output\_rate}}{2} \quad (8,000 \text{ Hz})$$
- **Impact**: Frequencies between 8,000 Hz and 16,000 Hz are not filtered out. When the signal is downsampled (decimated) to 16,000 Hz, these high-frequency sounds fold back (alias) into the audible 0–8,000 Hz band, creating acoustic artifacts and degrading transcription accuracy.

### 2. Panics on NaN Floats with `f32::clamp`
- **Location**: `src-tauri/src/audio_recorder.rs` (line 298) and `src-tauri/src/vad.rs` (line 104)
- **Code**:
  ```rust
  let clamped = sample.clamp(-1.0, 1.0);
  ```
- **Bug**: Rust's standard library `f32::clamp` panics if the input value is `NaN`.
- **Impact**: Hardware glitches, driver failures, or sound device disconnects can produce `NaN` values in the audio buffer. Passing a `NaN` sample will panic, crashing the entire Rust backend.

### 3. Trailing Audio Sample Loss
- **Location**: `src-tauri/src/audio_recorder.rs` (lines 150–154)
- **Code**:
  ```rust
  drop(active_recording.stream);
  while let Ok(chunk) = active_recording.receiver.try_recv() {
      active_recording.accumulated.extend(chunk);
  }
  ```
- **Bug**: `try_recv()` is non-blocking. If the audio stream thread is in the process of dropping but has not finished sending its final buffers, `try_recv()` returns `Err(TryRecvError::Empty)` immediately and exits the loop.
- **Impact**: The tail end of the audio recording is discarded, clipping the last word or syllable of the user's speech.

### 4. Parakeet Daemon Restart Failure
- **Location**: `src-tauri/src/whisper_runner.rs` (lines 726–730)
- **Code**:
  ```rust
  let mut server_guard = state.parakeet_server.lock().unwrap();
  if server_guard.is_some() {
      return Ok(());
  }
  ```
- **Bug**: If the Parakeet background server crashes, the child process exits, but its `Child` handle remains inside `parakeet_server` (it is still `Some(Child)`). Since `server_guard.is_some()` evaluates to `true`, the application assumes the server is running and will never attempt to restart it.
- **Impact**: The local transcription system remains dead after a single server crash until the entire application is restarted.

### 5. Log Path Spaces / Unicode Handling on Windows
- **Location**: `src-tauri/src/whisper_runner.rs` (lines 781, 818)
- **Code**:
  ```rust
  let log_file_path = app_local_data.join("parakeet_server_log.txt");
  ...
  format!("--log-file={}", log_file_path.to_string_lossy())
  ```
- **Bug**: Unlike the model path and the audio wave path, the log file path is not converted using `get_short_path` (which generates 8.3 short names on Windows).
- **Impact**: If the Windows username contains spaces or Unicode characters, the path will contain spaces. The CLI arguments are passed unescaped, causing the Parakeet server startup command to fail.

### 6. macOS Orphaned Sidecar Processes
- **Location**: `src-tauri/src/whisper_runner.rs` (lines 842–851)
- **Bug**: When stopping the server or on normal exit, the application calls `child.kill()`. However, if the application crashes or exits abruptly, the Parakeet child process continues running. On Windows, startup logic includes a `taskkill` routine (lines 734–741) to clean up orphaned daemons. No equivalent cleanup logic exists for macOS.
- **Impact**: Background server processes remain running indefinitely on macOS after crashes, leaking system resources.

### 7. Punctuation Sidecar Unicode Encoding
- **Location**: `src-tauri/src/whisper_runner.rs` (lines 1191–1195)
- **Code**:
  ```rust
  let mut cmd = Command::new(&short_punc_exe);
  cmd.args(&[
      format!("--ct-transformer={}", short_model_path.to_string_lossy()),
      text.to_string(),
  ]);
  ```
- **Bug**: The entire transcribed text is passed directly as a command-line argument to the sidecar. On Windows, the process command-line arguments are parsed under the active code page. If the text contains Unicode character sets (e.g. Cyrillic, CJK) that mismatch the system code page, they will be corrupted (replaced by `?`).
- **Impact**: Punctuated outputs for non-English transcription will be corrupted on Windows.

### 8. Clipboard Race Conditions & Data Loss
- **Location**: `src-tauri/src/lib.rs` (lines 1243–1257) & (lines 531–545)
- **Race Condition**:
  1. An asynchronous transcription task from a previous dictation finishes late.
  2. The check for `session_ok` evaluates to `false` because the session counter has incremented.
  3. The task writes its text to the clipboard: `cb.set_text(final_text.clone())`.
  4. Concurrently, the user initiates a new session (Session 2). That session performs a clipboard backup using `backup_clipboard()`.
  5. The backup routine captures the late text from Session 1 instead of the user's actual clipboard.
  6. When Session 2 completes, the backup (Session 1's text) is restored, permanently deleting the user's original clipboard content.
- **Incomplete Format Backup**:
  The `backup_clipboard` routine only supports text and images. Any other clipboard contents (such as files, rich text format, HTML) are discarded, and an empty clipboard state is restored.

### 9. High Overhead for VAD Rebuilding
- **Location**: `src-tauri/src/vad.rs` (lines 5–15, 33–43)
- **Bug**: The `VoiceActivityDetector` struct is rebuilt using `.build()` on every single invocation of the `has_speech` and `trim_silence` functions.
- **Impact**: Rebuilding the VAD object requires allocating memory and parsing configuration states repeatedly. Doing this inside an audio loop introduces substantial CPU overhead and latency.

---

## Section 5: Recommendations and Remediation Plan

### 1. Fix Security Design & Integration Gaps
- **Link Cryptography API**:
  Modify `src-tauri/Cargo.toml` to add `"Win32_Security_Cryptography"` to the `windows-sys` features:
  ```toml
  windows-sys = { version = "0.52", features = [
      "Win32_UI_WindowsAndMessaging",
      "Win32_Foundation",
      "Win32_System_LibraryLoader",
      "Win32_UI_Input_KeyboardAndMouse",
      "Win32_Storage_FileSystem",
      "Win32_Security_Cryptography"
  ] }
  ```
- **Integrate Secure Modules**:
  Update `src-tauri/src/lib.rs` to import `settings_secure` and `history_secure`:
  ```rust
  pub mod settings_secure;
  pub mod keyboard_simulator;
  pub mod history_secure;
  ```
  Ensure all references in `lib.rs` are updated to call the secure modules.
- **Implement DPAPI Entropy**:
  Define a static application-specific entropy key in `secure_storage.rs` and pass it to DPAPI:
  ```rust
  const ENTROPY: &[u8] = b"AuraSmartVoiceTypingSecureEntropy107!";
  let mut entropy_blob = CRYPT_INTEGER_BLOB {
      cbData: ENTROPY.len() as u32,
      pbData: ENTROPY.as_ptr() as *mut u8,
  };
  ```
- **Register `set_provider_key`**:
  Register the command in `src-tauri/src/lib.rs` inside the invocation handler:
  ```rust
  tauri::generate_handler![
      get_settings,
      set_settings,
      set_provider_key, // Add here
      ...
  ]
  ```
- **Fix Key Wiping in Frontend**:
  Refactor the frontend save sequence (`src/main.js`) to invoke `set_provider_key` immediately when an API key is entered, and remove API keys from the serialized `Settings` payload sent during `set_settings`.
- **Memory Scrubbing**:
  Use the `zeroize` crate to zero out sensitive `String` buffers when they are dropped. Manually clear the DPAPI buffer before calling `LocalFree` in `secure_storage.rs`:
  ```rust
  if !output.pbData.is_null() {
      std::ptr::write_bytes(output.pbData, 0, output.cbData as usize);
  }
  LocalFree(output.pbData.cast());
  ```
- **Secure File Erasure**:
  In `migrate_legacy_if_needed`, write zeroes to the old `settings.json` and `history.json` files before removing them with `fs::remove_file`.
- **Tauri Least-Privilege Capabilities**:
  Create a new capability profile at `src-tauri/capabilities/overlay.json` dedicated exclusively to the overlay window, removing permissions for `opener` and `updater`. Update `capabilities/default.json` to target only the `"main"` window.

### 2. Fix Audio Processing and Concurrency Bugs
- **Anti-Aliasing Filter Cutoff**:
  Change the cutoff value in `audio_recorder.rs` to divide by $2.0$:
  ```rust
  let cutoff = output_rate as f64 / (2.0 * input_rate as f64); // e.g. 16000 / (2 * 48000) = 1/6
  ```
  This shifts the digital cutoff to $\omega_c = \frac{\pi}{3}$, matching the target Nyquist frequency of 8,000 Hz.
- **NaN Handling**:
  Replace `f32::clamp` with a safe, non-panicking implementation that checks for `NaN`:
  ```rust
  fn safe_clamp(val: f32, min: f32, max: f32) -> f32 {
      if val.is_nan() {
          0.0
      } else if val < min {
          min
      } else if val > max {
          max
      } else {
          val
      }
  }
  ```
- **Drain Audio Channel**:
  When shutting down the recorder, use blocking reads (`recv()`) instead of `try_recv()`, and block until the channel is closed to ensure all trailing samples are written:
  ```rust
  drop(active_recording.stream);
  while let Ok(chunk) = active_recording.receiver.recv() {
      active_recording.accumulated.extend(chunk);
  }
  ```
- **Parakeet Server Crash Detection**:
  Use `Child::try_wait` to check if the process has exited:
  ```rust
  let mut server_guard = state.parakeet_server.lock().unwrap();
  if let Some(ref mut child) = *server_guard {
      if let Ok(Some(_status)) = child.try_wait() {
          *server_guard = None; // Process exited, clear the option
      }
  }
  if server_guard.is_none() {
      // Re-launch server
  }
  ```
- **Log Path Escaping**:
  Use the Windows 8.3 short path helper `get_short_path` to resolve the local data path before generating the `--log-file` argument.
- **macOS Daemon Cleanup**:
  Implement a cleanup routine on macOS startup that checks for and kills any orphaned processes associated with the Parakeet daemon.
- **Unicode CLI Arguments**:
  Instead of passing the transcribed text as a command-line argument to the punctuation sidecar, write the text to a temporary file and pass the path to the file, or write the text to the sidecar's standard input stream (`stdin`).
- **Clipboard Generation Validation**:
  In `lib.rs`, validate that the session generation ID matches before calling `set_text` to prevent late-running threads from writing to the clipboard:
  ```rust
  if session_ok.load(Ordering::SeqCst) {
      cb.set_text(final_text.clone());
  }
  ```
- **VAD Struct Optimization**:
  Instantiate the `VoiceActivityDetector` once and cache it inside the recorder's state or local processing loop instead of rebuilding it on every frame check.
