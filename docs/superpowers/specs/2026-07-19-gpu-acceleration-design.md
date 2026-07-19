# Design Spec: Local GPU/Hardware Acceleration (CUDA + DirectML)

This specification details the architecture for integrating precompiled GPU-accelerated sidecar binaries for `sherpa-onnx` (Parakeet TDT v3 model) on Windows, supporting both NVIDIA CUDA (on-demand download) and cross-GPU DirectML (on-demand download) options.

---

## 1. Context and Goals

Aura 1.0.8 needs to offload local transcription (Sherpa-ONNX) from CPU to GPU. 
- **CUDA:** Highest performance for NVIDIA GPUs (native TensorRT/cuDNN kernels). Requires heavy dependencies (~220 MB compressed, ~800 MB uncompressed).
- **DirectML:** Universal acceleration for AMD, Intel, and NVIDIA GPUs. Requires smaller dependencies (~40 MB compressed).
- **CPU:** Standard fallback if GPU components are not downloaded or unavailable.

To keep the main installer small (~25 MB), the GPU execution binaries and DLLs will be downloaded **on demand** and stored in the user's `AppData/Local` directory.

---

## 2. Technical Architecture

### 2.1 Settings Expansion
We introduce a new setting `local_acceleration` in the application configuration:
- `cpu` (default): Use pre-packaged CPU-only binaries.
- `cuda`: Use CUDA 11.x + cuDNN 8.x accelerated execution provider.
- `directml`: Use DirectML cross-GPU execution provider.

**Config file changes:**
- Modify `src-tauri/src/settings.rs` to include the `local_acceleration: String` field with a default of `"cpu"`.

---

### 2.2 Sidecar Dynamic Lookup and Execution
The `find_sherpa_websocket_server` and `find_sherpa_punctuation_exe` functions in [whisper_runner.rs](file:///G:/Aura/2.0/src-tauri/src/whisper_runner.rs) will resolve the executables as follows:

1. If `local_acceleration` is `"cpu"`: Look up the default shipped CPU binaries in the app resource directory.
2. If `local_acceleration` is `"cuda"` or `"directml"`:
   - Check if the GPU binary package is downloaded in `AppData/Local/com.aura.app/binaries/<provider>/bin`.
   - If the executable exists there, return its path.
   - Fall back to the default CPU binary if the GPU path is invalid.

When spawning the websocket server process, append the corresponding `--provider=cuda` or `--provider=directml` argument.

---

### 2.3 Download and Extraction Lifecycles
We will implement two new Tauri commands in `src-tauri/src/lib.rs`:

1. `download_gpu_binaries(provider: String)`:
   - Starts a background download task using a Rust HTTP client (`reqwest`).
   - Downloads the target package:
     - CUDA: `https://github.com/malashkadev/Aura/releases/download/v1.0.8-assets/sherpa-onnx-v1.13.4-win-x64-cuda.tar.bz2` (includes all CUDA runtime DLLs: `cudart64_110.dll`, `cublas64_11.dll`, `cublasLt64_11.dll`, `cudnn64_8.dll`, `cufft64_10.dll`).
     - DirectML: `https://github.com/malashkadev/Aura/releases/download/v1.0.8-assets/sherpa-onnx-v1.13.4-win-x64-directml.tar.bz2` (includes `DirectML.dll` and DirectML-enabled `onnxruntime.dll`).
   - Emits progress events `gpu-download-progress` to the frontend.
   - Extracts the downloaded archive directly to `AppData/Local/com.aura.app/binaries/<provider>` using a spawning `tar -xf` process (since `tar` is native to Windows 10/11).
   - Deletes the temporary archive.

2. `delete_gpu_binaries(provider: String)`:
   - Deletes the local directory `AppData/Local/com.aura.app/binaries/<provider>`.

---

## 3. UI/UX Design System Alignment

To ensure a seamless, premium look aligned with **The Stealth Command Deck** visual theme, we will replicate the exact UI patterns used for model management:

1. **GPU Cards Group**:
   Render the acceleration options in settings as `model-card` blocks:
   - **CPU**: Status `✓ УСТАНОВЛЕНО` (orange pill, no delete button).
   - **NVIDIA CUDA**: Pill button `[icon] Скачать` if not downloaded. Show progress bar if downloading. Show `✓ УСТАНОВЛЕНО` pill with a trash bin button `[delete]` if downloaded.
   - **DirectML**: Similar behavior to CUDA.

2. **Localization**:
   We will update the localized dictionary in `src/main.js` with:
   - `gpu_accel_label`: "Локальное аппаратное ускорение"
   - `gpu_accel_desc`: "Перенесите обработку речи с CPU на видеокарту для мгновенной транскрибации."
   - `gpu_accel_cpu_title`: "CPU (Без ускорения)"
   - `gpu_accel_cpu_desc`: "Стандартный режим. Безопасен, но нагружает процессор."
   - `gpu_accel_cuda_title`: "NVIDIA CUDA (Макс. скорость)"
   - `gpu_accel_cuda_desc`: "Для видеокарт GeForce RTX/GTX. Использование тензорных ядер."
   - `gpu_accel_dml_title`: "DirectML (Универсальный)"
   - `gpu_accel_dml_desc`: "Для видеокарт AMD, Intel и NVIDIA. Базовое ускорение."

---

## 4. Verification and Safety

- **Safety Checks:** If a GPU binary fails to start (e.g. incompatible drivers), the application will capture the failure and fall back automatically to the CPU server.
- **Compilation Check:** Code compiles cleanly under `cargo build` in `src-tauri`.
