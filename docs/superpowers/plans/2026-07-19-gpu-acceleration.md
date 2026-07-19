# GPU Hardware Acceleration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement dynamic, on-demand downloading and execution of GPU-accelerated (CUDA and DirectML) Sherpa-ONNX WebSocket server binaries in Aura.

**Architecture:** Extend backend Settings with `local_acceleration` enum ("cpu", "cuda", "directml"). Provide Tauri commands to download, extract (via native `tar.exe`), and delete precompiled GPU binary packages. Integrate GPU card layouts on the frontend settings page matching the existing model card style.

**Tech Stack:** Rust (Tauri, reqwest), JavaScript, HTML5/CSS3.

## Global Constraints
- Target platform: Windows x64.
- GPU dependencies must be extracted to `AppData/Local/com.aura.app/binaries/<provider>`.
- If the GPU binary or DLL is missing, automatically fall back to CPU sidecar binary.
- Use identical style classes `.status-ready-badge`, `.btn-download-card-model`, and `.btn-delete-card-model` for UI consistency.

---

### Task 1: Extend Settings Model in Rust and JS

**Files:**
- Modify: [settings.rs](file:///G:/Aura/2.0/src-tauri/src/settings.rs)
- Modify: [main.js](file:///G:/Aura/2.0/src/main.js)

**Interfaces:**
- Produces: `Settings.local_acceleration: String` ("cpu" | "cuda" | "directml")

- [ ] **Step 1: Add `local_acceleration` field to `Settings` struct**
  Add the field around line 32 in `settings.rs`:
  ```rust
  pub struct Settings {
      // ...
      pub local_engine: String,
      pub copy_context_on_start: bool,
      pub local_acceleration: String, // "cpu" | "cuda" | "directml"
  }
  ```

- [ ] **Step 2: Add default value in `impl Default for Settings`**
  Modify around line 58 in `settings.rs`:
  ```rust
  impl Default for Settings {
      fn default() -> Self {
          Self {
              // ...
              local_engine: "whisper".to_string(),
              copy_context_on_start: true,
              local_acceleration: "cpu".to_string(),
          }
      }
  }
  ```

- [ ] **Step 3: Update `test_settings_defaults` unit test**
  Add assertion around line 165 in `settings.rs`:
  ```rust
  #[test]
  fn test_settings_defaults() {
      let settings = Settings::default();
      // ...
      assert_eq!(settings.local_engine, "whisper");
      assert_eq!(settings.local_acceleration, "cpu");
  }
  ```

- [ ] **Step 4: Run unit tests to verify**
  Run: `cargo test` in `src-tauri`
  Expected: PASS

- [ ] **Step 5: Add default value in JS settings (`src/main.js`)**
  Update default configuration object and localization strings at the top of `src/main.js`:
  ```javascript
  // Around line 1515 in saveSettings():
  const settings = {
      transcription_mode: radioLocal.checked ? "local" : "cloud",
      local_engine: selectLocalEngine.value,
      local_acceleration: activeLocalAcceleration, // CPU/CUDA/DirectML
      // ...
  };
  ```

- [ ] **Step 6: Commit**
  ```bash
  git add src-tauri/src/settings.rs src/main.js
  git commit -m "feat: extend settings with local_acceleration field"
  ```

---

### Task 2: Implement Tauri Commands for GPU Download & Deletion

**Files:**
- Modify: [lib.rs](file:///G:/Aura/2.0/src-tauri/src/lib.rs)

**Interfaces:**
- Produces:
  - `#[tauri::command] async fn download_gpu_binaries(app_handle: tauri::AppHandle, provider: String) -> Result<(), String>`
  - `#[tauri::command] async fn delete_gpu_binaries(app_handle: tauri::AppHandle, provider: String) -> Result<(), String>`
  - `#[tauri::command] async fn check_gpu_downloaded(app_handle: tauri::AppHandle, provider: String) -> Result<bool, String>`

- [ ] **Step 1: Add new Tauri commands in `src-tauri/src/lib.rs`**
  Insert the commands at the bottom of the file (before `#[cfg(test)]`):
  ```rust
  #[derive(Clone, serde::Serialize)]
  struct GpuDownloadProgress {
      provider: String,
      downloaded: u64,
      total: Option<u64>,
      percentage: f64,
      done: bool,
  }

  #[tauri::command]
  async fn check_gpu_downloaded(app_handle: tauri::AppHandle, provider: String) -> Result<bool, String> {
      let app_local_data = app_handle
          .path()
          .app_local_data_dir()
          .map_err(|e| format!("Failed to get app local data: {}", e))?;
      let exe_name = if cfg!(target_os = "windows") {
          "sherpa-onnx-offline-websocket-server.exe"
      } else {
          "sherpa-onnx-offline-websocket-server"
      };
      let exe_path = app_local_data
          .join("binaries")
          .join(&provider)
          .join("bin")
          .join(exe_name);
      Ok(exe_path.exists())
  }

  #[tauri::command]
  async fn delete_gpu_binaries(app_handle: tauri::AppHandle, provider: String) -> Result<(), String> {
      let app_local_data = app_handle
          .path()
          .app_local_data_dir()
          .map_err(|e| format!("Failed to get app local data: {}", e))?;
      let provider_dir = app_local_data.join("binaries").join(&provider);
      if provider_dir.exists() {
          std::fs::remove_dir_all(&provider_dir)
              .map_err(|e| format!("Failed to delete binaries: {}", e))?;
      }
      Ok(())
  }

  #[tauri::command]
  async fn download_gpu_binaries(app_handle: tauri::AppHandle, provider: String) -> Result<(), String> {
      let app_local_data = app_handle
          .path()
          .app_local_data_dir()
          .map_err(|e| format!("Failed to get app local data: {}", e))?;
      
      let gpu_dir = app_local_data.join("binaries").join(&provider);
      std::fs::create_dir_all(&gpu_dir)
          .map_err(|e| format!("Failed to create folder: {}", e))?;

      let url = match provider.as_str() {
          "cuda" => "https://github.com/malashkadev/Aura/releases/download/v1.0.8-assets/sherpa-onnx-v1.13.4-win-x64-cuda.tar.bz2",
          "directml" => "https://github.com/malashkadev/Aura/releases/download/v1.0.8-assets/sherpa-onnx-v1.13.4-win-x64-directml.tar.bz2",
          _ => return Err("Invalid provider".to_string()),
      };

      let temp_tar_path = gpu_dir.join(format!("temp_{}.tar.bz2", provider));
      if temp_tar_path.exists() {
          let _ = std::fs::remove_file(&temp_tar_path);
      }

      eprintln!("Aura Dev Log: Starting download for GPU provider {}...", provider);
      let client = crate::ai_client::build_download_client();
      let mut response = client.get(url).send().await
          .map_err(|e| format!("Failed to fetch URL: {}", e))?;

      let total_size = response.content_length().unwrap_or(221_905_418);
      let mut total_downloaded = 0u64;

      {
          use tokio::io::AsyncWriteExt;
          let mut file = tokio::fs::File::create(&temp_tar_path).await
              .map_err(|e| format!("Failed to create temp file: {}", e))?;

          while let Some(chunk) = response.chunk().await
              .map_err(|e| format!("Error reading chunk: {}", e))?
          {
              file.write_all(&chunk).await
                  .map_err(|e| format!("Failed to write: {}", e))?;
              total_downloaded += chunk.len() as u64;

              let percentage = (total_downloaded as f64 / total_size as f64 * 100.0).min(99.9);
              let _ = app_handle.emit(
                  "gpu-download-progress",
                  GpuDownloadProgress {
                      provider: provider.clone(),
                      downloaded: total_downloaded,
                      total: Some(total_size),
                      percentage,
                      done: false,
                  },
              );
          }
          file.flush().await.map_err(|e| format!("Failed to flush: {}", e))?;
      }

      // Extraction using native tar.exe
      eprintln!("Aura Dev Log: Extracting package...");
      use std::process::Command;
      let mut cmd = Command::new("tar");
      cmd.args(&[
          "-xf",
          temp_tar_path.to_str().unwrap_or_default(),
          "-C",
          gpu_dir.to_str().unwrap_or_default(),
      ]);
      #[cfg(target_os = "windows")]
      {
          use std::os::windows::process::CommandExt;
          cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
      }
      
      let status = cmd.status().map_err(|e| format!("Failed to run tar: {}", e))?;
      if !status.success() {
          let _ = std::fs::remove_file(&temp_tar_path);
          return Err("Extraction failed".to_string());
      }

      let _ = std::fs::remove_file(&temp_tar_path);

      // Structure alignment: move binaries out of nested tar folder if necessary
      let nested_dir = gpu_dir.join(format!("sherpa-onnx-v1.13.4-win-x64-{}", provider));
      if nested_dir.exists() {
          let _ = std::fs::rename(nested_dir.join("bin"), gpu_dir.join("bin"));
          let _ = std::fs::remove_dir_all(&nested_dir);
      }

      let _ = app_handle.emit(
          "gpu-download-progress",
          GpuDownloadProgress {
              provider: provider.clone(),
              downloaded: total_size,
              total: Some(total_size),
              percentage: 100.0,
              done: true,
          },
      );

      Ok(())
  }
  ```

- [ ] **Step 2: Register the commands in the Tauri builder**
  Add the new commands inside `tauri::Builder::default().invoke_handler(tauri::generate_handler![...])` call in `src-tauri/src/lib.rs`:
  ```rust
  // Around line 600:
  .invoke_handler(tauri::generate_handler![
      // ... existing commands ...
      download_gpu_binaries,
      delete_gpu_binaries,
      check_gpu_downloaded
  ])
  ```

- [ ] **Step 3: Verify build**
  Run: `cargo build` in `src-tauri`
  Expected: Successful compilation without errors.

- [ ] **Step 4: Commit**
  ```bash
  git add src-tauri/src/lib.rs
  git commit -m "feat: implement tauri commands for gpu download, delete, and check"
  ```

---

### Task 3: Dynamic Binary Lookup and Provider Selection

**Files:**
- Modify: [whisper_runner.rs](file:///G:/Aura/2.0/src-tauri/src/whisper_runner.rs)

**Interfaces:**
- Consumes: `Settings.local_acceleration`
- Produces: dynamically resolved paths for sidecar executables.

- [ ] **Step 1: Update `find_sherpa_websocket_server` in `whisper_runner.rs`**
  Modify `find_sherpa_websocket_server` (lines 661–715) to check `local_acceleration`:
  ```rust
  pub fn find_sherpa_websocket_server<R: Runtime>(app_handle: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
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
                  return Ok(gpu_exe);
              }
          }
      }

      // Rest of existing candidate checks ...
  ```

- [ ] **Step 2: Add dynamic lookup function for punctuation sidecar**
  Create `find_sherpa_punctuation_exe` in `whisper_runner.rs`:
  ```rust
  pub fn find_sherpa_punctuation_exe<R: Runtime>(app_handle: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
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
                  return Ok(gpu_exe);
              }
          }
      }

      // Default candidate check:
      let resource_dir = app_handle.path().resource_dir()
          .map_err(|e| format!("Failed to get resource dir: {}", e))?;
      let candidates = vec![
          resource_dir.join("binaries").join(exe_name),
          resource_dir.join(exe_name),
          PathBuf::from("binaries").join(exe_name),
      ];
      if let Some(path) = candidates.into_iter().find(|p| p.exists()) {
          return Ok(path);
      }
      Err("Punctuation sidecar executable not found".to_string())
  }
  ```

- [ ] **Step 3: Inject `--provider` argument in `start_parakeet_server`**
  Modify `start_parakeet_server` around line 810 to include provider settings:
  ```rust
        let mut args = vec![
            format!("--encoder={}", short_encoder.to_string_lossy()),
            format!("--decoder={}", short_decoder.to_string_lossy()),
            format!("--joiner={}", short_joiner.to_string_lossy()),
            format!("--tokens={}", short_tokens.to_string_lossy()),
            format!("--port={}", port),
            "--feat-dim=128".to_string(),
            format!("--num-work-threads={}", n_threads),
            format!("--log-file={}", log_file_path.to_string_lossy()),
        ];

        if settings.local_acceleration == "cuda" {
            args.push("--provider=cuda".to_string());
        } else if settings.local_acceleration == "directml" {
            args.push("--provider=directml".to_string());
        } else {
            args.push("--provider=cpu".to_string());
        }
  ```

- [ ] **Step 4: Update `run_punctuation` to use `find_sherpa_punctuation_exe`**
  Replace line 1187 `let punc_exe = find_sherpa_punctuation_exe(app_handle)?;` inside `run_punctuation` to call the newly created helper.

- [ ] **Step 5: Verify build**
  Run: `cargo build` in `src-tauri`
  Expected: Success.

- [ ] **Step 6: Commit**
  ```bash
  git add src-tauri/src/whisper_runner.rs
  git commit -m "feat: dynamically resolve sidecars and inject provider argument"
  ```

---

### Task 4: UI Rendering and Actions Binding

**Files:**
- Modify: [index.html](file:///G:/Aura/2.0/src/index.html)
- Modify: [main.js](file:///G:/Aura/2.0/src/main.js)

- [ ] **Step 1: Add GPU acceleration selection markup in `index.html`**
  Insert the cards group markup right below the `select-local-engine` form field group (around line 140):
  ```html
  <div class="settings-form-group" id="gpu-acceleration-settings" style="display: none;">
      <label class="field-label" data-i18n="gpu_accel_label">Локальное аппаратное ускорение</label>
      <div class="model-cards-group" role="radiogroup" aria-label="Выберите режим GPU">
          <!-- CPU Card -->
          <div class="model-card" data-gpu="cpu" role="radio" aria-checked="true" tabindex="0">
              <div class="model-card-main">
                  <div class="model-card-info">
                      <div class="model-card-title" data-i18n="gpu_accel_cpu_title">CPU (Без ускорения)</div>
                      <div class="model-card-desc" data-i18n="gpu_accel_cpu_desc">Стандартный режим. Безопасен, но нагружает процессор.</div>
                  </div>
                  <div class="model-card-action" id="action-gpu-cpu">
                      <span class="status-ready-badge">
                          <svg class="status-ready-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
                          <span data-i18n="model_status_ready">Установлено</span>
                      </span>
                  </div>
              </div>
          </div>
          <!-- CUDA Card -->
          <div class="model-card" data-gpu="cuda" role="radio" aria-checked="false" tabindex="0">
              <div class="model-card-main">
                  <div class="model-card-info">
                      <div class="model-card-title" data-i18n="gpu_accel_cuda_title">NVIDIA CUDA (Макс. скорость)</div>
                      <div class="model-card-desc" data-i18n="gpu_accel_cuda_desc">Для видеокарт GeForce RTX/GTX. Использование тензорных ядер.</div>
                  </div>
                  <div class="model-card-action" id="action-gpu-cuda"></div>
              </div>
              <div class="model-card-progress" id="progress-gpu-cuda" style="display: none;">
                  <div class="progress-bar-bg"><div class="progress-bar-fill" id="fill-gpu-cuda"></div></div>
                  <span class="progress-percentage" id="percent-gpu-cuda">0%</span>
              </div>
          </div>
          <!-- DirectML Card -->
          <div class="model-card" data-gpu="directml" role="radio" aria-checked="false" tabindex="0">
              <div class="model-card-main">
                  <div class="model-card-info">
                      <div class="model-card-title" data-i18n="gpu_accel_dml_title">DirectML (Универсальный)</div>
                      <div class="model-card-desc" data-i18n="gpu_accel_dml_desc">Для видеокарт AMD, Intel и NVIDIA. Базовое ускорение.</div>
                  </div>
                  <div class="model-card-action" id="action-gpu-directml"></div>
              </div>
              <div class="model-card-progress" id="progress-gpu-directml" style="display: none;">
                  <div class="progress-bar-bg"><div class="progress-bar-fill" id="fill-gpu-directml"></div></div>
                  <span class="progress-percentage" id="percent-gpu-directml">0%</span>
              </div>
          </div>
      </div>
  </div>
  ```

- [ ] **Step 2: Initialize GPU selection states in `src/main.js`**
  Define a global state `let activeLocalAcceleration = "cpu";` at the top of the frontend logic. Add status lookup and click bindings:
  ```javascript
  // Handle click on GPU card selection
  document.querySelectorAll('[data-gpu]').forEach(card => {
      card.addEventListener('click', () => {
          const provider = card.getAttribute('data-gpu');
          // If the provider needs to be downloaded, prevent selection until installed
          checkGpuInstalled(provider).then(installed => {
              if (installed || provider === 'cpu') {
                  selectGpuProvider(provider);
              }
          });
      });
  });

  function selectGpuProvider(provider) {
      activeLocalAcceleration = provider;
      document.querySelectorAll('[data-gpu]').forEach(c => {
          const isSelected = c.getAttribute('data-gpu') === provider;
          c.setAttribute('aria-checked', isSelected ? 'true' : 'false');
          if (isSelected) {
              c.classList.add('selected');
          } else {
              c.classList.remove('selected');
          }
      });
      // Set settings dirty state
      markSettingsDirty();
  }
  ```

- [ ] **Step 3: Implement download, delete, and status check logic for GPU cards**
  Add helper functions to check and render state:
  ```javascript
  async function updateGpuCardStates() {
      const providers = ['cuda', 'directml'];
      for (const provider of providers) {
          const isDownloaded = await tauri.invoke('check_gpu_downloaded', { provider });
          const actionEl = document.getElementById(`action-gpu-${provider}`);
          const progressEl = document.getElementById(`progress-gpu-${provider}`);
          
          if (!actionEl) continue;
          progressEl.style.display = 'none';
          
          if (isDownloaded) {
              actionEl.innerHTML = `
                  <span class="status-ready-badge">
                      <svg class="status-ready-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
                      <span data-i18n="model_status_ready">${i18nDict[currentLanguage].model_status_ready}</span>
                  </span>
                  <button type="button" class="btn-delete-card-model" title="${i18nDict[currentLanguage].model_action_delete}" data-gpu-provider="${provider}">
                      <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path><line x1="10" y1="11" x2="10" y2="17"></line><line x1="14" y1="11" x2="14" y2="17"></line></svg>
                  </button>
              `;
              actionEl.querySelector('.btn-delete-card-model').addEventListener('click', (e) => {
                  e.stopPropagation();
                  deleteGpuBinaries(provider);
              });
          } else {
              actionEl.innerHTML = `
                  <button type="button" class="btn-download-card-model" data-gpu-provider="${provider}">
                      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="btn-icon"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>
                      ${i18nDict[currentLanguage].model_action_download}
                  </button>
              `;
              actionEl.querySelector('.btn-download-card-model').addEventListener('click', (e) => {
                  e.stopPropagation();
                  downloadGpuBinaries(provider);
              });
          }
      }
  }

  async function downloadGpuBinaries(provider) {
      const actionEl = document.getElementById(`action-gpu-${provider}`);
      const progressEl = document.getElementById(`progress-gpu-${provider}`);
      const fillEl = document.getElementById(`fill-gpu-${provider}`);
      const percentEl = document.getElementById(`percent-gpu-${provider}`);

      actionEl.style.display = 'none';
      progressEl.style.display = 'flex';
      fillEl.style.width = '0%';
      percentEl.innerText = '0%';

      try {
          await tauri.invoke('download_gpu_binaries', { provider });
          showToast(`GPU component ${provider} downloaded successfully!`);
      } catch (err) {
          showToast(`Error: ${err}`);
      } finally {
          actionEl.style.display = 'flex';
          updateGpuCardStates();
      }
  }

  async function deleteGpuBinaries(provider) {
      if (confirm(`Remove GPU components for ${provider}?`)) {
          try {
              await tauri.invoke('delete_gpu_binaries', { provider });
              showToast(`GPU components for ${provider} deleted.`);
              if (activeLocalAcceleration === provider) {
                  selectGpuProvider('cpu');
              }
          } catch (err) {
              showToast(`Error: ${err}`);
          } finally {
              updateGpuCardStates();
          }
      }
  }
  ```

- [ ] **Step 4: Bind download progress listeners**
  Add listener for `"gpu-download-progress"`:
  ```javascript
  tauri.listen('gpu-download-progress', event => {
      const progress = event.payload;
      const fillEl = document.getElementById(`fill-gpu-${progress.provider}`);
      const percentEl = document.getElementById(`percent-gpu-${progress.provider}`);
      if (fillEl && percentEl) {
          fillEl.style.width = `${progress.percentage.toFixed(1)}%`;
          percentEl.innerText = `${progress.percentage.toFixed(1)}%`;
      }
  });
  ```

- [ ] **Step 5: Run compiler check**
  Run: `cargo build` in `src-tauri`
  Expected: Success.

- [ ] **Step 6: Commit changes**
  ```bash
  git add src/index.html src/main.js
  git commit -m "feat: add GPU acceleration settings UI and download interaction"
  ```
