import { bindTabKeyboardNavigation } from "./ui-accessibility.js";

// Retrieve Tauri APIs from window.__TAURI__
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

window.logEvent = function(level, tag, message) {
  invoke("log_frontend_event", { level, tag, session: null, message }).catch(err => {
    console.error("Failed to log frontend event", err);
  });
};

const i18nDict = {
  ru: {
    title_settings: "Настройки",
    tab_general: "Основные",
    tab_speech: "Голос",
    tab_hotkeys: "Управление",
    tab_apikeys: "Облако",
    section_cloud_functions: "Облачные функции",
    section_engine: "Движок",
    section_recognition: "Распознавание",
    section_input: "Ввод",
    section_dictionary: "Словарь",
    tab_history: "История",
    tab_about: "О программе",
    general_autostart_title: "Автозапуск Aura",
    general_autostart_desc: "Запускать приложение автоматически при входе в операционную систему Windows.",
    general_autostart_checkbox: "Запускать Aura при старте системы",
    engine_title: "Способ распознавания",
    engine_desc: "Выберите между облачной обработкой высокого качества или полностью автономным локальным распознаванием речи.",
    engine_cloud: "Облачный ИИ",
    engine_cloud_meta: "Gemini / OpenAI / Groq (требуется API-ключ)",
    engine_local: "Локальный ИИ",
    engine_local_meta: "Whisper / Parakeet (100% оффлайн)",
    lang_bias_title: "Язык распознавания",
    lang_bias_desc: "Выберите принудительный язык ввода или включите автоопределение.",
    lang_bias_label: "Выберите язык",
    lang_opt_auto: "Автоопределение (по умолчанию)",
    lang_opt_layout: "По раскладке клавиатуры",
    streaming_title: "Режим ввода текста",
    streaming_desc: "Выберите способ отображения наговариваемого текста.",
    streaming_checkbox: "Потоковый ввод в реальном времени (экспериментальный)",
    streaming_subdesc: "Если выключено: текст вставится целиком только после того, как вы отпустите клавиши.",
    punct_title: "Интеллектуальная пунктуация",
    punct_desc: "Преобразовывать голосовые команды (\"запятая\", \"точка с запятой\") в знаки препинания.",
    punct_checkbox: "Включить обработку голосовой пунктуации",
    vocab_title: "Пользовательский словарь",
    vocab_desc: "Внесите термины, имена или брендовые названия через запятую, чтобы улучшить их распознавание.",
    vocab_placeholder: "Например: Аура, коммит, репозиторий...",
    punct_model_label: "Пунктуация (для английского)",
    punct_model_name: "CT-Transformer (zh-en, int8)",
    punct_model_meta: "~62 МБ — голосовая пунктуация",
    engine_health_whisper: "Whisper: встроенный движок, запускается по требованию",
    engine_health_parakeet_running: "Parakeet: сервер запущен ({provider}, порт {port})",
    engine_health_parakeet_stopped: "Parakeet: сервер не запущен",
    local_model_title: "Локальное распознавание",
    local_model_desc: "Настройте локальный движок распознавания речи для полной приватности.",
    local_model_label: "Размер модели",
    model_meta_tiny: "~75 МБ — сверхбыстрая",
    model_meta_base: "~145 МБ — рекомендуемая",
    model_meta_small: "~465 МБ — точная для русского",
    model_meta_medium: "~1.5 ГБ — продвинутая",
    model_meta_turbo: "~1.6 ГБ — лучшая точность для RU/EN",
    model_meta_turbo_q5: "~550 МБ — почти как Turbo, вдвое легче",
    model_cancel_download: "Отменить загрузку",
    model_download_cancelled: "Загрузка отменена",
    update_available: "Доступно обновление",
    hotkey_title: "Глобальная горячая клавиша",
    hotkey_desc: "Зажмите выбранную комбинацию для начала записи, отпустите для распознавания.",
    hotkey_label: "Комбинация",
    hotkey_toggle_mode: "Режим переключателя (короткое нажатие)",
    hotkey_toggle_mode_desc: "Короткое нажатие начинает запись без удержания клавиши. Повторный клик останавливает запись.",
    sound_title: "Звуковое сопровождение",
    sound_desc: "Звуковые эффекты оверлея при записи.",
    sound_enable: "Включить звуки оверлея",
    sound_volume_label: "Громкость звука",
    sound_theme_label: "Звуковая тема",
    sound_theme_zen: "Дзен (Поющие чаши)",
    sound_theme_rhodes: "Rhodes (Джаз-электропианино)",
    sound_theme_scifi: "Sci-Fi (Космический)",
    sound_theme_classic: "Колокольчик (Классический)",
    api_title: "Авторизация API-ключей",
    api_desc: "Укажите ваши API-ключи для авторизации в облачных сервисах Gemini, OpenAI или Groq.",
    api_provider: "Провайдер API",
    api_key: "API-ключ",
    api_key_placeholder: "Введите ваш API-ключ...",
    hotkey_prompt: "Нажмите клавиши...",
    key_saved_placeholder: "•••••••• (сохранён безопасно)",
    key_placeholder: "Введите API-ключ",
    api_get_key: "Получить ключ API",
    history_title: "История транскрипций",
    history_clear: "Очистить историю",
    history_desc: "Последние надиктованные фразы хранятся локально.",
    history_empty: "История пуста. Ваши надиктованные тексты будут отображаться здесь.",
    history_badge_cloud: "Облако",
    history_badge_local: "Локально",
    history_engine_whisper: "Whisper",
    history_engine_parakeet: "NVIDIA Parakeet",
    history_unit_ms: "мс",
    history_unit_sec: "с",
    about_app_title: "Голосовой ввод Aura",
    about_version: "v1.0.9",
    about_description: "Инструмент глобального голосового ввода для Windows. Программа переводит речь в текст и вставляет его в любое активное окно с автоматическим форматированием и расстановкой пунктуации.",
    status_ready: "Готово",
    btn_save: "Сохранить настройки",
    confirm_title: "Подтверждение",
    confirm_message: "Вы действительно хотите выполнить это действие?",
    confirm_cancel: "Отмена",
    confirm_ok: "Подтвердить",
    status_loading: "Загрузка настроек...",
    status_modified: "Настройки изменены (не сохранены)",
    status_saving: "Сохранение настроек...",
    status_saved: "Настройки успешно сохранены!",
    status_error: "Ошибка: ",
    model_status_ready: "Установлено",
    model_action_download: "Скачать",
    model_action_delete: "Удалить",
    api_get_key_pattern: "Получить ключ на {name}",
    status_loaded: "Настройки загружены",
    status_load_error: "Ошибка загрузки настроек: ",
    status_save_error: "Ошибка сохранения настроек: ",
    model_downloading_pattern: "Запуск скачивания для модели '{model}'...",
    model_download_error_pattern: "Ошибка скачивания: {err}",
    delete_model_title: "Удаление модели",
    delete_model_confirm_pattern: "Вы действительно хотите удалить локальную модель '{model}'?",
    delete_model_btn: "Удалить",
    model_deleting_pattern: "Удаление модели '{model}'...",
    model_deleted_success: "Модель успешно удалена",
    model_delete_error_pattern: "Ошибка удаления: {err}",
    model_downloaded_success_pattern: "Модель '{model}' скачана!",
    confirm_clear_history_title: "Очистить историю",
    confirm_clear_history_msg: "Вы действительно хотите очистить всю историю транскрипций?",
    general_ui_lang_title: "Язык интерфейса",
    general_ui_lang_desc: "Выберите язык для отображения настроек и уведомлений приложения.",
    update_checks_title: "Проверка обновлений",
    update_checks_desc: "Aura обращается к GitHub только при ручной проверке или если вы включили автоматическую проверку.",
    update_checks_checkbox: "Автоматически проверять обновления при запуске",
    update_check_now: "Проверить обновления",
    cloud_data_desc: "Облачному провайдеру передаются аудио и транскрипт, а при включённых функциях — выделенный текст и пользовательский словарь. Локальный режим эти данные не отправляет.",
    update_current: "Установлена актуальная версия Aura.",
    update_available_pattern: "Доступна Aura v{version}.",
    update_check_error_pattern: "Не удалось проверить обновления: {error}",
    update_installing: "Скачивание, проверка подписи и установка обновления...",
    update_installed_restarting: "Обновление установлено. Перезапуск...",
    update_install_error_open_release: "Не удалось установить обновление. Открываю страницу релиза...",
    hotkey_reset_title: "Сбросить на Alt+V",
    local_engine_label: "Движок распознавания",
    local_engine_whisper: "Whisper.cpp (на базе OpenAI Whisper)",
    local_engine_parakeet: "NVIDIA Parakeet (через sherpa-onnx)",
    parakeet_model_label: "Модель Parakeet",
    model_meta_parakeet: "~670 МБ — оптимизировано NVIDIA",
    fallback_title: "Автопереключение при недоступности облака",
    fallback_desc: "Если облачный ИИ недоступен (VPN, блокировка региона, нет сети), автоматически использовать уже скачанную локальную модель для этой записи.",
    fallback_checkbox: "Включить автопереключение на локальную модель",
    copy_context_title: "Редактирование выделенного текста",
    copy_context_desc: "Если включено, Aura отправляет Ctrl+C и передаёт выделенный текст выбранному облачному провайдеру как контекст для команды редактирования. Отключите эту функцию при работе в терминале.",
    copy_context_checkbox: "Разрешить захват и облачное редактирование выделения",
    gpu_accel_label: "Локальное аппаратное ускорение",
    gpu_accel_cpu_title: "CPU (без ускорения)",
    gpu_accel_cpu_desc: "Стандартный режим. Надёжен, но нагружает процессор.",
    gpu_accel_cuda_title: "NVIDIA CUDA (максимальная скорость)",
    gpu_accel_cuda_desc: "Для видеокарт GeForce RTX/GTX. Использует тензорные ядра.",
    gpu_accel_dml_title: "DirectML (универсальный)",
    gpu_accel_dml_desc: "Для видеокарт AMD, Intel и NVIDIA. Базовое ускорение.",
    btn_copy_diagnostics: "Скопировать отчет диагностики",
    toast_diagnostics_copied: "Отчет диагностики скопирован в буфер обмена!",
    diag_speech_text_title: "Логирование текста речи (режим разработчика)",
    diag_title: "Диагностика",
    diag_speech_text_desc: "Сохранять точный текст распознанной речи в диагностические логи. По умолчанию выключено для приватности.",
    diag_speech_text_checkbox: "Записывать текст речи в логи"
  },
  en: {
    title_settings: "Settings",
    tab_general: "General",
    tab_speech: "Speech",
    tab_hotkeys: "Hotkeys",
    tab_apikeys: "Cloud",
    section_cloud_functions: "Cloud features",
    section_engine: "Recognition engine",
    section_recognition: "Recognition",
    section_input: "Input",
    section_dictionary: "Dictionary",
    tab_history: "History",
    tab_about: "About",
    general_autostart_title: "Aura Autostart",
    general_autostart_desc: "Launch the app automatically when starting Windows.",
    general_autostart_checkbox: "Start Aura at system boot",
    engine_title: "Processing Type",
    engine_desc: "Choose between high-quality cloud transcription or fully local speech recognition.",
    engine_cloud: "Cloud AI",
    engine_cloud_meta: "Gemini / OpenAI / Groq (API key required)",
    engine_local: "Local AI",
    engine_local_meta: "Whisper / Parakeet (100% offline & private)",
    lang_bias_title: "Speech Language",
    lang_bias_desc: "Forcibly set transcription language or use automatic detection.",
    lang_bias_label: "Select Language",
    lang_opt_auto: "Auto-detect (default)",
    lang_opt_layout: "Follow Keyboard Layout",
    streaming_title: "Text Streaming",
    streaming_desc: "Choose how transcribed text is displayed.",
    streaming_checkbox: "Real-time streaming typing (experimental)",
    streaming_subdesc: "If disabled: text is typed as a whole only when you release hotkeys.",
    punct_title: "Smart Punctuation",
    punct_desc: "Convert spoken punctuation commands (like \"comma\", \"period\") into punctuation.",
    punct_checkbox: "Enable spoken punctuation processing",
    vocab_title: "Custom Vocabulary",
    vocab_desc: "Add specific terms, names, or jargon separated by commas to improve recognition.",
    vocab_placeholder: "e.g. Aura, commit, repository...",
    punct_model_label: "Punctuation (for English)",
    punct_model_name: "CT-Transformer (zh-en, int8)",
    punct_model_meta: "~62 MB — spoken punctuation",
    engine_health_whisper: "Whisper: in-process engine, spawned on demand",
    engine_health_parakeet_running: "Parakeet: server running ({provider}, port {port})",
    engine_health_parakeet_stopped: "Parakeet: server not running",
    local_model_title: "Local Recognition",
    local_model_desc: "Configure a local speech-to-text engine for complete privacy.",
    local_model_label: "Model Size",
    model_meta_tiny: "~75 MB — superfast",
    model_meta_base: "~145 MB — recommended",
    model_meta_small: "~465 MB — accurate",
    model_meta_medium: "~1.5 GB — advanced",
    model_meta_turbo: "~1.6 GB — best accuracy for RU/EN",
    model_meta_turbo_q5: "~550 MB — near-Turbo, half the size",
    model_cancel_download: "Cancel download",
    model_download_cancelled: "Download cancelled",
    update_available: "Update available",
    hotkey_title: "Global Hotkey",
    hotkey_desc: "Hold down the selected hotkey to record, release to transcribe.",
    hotkey_label: "Combination",
    hotkey_toggle_mode: "Toggle mode (short tap)",
    hotkey_toggle_mode_desc: "Short tap starts/stops recording without holding key down.",
    sound_title: "Overlay Audio Feedback",
    sound_desc: "Audio sound effects when recording states change.",
    sound_enable: "Enable overlay sounds",
    sound_volume_label: "Sound Volume",
    sound_theme_label: "Sound Theme",
    sound_theme_zen: "Zen (Singing Bowls)",
    sound_theme_rhodes: "Rhodes (Jazz Electric Piano)",
    sound_theme_scifi: "Sci-Fi (Space/Synth)",
    sound_theme_classic: "Bell (Classic)",
    api_title: "API Keys Authorization",
    api_desc: "Provide API keys for Gemini, OpenAI, or Groq cloud services.",
    api_provider: "API Provider",
    api_key: "API Key",
    api_key_placeholder: "Enter your API key...",
    hotkey_prompt: "Press keys...",
    key_saved_placeholder: "•••••••• (saved securely)",
    key_placeholder: "Enter API key",
    api_get_key: "Get API Key",
    history_title: "Transcription History",
    history_clear: "Clear History",
    history_desc: "Your latest transcribed phrases are cached locally.",
    history_empty: "History is empty. Dictated text fragments will appear here.",
    history_badge_cloud: "Cloud",
    history_badge_local: "Local",
    history_engine_whisper: "Whisper",
    history_engine_parakeet: "NVIDIA Parakeet",
    history_unit_ms: "ms",
    history_unit_sec: "s",
    about_app_title: "Aura Voice Input",
    about_version: "v1.0.9",
    about_description: "Global voice input tool for Windows. The program transcribes speech to text and inserts it into any active window with automatic formatting and punctuation.",
    status_ready: "Ready",
    btn_save: "Save Settings",
    confirm_title: "Confirmation",
    confirm_message: "Are you sure you want to perform this action?",
    confirm_cancel: "Cancel",
    confirm_ok: "Confirm",
    status_loading: "Loading settings...",
    status_modified: "Settings changed (unsaved)",
    status_saving: "Saving settings...",
    status_saved: "Settings saved successfully!",
    status_error: "Error: ",
    model_status_ready: "Installed",
    model_action_download: "Download",
    model_action_delete: "Delete",
    api_get_key_pattern: "Get key on {name}",
    status_loaded: "Settings loaded",
    status_load_error: "Failed to load settings: ",
    status_save_error: "Failed to save settings: ",
    model_downloading_pattern: "Starting download for model '{model}'...",
    model_download_error_pattern: "Download error: {err}",
    delete_model_title: "Delete model",
    delete_model_confirm_pattern: "Are you sure you want to delete the local model '{model}'?",
    delete_model_btn: "Delete",
    model_deleting_pattern: "Deleting model '{model}'...",
    model_deleted_success: "Model deleted successfully",
    model_delete_error_pattern: "Delete error: {err}",
    model_downloaded_success_pattern: "Model '{model}' downloaded!",
    confirm_clear_history_title: "Clear history",
    confirm_clear_history_msg: "Are you sure you want to clear all transcription history?",
    general_ui_lang_title: "Interface Language",
    general_ui_lang_desc: "Select the language for settings and application notifications.",
    update_checks_title: "Update checks",
    update_checks_desc: "Aura contacts GitHub only when you check manually or enable automatic checks.",
    update_checks_checkbox: "Check for updates automatically at startup",
    update_check_now: "Check for updates",
    cloud_data_desc: "The selected cloud provider receives audio and the transcript and, when the related features are enabled, selected text and the custom dictionary. Local mode does not send this data.",
    update_current: "Aura is up to date.",
    update_available_pattern: "Aura v{version} is available.",
    update_check_error_pattern: "Could not check for updates: {error}",
    update_installing: "Downloading, verifying the signature, and installing the update...",
    update_installed_restarting: "Update installed. Restarting...",
    update_install_error_open_release: "Could not install the update. Opening the release page...",
    hotkey_reset_title: "Reset to Alt+V",
    local_engine_label: "ASR Engine",
    local_engine_whisper: "Whisper.cpp (OpenAI Whisper)",
    local_engine_parakeet: "NVIDIA Parakeet (via sherpa-onnx)",
    parakeet_model_label: "Parakeet Model",
    model_meta_parakeet: "~670 MB — optimized by NVIDIA",
    fallback_title: "Automatic fallback when cloud is unavailable",
    fallback_desc: "If cloud AI is unavailable (VPN, region block, no network), automatically use the already downloaded local model for this recording.",
    fallback_checkbox: "Enable automatic fallback to local model",
    copy_context_title: "Edit selected text",
    copy_context_desc: "When enabled, Aura sends Ctrl+C and passes the selected text to the chosen cloud provider as context for an editing command. Disable this feature when working in a terminal.",
    copy_context_checkbox: "Allow selection capture and cloud editing",
    gpu_accel_label: "Local hardware acceleration",
    gpu_accel_cpu_title: "CPU (no acceleration)",
    gpu_accel_cpu_desc: "Standard mode. Reliable, but uses more CPU.",
    gpu_accel_cuda_title: "NVIDIA CUDA (maximum speed)",
    gpu_accel_cuda_desc: "For GeForce RTX/GTX GPUs. Uses Tensor Cores.",
    gpu_accel_dml_title: "DirectML (universal)",
    gpu_accel_dml_desc: "For AMD, Intel, and NVIDIA GPUs. Basic acceleration.",
    btn_copy_diagnostics: "Copy Diagnostic Report",
    toast_diagnostics_copied: "Diagnostic report copied to clipboard!",
    diag_speech_text_title: "Log Speech Text (Developer Mode)",
    diag_title: "Diagnostics",
    diag_speech_text_desc: "Include exact transcribed speech text in diagnostic logs. Disabled by default for privacy.",
    diag_speech_text_checkbox: "Include speech text in logs"
  },
  de: {
    gpu_accel_label: "Lokale Hardware-Beschleunigung",
    gpu_accel_cpu_title: "CPU (keine Beschleunigung)",
    gpu_accel_cpu_desc: "Standardmodus. Zuverlässig, beansprucht aber die CPU.",
    gpu_accel_cuda_title: "NVIDIA CUDA (maximale Geschwindigkeit)",
    gpu_accel_cuda_desc: "Für GeForce RTX/GTX-Grafikkarten. Nutzt Tensor Cores.",
    gpu_accel_dml_title: "DirectML (universell)",
    gpu_accel_dml_desc: "Für AMD-, Intel- und NVIDIA-Grafikkarten. Basisbeschleunigung.",
    title_settings: "Einstellungen",
    tab_general: "Allgemein",
    tab_speech: "Diktat",
    tab_hotkeys: "Tastenkombinationen",
    tab_apikeys: "Cloud",
    section_cloud_functions: "Cloud-Funktionen",
    section_engine: "Erkennungsmodul",
    section_recognition: "Spracherkennung",
    section_input: "Eingabe",
    section_dictionary: "Wörterbuch",
    tab_history: "Verlauf",
    tab_about: "Über Aura",
    general_autostart_title: "Aura Autostart",
    general_autostart_desc: "Startet die App automatisch beim Anmelden in Windows.",
    general_autostart_checkbox: "Aura beim Systemstart starten",
    engine_title: "Verarbeitungstyp",
    engine_desc: "Wählen Sie zwischen Cloud-Transkription oder vollständig lokaler Spracherkennung.",
    engine_cloud: "Cloud-KI",
    engine_cloud_meta: "Gemini / OpenAI / Groq (API-Schlüssel erforderlich)",
    engine_local: "Lokale KI",
    engine_local_meta: "Whisper / Parakeet (100% offline & privat)",
    lang_bias_title: "Sprache",
    lang_bias_desc: "Wählen Sie eine feste Sprache für das Diktat oder aktivieren Sie die Auto-Erkennung.",
    lang_bias_label: "Sprache auswählen",
    lang_opt_auto: "Auto-Erkennung (Standard)",
    lang_opt_layout: "Tastaturlayout folgen",
    streaming_title: "Text-Streaming",
    streaming_desc: "Wählen Sie, wie die Transkription eingegeben wird.",
    streaming_checkbox: "Echtzeit-Streaming-Eingabe (experimentell)",
    streaming_subdesc: "Wenn deaktiviert: Text wird als Ganzes eingefügt, wenn die Taste losgelassen wird.",
    punct_title: "Intelligente Interpunktion",
    punct_desc: "Gesprochene Satzzeichen (z. B. \"Komma\", \"Punkt\") in Interpunktion umwandeln.",
    punct_checkbox: "Verarbeitung gesprochener Satzzeichen aktivieren",
    vocab_title: "Eigenes Wörterbuch",
    vocab_desc: "Tragen Sie Begriffe, Namen oder Fachbegriffe durch Komma getrennt ein, um die Erkennung zu verbessern.",
    vocab_placeholder: "z.B. Aura, Commit, Repository...",
    punct_model_label: "Interpunktion (für Englisch)",
    punct_model_name: "CT-Transformer (zh-en, int8)",
    punct_model_meta: "~62 MB — Sprachinterpunktion",
    engine_health_whisper: "Whisper: integrierte Engine, wird bei Bedarf gestartet",
    engine_health_parakeet_running: "Parakeet: Server läuft ({provider}, Port {port})",
    engine_health_parakeet_stopped: "Parakeet: Server läuft nicht",
    local_model_title: "Lokales Whisper-Modell",
    local_model_desc: "Konfigurieren Sie eine lokale Spracherkennungs-Engine für vollständige Privatsphäre.",
    local_model_label: "Modellgröße",
    model_meta_tiny: "~75 MB — superschnell",
    model_meta_base: "~145 MB — empfohlen",
    model_meta_small: "~465 MB — präzise",
    model_meta_medium: "~1.5 GB — fortgeschritten",
    model_meta_turbo: "~1.6 GB — beste Genauigkeit für RU/EN",
    model_meta_turbo_q5: "~550 MB — fast wie Turbo, halb so groß",
    hotkey_title: "Globale Taste",
    hotkey_desc: "Tastenkombination gedrückt halten, um aufzunehmen, loslassen zur Transkription.",
    hotkey_label: "Kombination",
    hotkey_toggle_mode: "Umschaltmodus (kurzes Tippen)",
    hotkey_toggle_mode_desc: "Kurzes Antippen startet/stoppt Aufnahme ohne Halten.",
    sound_title: "Audio-Rückmeldung",
    sound_desc: "Soundeffekte des Overlays während der Aufnahme.",
    sound_enable: "Overlay-Sounds aktivieren",
    sound_volume_label: "Tonlautstärke",
    sound_theme_label: "Sound-Theme",
    sound_theme_zen: "Zen (Klangschalen)",
    sound_theme_rhodes: "Rhodes (Jazz Electric Piano)",
    sound_theme_scifi: "Sci-Fi (Weltraum)",
    sound_theme_classic: "Glocke (Klassisch)",
    api_title: "API-Schlüssel Autorisierung",
    api_desc: "Geben Sie Ihre API-Schlüssel für Gemini, OpenAI oder Groq Cloud-Dienste ein.",
    api_provider: "API-Provider",
    api_key: "API-Schlüssel",
    api_key_placeholder: "Geben Sie Ihren API-Schlüssel ein...",
    hotkey_prompt: "Tasten drücken...",
    key_saved_placeholder: "•••••••• (sicher gespeichert)",
    key_placeholder: "API-Schlüssel eingeben",
    api_get_key: "API-Schlüssel erhalten",
    history_title: "Diktatverlauf",
    history_clear: "Verlauf löschen",
    history_desc: "Die letzten aufgezeichneten Sätze werden lokal gespeichert.",
    history_empty: "Der Verlauf ist leer. Transkribierte Texte werden hier angezeigt.",
    history_badge_cloud: "Cloud",
    history_badge_local: "Lokal",
    history_engine_whisper: "Whisper",
    history_engine_parakeet: "NVIDIA Parakeet",
    history_unit_ms: "ms",
    history_unit_sec: "s",
    about_app_title: "Aura Spracheingabe",
    about_version: "v1.0.9",
    about_description: "Globales Spracheingabe-Tool für Windows. Die Anwendung überträgt Sprache in Text und fügt ihn mit automatischer Formatierung und Zeichensetzung in jedes aktive Fenster ein.",
    status_ready: "Bereit",
    btn_save: "Einstellungen speichern",
    confirm_title: "Bestätigung",
    confirm_message: "Sind Sie sicher, dass Sie diese Aktion ausführen möchten?",
    confirm_cancel: "Abbrechen",
    confirm_ok: "Bestätigen",
    status_loading: "Einstellungen werden geladen...",
    status_modified: "Einstellungen geändert (ungespeichert)",
    status_saving: "Einstellungen werden gespeichert...",
    status_saved: "Einstellungen erfolgreich gespeichert!",
    status_error: "Fehler: ",
    model_status_ready: "Installiert",
    model_action_download: "Herunterladen",
    model_action_delete: "Löschen",
    api_get_key_pattern: "Schlüssel erhalten auf {name}",
    status_loaded: "Einstellungen geladen",
    status_load_error: "Fehler beim Laden der Einstellungen: ",
    status_save_error: "Fehler beim Speichern der Einstellungen: ",
    model_downloading_pattern: "Download für Modell '{model}' wird gestartet...",
    model_download_error_pattern: "Download-Fehler: {err}",
    delete_model_title: "Modell löschen",
    delete_model_confirm_pattern: "Möchten Sie das lokale Modell '{model}' wirklich löschen?",
    delete_model_btn: "Löschen",
    model_deleting_pattern: "Modell '{model}' wird gelöscht...",
    model_deleted_success: "Modell erfolgreich gelöscht",
    model_delete_error_pattern: "Fehler beim Löschen: {err}",
    model_downloaded_success_pattern: "Modell '{model}' heruntergeladen!",
    confirm_clear_history_title: "Verlauf löschen",
    confirm_clear_history_msg: "Möchten Sie den gesamten Transkriptionsverlauf wirklich löschen?",
    general_ui_lang_title: "Sprache der Benutzeroberfläche",
    general_ui_lang_desc: "Wählen Sie die Sprache für Einstellungen und Benachrichtigungen.",
    update_checks_title: "Update-Prüfung",
    update_checks_desc: "Aura kontaktiert GitHub nur bei einer manuellen Prüfung oder wenn Sie automatische Prüfungen aktivieren.",
    update_checks_checkbox: "Beim Start automatisch nach Updates suchen",
    update_check_now: "Nach Updates suchen",
    cloud_data_desc: "Der ausgewählte Cloud-Anbieter erhält Audio und Transkript sowie, wenn die entsprechenden Funktionen aktiviert sind, ausgewählten Text und das Benutzerwörterbuch. Im lokalen Modus werden diese Daten nicht gesendet.",
    update_current: "Aura ist auf dem neuesten Stand.",
    update_available_pattern: "Aura v{version} ist verfügbar.",
    update_check_error_pattern: "Updates konnten nicht geprüft werden: {error}",
    update_installing: "Update wird heruntergeladen, die Signatur geprüft und die Installation ausgeführt...",
    update_installed_restarting: "Update installiert. Neustart...",
    update_install_error_open_release: "Update konnte nicht installiert werden. Die Release-Seite wird geöffnet...",
    hotkey_reset_title: "Auf Alt+V zurücksetzen",
    local_engine_label: "Erkennungsmodul",
    local_engine_whisper: "Whisper.cpp (OpenAI Whisper)",
    local_engine_parakeet: "NVIDIA Parakeet (über sherpa-onnx)",
    parakeet_model_label: "Parakeet-Modell",
    model_meta_parakeet: "~670 MB — optimiert von NVIDIA",
    model_cancel_download: "Download abbrechen",
    model_download_cancelled: "Download abgebrochen",
    update_available: "Update verfügbar",
    fallback_title: "Automatischer Wechsel bei nicht verfügbarer Cloud",
    fallback_desc: "Wenn die Cloud-KI nicht erreichbar ist (VPN, Regionssperre, kein Netzwerk), automatisch das bereits heruntergeladene lokale Modell für diese Aufnahme verwenden.",
    fallback_checkbox: "Automatischen Fallback auf lokales Modell aktivieren",
    copy_context_title: "Ausgewählten Text bearbeiten",
    copy_context_desc: "Wenn aktiviert, sendet Aura Strg+C und übermittelt den ausgewählten Text als Kontext für einen Bearbeitungsbefehl an den gewählten Cloud-Anbieter. Deaktivieren Sie diese Funktion bei der Arbeit im Terminal.",
    copy_context_checkbox: "Erfassen der Auswahl und Cloud-Bearbeitung zulassen",
    btn_copy_diagnostics: "Diagnosebericht kopieren",
    toast_diagnostics_copied: "Diagnosebericht in Zwischenablage kopiert!",
    diag_speech_text_title: "Sprachtext protokollieren (Entwicklermodus)",
    diag_title: "Diagnose",
    diag_speech_text_desc: "Exakten transkribierten Sprachtext in Diagnoseprotokollen speichern. Aus Datenschutzgründen standardmäßig deaktiviert.",
    diag_speech_text_checkbox: "Sprachtext in Protokolle aufnehmen"
  },
  es: {
    title_settings: "Ajustes",
    tab_general: "General",
    tab_speech: "Voz",
    tab_hotkeys: "Accesos rápidos",
    tab_apikeys: "Nube",
    section_cloud_functions: "Funciones en la nube",
    section_engine: "Motor de reconocimiento",
    section_recognition: "Reconocimiento",
    section_input: "Entrada",
    section_dictionary: "Diccionario",
    tab_history: "Historial",
    tab_about: "Acerca de",
    general_autostart_title: "Inicio automático",
    general_autostart_desc: "Iniciar la aplicación de forma automática al arrancar Windows.",
    general_autostart_checkbox: "Iniciar Aura con el sistema",
    engine_title: "Tipo de procesamiento",
    engine_desc: "Seleccione entre el procesamiento en la nube de alta calidad o el reconocimiento local totalmente autónomo.",
    engine_cloud: "IA en la nube",
    engine_cloud_meta: "Gemini / OpenAI / Groq (requiere clave API)",
    engine_local: "IA local",
    engine_local_meta: "Whisper / Parakeet (100% offline y privado)",
    lang_bias_title: "Idioma de dictado",
    lang_bias_desc: "Forzar un idioma específico para la transcripción o usar detección automática.",
    lang_bias_label: "Seleccionar idioma",
    lang_opt_auto: "Autodetectar (por defecto)",
    lang_opt_layout: "Según teclado activo",
    streaming_title: "Escritura fluida",
    streaming_desc: "Seleccione el método para mostrar el texto transcrito.",
    streaming_checkbox: "Escritura en tiempo real (experimental)",
    streaming_subdesc: "Si está desactivado: el texto se inserta completo tras soltar el atajo.",
    punct_title: "Puntuación inteligente",
    punct_desc: "Convertir comandos de voz (ej. \"coma\", \"punto\") en signos gráficos.",
    punct_checkbox: "Activar procesamiento de puntuación por voz",
    vocab_title: "Vocabulario personalizado",
    vocab_desc: "Añada términos específicos, nombres o siglas separados por comas para mejorar el dictado.",
    vocab_placeholder: "ej. Aura, commit, repositorio...",
    punct_model_label: "Puntuación (para inglés)",
    punct_model_name: "CT-Transformer (zh-en, int8)",
    punct_model_meta: "~62 MB — puntuación por voz",
    engine_health_whisper: "Whisper: motor integrado, se inicia bajo demanda",
    engine_health_parakeet_running: "Parakeet: servidor en ejecución ({provider}, puerto {port})",
    engine_health_parakeet_stopped: "Parakeet: servidor no en ejecución",
    local_model_title: "Modelo Whisper local",
    local_model_desc: "Configure un motor local de reconocimiento de voz para mantener la privacidad.",
    local_model_label: "Tamaño del modelo",
    model_meta_tiny: "~75 MB — superrápido",
    model_meta_base: "~145 MB — recomendado",
    model_meta_small: "~465 MB — preciso",
    model_meta_medium: "~1.5 GB — avanzado",
    model_meta_turbo: "~1.6 GB — mejor precisión para RU/EN",
    model_meta_turbo_q5: "~550 MB — casi como Turbo, mitad de tamaño",
    hotkey_title: "Acceso rápido global",
    hotkey_desc: "Mantenga presionadas las teclas seleccionadas para grabar, suéltelas para transcribir.",
    hotkey_label: "Combinación",
    hotkey_toggle_mode: "Modo alternar (pulsación corta)",
    hotkey_toggle_mode_desc: "Una pulsación corta inicia/detiene la grabación sin mantener la tecla.",
    sound_title: "Efectos de audio",
    sound_desc: "Efectos sonoros del overlay al grabar.",
    sound_enable: "Activar sonidos del overlay",
    sound_volume_label: "Volumen del sonido",
    sound_theme_label: "Tema sonoro",
    sound_theme_zen: "Zen (Cuencos tibetanos)",
    sound_theme_rhodes: "Rhodes (Piano eléctrico)",
    sound_theme_scifi: "Sci-Fi (Futurista)",
    sound_theme_classic: "Campana (Clásico)",
    api_title: "Autorización de claves API",
    api_desc: "Introduzca sus claves API para los servicios en la nube de Gemini, OpenAI o Groq.",
    api_provider: "Proveedor de API",
    api_key: "Clave API",
    api_key_placeholder: "Introduzca su clave API...",
    hotkey_prompt: "Pulse las teclas...",
    key_saved_placeholder: "•••••••• (guardado de forma segura)",
    key_placeholder: "Introduzca la clave API",
    api_get_key: "Obtener clave API",
    history_title: "Historial de transcripción",
    history_clear: "Limpiar historial",
    history_desc: "Las últimas frases dictadas se guardan de forma local.",
    history_empty: "El historial está vacío. Los textos dictados se mostrarán aquí.",
    history_badge_cloud: "Nube",
    history_badge_local: "Local",
    history_engine_whisper: "Whisper",
    history_engine_parakeet: "Parakeet de NVIDIA",
    history_unit_ms: "ms",
    history_unit_sec: "s",
    about_app_title: "Dictado por voz Aura",
    about_version: "v1.0.9",
    about_description: "Herramienta de entrada de voz global para Windows. El programa transcribe el habla en texto y lo inserta en cualquier ventana activa con formato y puntuación automáticos.",
    status_ready: "Listo",
    btn_save: "Guardar ajustes",
    confirm_title: "Confirmación",
    confirm_message: "¿Está seguro de realizar esta acción?",
    confirm_cancel: "Cancelar",
    confirm_ok: "Confirmar",
    status_loading: "Cargando ajustes...",
    status_modified: "Ajustes modificados (sin guardar)",
    status_saving: "Guardando ajustes...",
    status_saved: "¡Ajustes guardados correctamente!",
    status_error: "Error: ",
    model_status_ready: "Instalado",
    model_action_download: "Descargar",
    model_action_delete: "Eliminar",
    api_get_key_pattern: "Obtener clave en {name}",
    status_loaded: "Ajustes cargados",
    status_load_error: "Error al cargar los ajustes: ",
    status_save_error: "Error al guardar los ajustes: ",
    model_downloading_pattern: "Iniciando descarga para el modelo '{model}'...",
    model_download_error_pattern: "Error de descarga: {err}",
    delete_model_title: "Eliminar modelo",
    delete_model_confirm_pattern: "¿Está seguro de que desea eliminar el modelo local '{model}'?",
    delete_model_btn: "Eliminar",
    model_deleting_pattern: "Eliminando modelo '{model}'...",
    model_deleted_success: "Modelo eliminado correctamente",
    model_delete_error_pattern: "Error al eliminar: {err}",
    model_downloaded_success_pattern: "¡Modelo '{model}' descargado!",
    confirm_clear_history_title: "Limpiar historial",
    confirm_clear_history_msg: "¿Está seguro de que desea limpiar todo el historial de transcripciones?",
    general_ui_lang_title: "Idioma de la interfaz",
    general_ui_lang_desc: "Seleccione el idioma para los ajustes y las notificaciones.",
    update_checks_title: "Comprobación de actualizaciones",
    update_checks_desc: "Aura se conecta a GitHub solo al comprobar manualmente o al activar las comprobaciones automáticas.",
    update_checks_checkbox: "Buscar actualizaciones automáticamente al iniciar",
    update_check_now: "Buscar actualizaciones",
    cloud_data_desc: "El proveedor en la nube seleccionado recibe el audio y la transcripción y, cuando se activan las funciones correspondientes, el texto seleccionado y el diccionario personalizado. El modo local no envía estos datos.",
    update_current: "Aura está actualizada.",
    update_available_pattern: "Aura v{version} está disponible.",
    update_check_error_pattern: "No se pudieron buscar actualizaciones: {error}",
    update_installing: "Descargando, verificando la firma e instalando la actualización...",
    update_installed_restarting: "Actualización instalada. Reiniciando...",
    update_install_error_open_release: "No se pudo instalar la actualización. Abriendo la página de la versión...",
    gpu_accel_label: "Aceleración de hardware local",
    gpu_accel_cpu_title: "CPU (sin aceleración)",
    gpu_accel_cpu_desc: "Modo estándar. Es fiable, pero aumenta la carga del procesador.",
    gpu_accel_cuda_title: "NVIDIA CUDA (velocidad máxima)",
    gpu_accel_cuda_desc: "Para GPU GeForce RTX/GTX. Utiliza Tensor Cores.",
    gpu_accel_dml_title: "DirectML (universal)",
    gpu_accel_dml_desc: "Para GPU AMD, Intel y NVIDIA. Aceleración básica.",
    hotkey_reset_title: "Restablecer a Alt+V",
    local_engine_label: "Motor de reconocimiento",
    local_engine_whisper: "Whisper.cpp (OpenAI Whisper)",
    local_engine_parakeet: "NVIDIA Parakeet (vía sherpa-onnx)",
    parakeet_model_label: "Modelo Parakeet",
    model_meta_parakeet: "~670 MB — optimizado por NVIDIA",
    model_cancel_download: "Cancelar descarga",
    model_download_cancelled: "Descarga cancelada",
    update_available: "Actualización disponible",
    fallback_title: "Cambio automático si la nube no está disponible",
    fallback_desc: "Si la IA en la nube no está disponible (VPN, bloqueo regional, sin red), usar automáticamente el modelo local ya descargado para esta grabación.",
    fallback_checkbox: "Activar cambio automático al modelo local",
    copy_context_title: "Editar texto seleccionado",
    copy_context_desc: "Cuando está activado, Aura envía Ctrl+C y pasa el texto seleccionado al proveedor en la nube elegido como contexto para una orden de edición. Desactive esta función al trabajar en una terminal.",
    copy_context_checkbox: "Permitir captura de selección y edición en la nube",
    btn_copy_diagnostics: "Copiar informe de diagnóstico",
    toast_diagnostics_copied: "¡Informe de diagnóstico copiado al portapapeles!",
    diag_speech_text_title: "Registrar texto de voz (Modo desarrollador)",
    diag_title: "Diagnóstico",
    diag_speech_text_desc: "Incluir texto de voz transcrito exacto en los registros de diagnóstico. Desactivado por defecto por privacidad.",
    diag_speech_text_checkbox: "Incluir texto de voz en los registros"
  },
  fr: {
    title_settings: "Paramètres",
    tab_general: "Général",
    tab_speech: "Dictée",
    tab_hotkeys: "Raccourcis",
    tab_apikeys: "Cloud",
    section_cloud_functions: "Fonctions cloud",
    section_engine: "Moteur de reconnaissance",
    section_recognition: "Reconnaissance",
    section_input: "Saisie",
    section_dictionary: "Dictionnaire",
    tab_history: "Historique",
    tab_about: "À propos",
    general_autostart_title: "Lancement automatique",
    general_autostart_desc: "Lancer l'application automatiquement au démarrage de Windows.",
    general_autostart_checkbox: "Démarrer Aura avec Windows",
    engine_title: "Type de traitement",
    engine_desc: "Choisissez entre un traitement cloud de haute qualité ou une reconnaissance locale 100% hors ligne.",
    engine_cloud: "IA Cloud",
    engine_cloud_meta: "Gemini / OpenAI / Groq (clé API requise)",
    engine_local: "IA Locale",
    engine_local_meta: "Whisper / Parakeet (100% hors ligne et privé)",
    lang_bias_title: "Langue de dictée",
    lang_bias_desc: "Forcer une langue spécifique pour la dictée ou utiliser la détection automatique.",
    lang_bias_label: "Sélectionner la langue",
    lang_opt_auto: "Détection automatique",
    lang_opt_layout: "Selon le clavier actif",
    streaming_title: "Saisie en continu",
    streaming_desc: "Sélectionnez le mode d'affichage du texte transcrit.",
    streaming_checkbox: "Affichage du texte en temps réel (expérimental)",
    streaming_subdesc: "Si désactivé: le texte est inséré en une fois lorsque vous relâchez le raccourci.",
    punct_title: "Ponctuation intelligente",
    punct_desc: "Convertir les commandes vocales (ex. \"virgule\", \"point\") en signes de ponctuation.",
    punct_checkbox: "Activer le traitement de la ponctuation dictée",
    vocab_title: "Vocabulaire personnalisé",
    vocab_desc: "Ajoutez des termes spécifiques, noms propres ou sigles séparés par des virgules pour améliorer la dictée.",
    vocab_placeholder: "ex. Aura, commit, dépôt...",
    punct_model_label: "Ponctuation (pour l'anglais)",
    punct_model_name: "CT-Transformer (zh-en, int8)",
    punct_model_meta: "~62 Mo — ponctuation vocale",
    engine_health_whisper: "Whisper : moteur intégré, lancé à la demande",
    engine_health_parakeet_running: "Parakeet : serveur en cours d'exécution ({provider}, port {port})",
    engine_health_parakeet_stopped: "Parakeet : serveur non démarré",
    local_model_title: "Modèle Whisper local",
    local_model_desc: "Configurez un moteur local de reconnaissance vocale pour préserver entièrement votre confidentialité.",
    local_model_label: "Taille du modèle",
    model_meta_tiny: "~75 Mo — super rapide",
    model_meta_base: "~145 Mo — recommandé",
    model_meta_small: "~465 Mo — précis",
    model_meta_medium: "~1.5 Go — avancé",
    model_meta_turbo: "~1.6 Go — meilleure précision RU/EN",
    model_meta_turbo_q5: "~550 Mo — proche de Turbo, deux fois plus léger",
    hotkey_title: "Raccourci global",
    hotkey_desc: "Maintenez le raccourci pour enregistrer, relâchez pour transcrire.",
    hotkey_label: "Combinaison",
    hotkey_toggle_mode: "Mode alterné (appui court)",
    hotkey_toggle_mode_desc: "Un appui court démarre/arrête l'enregistrement sans maintenir la touche.",
    sound_title: "Retours audio",
    sound_desc: "Effets sonores de l'overlay lors de l'enregistrement.",
    sound_enable: "Activer les sons de l'overlay",
    sound_volume_label: "Volume du son",
    sound_theme_label: "Thème sonore",
    sound_theme_zen: "Zen (Bols chantants)",
    sound_theme_rhodes: "Rhodes (Piano électrique)",
    sound_theme_scifi: "Sci-Fi (Spatiale)",
    sound_theme_classic: "Cloche (Classique)",
    api_title: "Clés d'API",
    api_desc: "Saisissez vos clés d'API pour les services Gemini, OpenAI ou Groq.",
    api_provider: "Fournisseur d'API",
    api_key: "Clé d'API",
    api_key_placeholder: "Saisissez votre clé d'API...",
    hotkey_prompt: "Appuyez sur les touches...",
    key_saved_placeholder: "•••••••• (enregistré en toute sécurité)",
    key_placeholder: "Saisissez la clé API",
    api_get_key: "Obtenir une clé d'API",
    history_title: "Historique de dictée",
    history_clear: "Effacer l'historique",
    history_desc: "Les dernières phrases dictées sont enregistrées localement.",
    history_empty: "Historique vide. Vos textes transcrits s'afficheront ici.",
    history_badge_cloud: "Cloud",
    history_badge_local: "Local",
    history_engine_whisper: "Whisper",
    history_engine_parakeet: "Parakeet NVIDIA",
    history_unit_ms: "ms",
    history_unit_sec: "s",
    about_app_title: "Dictée vocale Aura",
    about_version: "v1.0.9",
    about_description: "Outil de saisie vocale globale pour Windows. Le programme transcrit la parole en texte et l'insère dans n'importe quelle fenêtre active avec un formatage et une ponctuation automatiques.",
    status_ready: "Prêt",
    btn_save: "Enregistrer",
    confirm_title: "Confirmation",
    confirm_message: "Voulez-vous vraiment effectuer cette action?",
    confirm_cancel: "Annuler",
    confirm_ok: "Confirmer",
    status_loading: "Chargement...",
    status_modified: "Modifications non enregistrées",
    status_saving: "Enregistrement...",
    status_saved: "Paramètres enregistrés !",
    status_error: "Erreur: ",
    model_status_ready: "Installé",
    model_action_download: "Télécharger",
    model_action_delete: "Supprimer",
    api_get_key_pattern: "Obtenir la clé sur {name}",
    status_loaded: "Paramètres chargés",
    status_load_error: "Échec du chargement des paramètres : ",
    status_save_error: "Échec de l'enregistrement des paramètres : ",
    model_downloading_pattern: "Démarrage du téléchargement pour le modèle '{model}'...",
    model_download_error_pattern: "Erreur de téléchargement: {err}",
    delete_model_title: "Supprimer le modèle",
    delete_model_confirm_pattern: "Voulez-vous vraiment supprimer le modèle local '{model}' ?",
    delete_model_btn: "Supprimer",
    model_deleting_pattern: "Suppression du modèle '{model}'...",
    model_deleted_success: "Modèle supprimé avec succès",
    model_delete_error_pattern: "Erreur de suppression: {err}",
    model_downloaded_success_pattern: "Modèle '{model}' téléchargé !",
    confirm_clear_history_title: "Effacer l'historique",
    confirm_clear_history_msg: "Voulez-vous vraiment effacer tout l'historique des transcriptions ?",
    general_ui_lang_title: "Langue de l'interface",
    general_ui_lang_desc: "Sélectionnez la langue pour les paramètres et les notifications de l'application.",
    update_checks_title: "Recherche de mises à jour",
    update_checks_desc: "Aura contacte GitHub uniquement lors d’une vérification manuelle ou si vous activez les vérifications automatiques.",
    update_checks_checkbox: "Rechercher automatiquement les mises à jour au démarrage",
    update_check_now: "Rechercher les mises à jour",
    cloud_data_desc: "Le fournisseur cloud sélectionné reçoit l’audio et la transcription ainsi que, lorsque les fonctions concernées sont activées, le texte sélectionné et le dictionnaire personnalisé. Le mode local n’envoie pas ces données.",
    update_current: "Aura est à jour.",
    update_available_pattern: "Aura v{version} est disponible.",
    update_check_error_pattern: "Impossible de rechercher les mises à jour : {error}",
    update_installing: "Téléchargement, vérification de la signature et installation de la mise à jour…",
    update_installed_restarting: "Mise à jour installée. Redémarrage…",
    update_install_error_open_release: "Impossible d’installer la mise à jour. Ouverture de la page de la version…",
    gpu_accel_label: "Accélération matérielle locale",
    gpu_accel_cpu_title: "CPU (sans accélération)",
    gpu_accel_cpu_desc: "Mode standard. Fiable, mais sollicite le processeur.",
    gpu_accel_cuda_title: "NVIDIA CUDA (vitesse maximale)",
    gpu_accel_cuda_desc: "Pour les GPU GeForce RTX/GTX. Utilise les Tensor Cores.",
    gpu_accel_dml_title: "DirectML (universel)",
    gpu_accel_dml_desc: "Pour les GPU AMD, Intel et NVIDIA. Accélération de base.",
    hotkey_reset_title: "Réinitialiser à Alt+V",
    local_engine_label: "Moteur de reconnaissance",
    local_engine_whisper: "Whisper.cpp (OpenAI Whisper)",
    local_engine_parakeet: "NVIDIA Parakeet (via sherpa-onnx)",
    parakeet_model_label: "Modèle Parakeet",
    model_meta_parakeet: "~670 Mo — optimisé par NVIDIA",
    model_cancel_download: "Annuler le téléchargement",
    model_download_cancelled: "Téléchargement annulé",
    update_available: "Mise à jour disponible",
    fallback_title: "Basculement automatique si le cloud est indisponible",
    fallback_desc: "Si l'IA cloud est indisponible (VPN, blocage régional, pas de réseau), utiliser automatiquement le modèle local déjà téléchargé pour cet enregistrement.",
    fallback_checkbox: "Activer le basculement automatique vers le modèle local",
    copy_context_title: "Modifier le texte sélectionné",
    copy_context_desc: "Lorsque cette option est activée, Aura envoie Ctrl+C et transmet le texte sélectionné au fournisseur cloud choisi comme contexte d’une commande de modification. Désactivez-la lorsque vous travaillez dans un terminal.",
    copy_context_checkbox: "Autoriser la capture de la sélection et la modification dans le cloud",
    btn_copy_diagnostics: "Copier le rapport de diagnostic",
    toast_diagnostics_copied: "Rapport de diagnostic copié dans le presse-papiers !",
    diag_speech_text_title: "Consigner le texte vocal (Mode développeur)",
    diag_title: "Diagnostic",
    diag_speech_text_desc: "Inclure le texte vocal transcrit exact dans les journaux de diagnostic. Désactivé par défaut par confidentialité.",
    diag_speech_text_checkbox: "Inclure le texte vocal dans les journaux"
  },
  it: {
    title_settings: "Impostazioni",
    tab_general: "Generale",
    tab_speech: "Dettatura",
    tab_hotkeys: "Scorciatoie",
    tab_apikeys: "Cloud",
    section_cloud_functions: "Funzioni cloud",
    section_engine: "Motore di riconoscimento",
    section_recognition: "Riconoscimento",
    section_input: "Digitazione",
    section_dictionary: "Dizionario",
    tab_history: "Cronologia",
    tab_about: "Informazioni",
    general_autostart_title: "Avvio automatico",
    general_autostart_desc: "Avvia l'app automaticamente all'accesso di Windows.",
    general_autostart_checkbox: "Avvia Aura con il sistema",
    engine_title: "Tipo di elaborazione",
    engine_desc: "Scegli tra l'elaborazione cloud di alta qualità o il riconoscimento locale offline.",
    engine_cloud: "IA Cloud",
    engine_cloud_meta: "Gemini / OpenAI / Groq (chiave API richiesta)",
    engine_local: "IA Locale",
    engine_local_meta: "Whisper / Parakeet (100% offline e privato)",
    lang_bias_title: "Lingua dettatura",
    lang_bias_desc: "Imposta una lingua fissa per la transrizione o usa il rilevamento automatico.",
    lang_bias_label: "Seleziona lingua",
    lang_opt_auto: "Rilevamento automatico",
    lang_opt_layout: "In base alla tastiera",
    streaming_title: "Dattilografia a scorrimento",
    streaming_desc: "Seleziona come visualizzare il testo trascritto.",
    streaming_checkbox: "Inserimento del testo in tempo reale (sperimentale)",
    streaming_subdesc: "Se disattivato: il testo viene inserito interamente solo quando rilasci la scorciatoia.",
    punct_title: "Punteggiatura intelligente",
    punct_desc: "Converte i comandi vocali (es. \"virgola\", \"punto\") in simboli grafici.",
    punct_checkbox: "Attiva elaborazione della punteggiatura vocale",
    vocab_title: "Vocabolario personalizzato",
    vocab_desc: "Aggiungi parole specifiche, nomi o acronimi separati da virgole per migliorare la precisione.",
    vocab_placeholder: "es. Aura, commit, repository...",
    punct_model_label: "Punteggiatura (per l'inglese)",
    punct_model_name: "CT-Transformer (zh-en, int8)",
    punct_model_meta: "~62 MB — punteggiatura vocale",
    engine_health_whisper: "Whisper: motore integrato, avviato su richiesta",
    engine_health_parakeet_running: "Parakeet: server in esecuzione ({provider}, porta {port})",
    engine_health_parakeet_stopped: "Parakeet: server non in esecuzione",
    local_model_title: "Modello Whisper locale",
    local_model_desc: "Configura un motore locale di riconoscimento vocale per la massima privacy.",
    local_model_label: "Dimensione modello",
    model_meta_tiny: "~75 MB — superveloce",
    model_meta_base: "~145 MB — consigliato",
    model_meta_small: "~465 MB — preciso",
    model_meta_medium: "~1.5 GB — avanzato",
    model_meta_turbo: "~1.6 GB — massima precisione RU/EN",
    model_meta_turbo_q5: "~550 MB — quasi come Turbo, metà del peso",
    hotkey_title: "Tasto di scelta rapida",
    hotkey_desc: "Tieni premuto il tasto per registrare, rilascelo per trascrivere.",
    hotkey_label: "Scorciatoia",
    hotkey_toggle_mode: "Modalità alternata (tocco breve)",
    hotkey_toggle_mode_desc: "Un tocco breve avvia/ferma la registrazione senza tenere premuto.",
    sound_title: "Feedback sonori",
    sound_desc: "Effetti acustici dell'overlay durante la registrazione.",
    sound_enable: "Attiva i suoni dell'overlay",
    sound_volume_label: "Volume del suono",
    sound_theme_label: "Tema sonoro",
    sound_theme_zen: "Zen (Campane tibetane)",
    sound_theme_rhodes: "Rhodes (Piano elettrico)",
    sound_theme_scifi: "Sci-Fi (Spaziale)",
    sound_theme_classic: "Campanella (Classico)",
    api_title: "Autorizzazione chiavi API",
    api_desc: "Inserisci le tue chiavi API per Gemini, OpenAI o Groq.",
    api_provider: "Provider API",
    api_key: "Chiave API",
    api_key_placeholder: "Inserisci la tua chiave API...",
    hotkey_prompt: "Premi i tasti...",
    key_saved_placeholder: "•••••••• (salvato in modo sicuro)",
    key_placeholder: "Inserisci la chiave API",
    api_get_key: "Ottieni chiave API",
    history_title: "Cronologia dettati",
    history_clear: "Cancella cronologia",
    history_desc: "Le ultime frasi dettate vengono salvate in locale.",
    history_empty: "La cronologia è vuota. I testi dettati appariranno qui.",
    history_badge_cloud: "Cloud",
    history_badge_local: "Locale",
    history_engine_whisper: "Whisper",
    history_engine_parakeet: "Parakeet NVIDIA",
    history_unit_ms: "ms",
    history_unit_sec: "s",
    about_app_title: "Dettatura vocale Aura",
    about_version: "v1.0.9",
    about_description: "Strumento di inserimento vocale globale per Windows. Il programma trascrive la voce in testo e la inserisce in qualsiasi finestra attiva con formattazione e punteggiatura automatiche.",
    status_ready: "Pronto",
    btn_save: "Salva impostazioni",
    confirm_title: "Conferma",
    confirm_message: "Sei sicuro di voler procedere?",
    confirm_cancel: "Annulla",
    confirm_ok: "Conferma",
    status_loading: "Caricamento...",
    status_modified: "Impostazioni modificate (non salvate)",
    status_saving: "Salvataggio...",
    status_saved: "Impostazioni salvate con successo!",
    status_error: "Errore: ",
    model_status_ready: "Installato",
    model_action_download: "Scarica",
    model_action_delete: "Elimina",
    api_get_key_pattern: "Ottieni la chiave su {name}",
    status_loaded: "Impostazioni caricate",
    status_load_error: "Impossibile caricare le impostazioni: ",
    status_save_error: "Impossibile salvare le impostazioni: ",
    model_downloading_pattern: "Avvio del download per il modello '{model}'...",
    model_download_error_pattern: "Errore di download: {err}",
    delete_model_title: "Elimina modello",
    delete_model_confirm_pattern: "Sei sicuro di voler eliminare il modello locale '{model}'?",
    delete_model_btn: "Elimina",
    model_deleting_pattern: "Eliminazione del modello '{model}'...",
    model_deleted_success: "Modello eliminato con successo",
    model_delete_error_pattern: "Errore di eliminazione: {err}",
    model_downloaded_success_pattern: "Modello '{model}' scaricato!",
    confirm_clear_history_title: "Cancella cronologia",
    confirm_clear_history_msg: "Sei sicuro di voler cancellare tutta la cronologia delle trascrizioni?",
    general_ui_lang_title: "Lingua dell'interfaccia",
    general_ui_lang_desc: "Seleziona la lingua per le impostazioni e le notifiche dell'applicazione.",
    update_checks_title: "Controllo aggiornamenti",
    update_checks_desc: "Aura contatta GitHub solo durante un controllo manuale o se abiliti i controlli automatici.",
    update_checks_checkbox: "Controlla automaticamente gli aggiornamenti all’avvio",
    update_check_now: "Controlla aggiornamenti",
    cloud_data_desc: "Il provider cloud selezionato riceve l’audio e la trascrizione e, quando le relative funzioni sono abilitate, il testo selezionato e il dizionario personalizzato. La modalità locale non invia questi dati.",
    update_current: "Aura è aggiornata.",
    update_available_pattern: "È disponibile Aura v{version}.",
    update_check_error_pattern: "Impossibile verificare gli aggiornamenti: {error}",
    update_installing: "Download, verifica della firma e installazione dell’aggiornamento...",
    update_installed_restarting: "Aggiornamento installato. Riavvio...",
    update_install_error_open_release: "Impossibile installare l’aggiornamento. Apertura della pagina della versione...",
    gpu_accel_label: "Accelerazione hardware locale",
    gpu_accel_cpu_title: "CPU (senza accelerazione)",
    gpu_accel_cpu_desc: "Modalità standard. Affidabile, ma utilizza maggiormente la CPU.",
    gpu_accel_cuda_title: "NVIDIA CUDA (velocità massima)",
    gpu_accel_cuda_desc: "Per GPU GeForce RTX/GTX. Usa i Tensor Core.",
    gpu_accel_dml_title: "DirectML (universale)",
    gpu_accel_dml_desc: "Per GPU AMD, Intel e NVIDIA. Accelerazione di base.",
    hotkey_reset_title: "Ripristina ad Alt+V",
    local_engine_label: "Motore di riconoscimento",
    local_engine_whisper: "Whisper.cpp (OpenAI Whisper)",
    local_engine_parakeet: "NVIDIA Parakeet (via sherpa-onnx)",
    parakeet_model_label: "Modello Parakeet",
    model_meta_parakeet: "~670 MB — ottimizzato da NVIDIA",
    model_cancel_download: "Annulla download",
    model_download_cancelled: "Download annullato",
    update_available: "Aggiornamento disponibile",
    fallback_title: "Passaggio automatico quando il cloud non è disponibile",
    fallback_desc: "Se l'IA cloud non è disponibile (VPN, blocco regionale, nessuna rete), utilizza automaticamente il modello locale già scaricato per questa registrazione.",
    fallback_checkbox: "Attiva il fallback automatico al modello locale",
    copy_context_title: "Modifica testo selezionato",
    copy_context_desc: "Quando è abilitata, Aura invia Ctrl+C e passa il testo selezionato al provider cloud scelto come contesto per un comando di modifica. Disattiva questa funzione quando lavori in un terminale.",
    copy_context_checkbox: "Consenti acquisizione della selezione e modifica nel cloud",
    btn_copy_diagnostics: "Copia rapporto diagnostico",
    toast_diagnostics_copied: "Rapporto diagnostico copiato negli appunti!",
    diag_speech_text_title: "Registra testo vocale (Modalità sviluppatore)",
    diag_title: "Diagnostica",
    diag_speech_text_desc: "Include il testo vocale trascritto esatto nei log di diagnostica. Disattivato di default per la privacy.",
    diag_speech_text_checkbox: "Includi testo vocale nei log"
  },
  zh: {
    title_settings: "设置",
    tab_general: "常规",
    tab_speech: "语音",
    tab_hotkeys: "快捷键",
    tab_apikeys: "云端",
    section_cloud_functions: "云端功能",
    section_engine: "识别引擎",
    section_recognition: "识别",
    section_input: "输入",
    section_dictionary: "词典",
    tab_history: "历史记录",
    tab_about: "关于我们",
    general_autostart_title: "自启动设置",
    general_autostart_desc: "在Windows启动时自动运行此应用程序。",
    general_autostart_checkbox: "系统启动时运行 Aura",
    engine_title: "处理类型",
    engine_desc: "选择高品质云端识别，或完全离线的本地语音识别。",
    engine_cloud: "云端智能 AI",
    engine_cloud_meta: "Gemini / OpenAI / Groq (需要 API 密钥)",
    engine_local: "本地 AI (离线)",
    engine_local_meta: "Whisper / Parakeet (100% 离线和私密)",
    lang_bias_title: "识别语言",
    lang_bias_desc: "强制设定特定的听写语言，或使用自动检测。",
    lang_bias_label: "选择语言",
    lang_opt_auto: "自动检测 (默认)",
    lang_opt_layout: "遵循当前键盘布局",
    streaming_title: "输入模式",
    streaming_desc: "选择转换后文本的录入方式。",
    streaming_checkbox: "实时流式文本录入 (实验性)",
    streaming_subdesc: "如果关闭: 只有松开按键后，文字才会一次性录入。",
    punct_title: "智能标点符号",
    punct_desc: "将语音指令(如“逗号”、“句号”)转换为对应的标点符号。",
    punct_checkbox: "开启语音标点转换处理",
    vocab_title: "自定义词典",
    vocab_desc: "以逗号分隔输入专用术语、人名或品牌，以便提高识别精度。",
    vocab_placeholder: "例如：Aura, commit, 仓库...",
    punct_model_label: "标点（用于英语）",
    punct_model_name: "CT-Transformer (zh-en, int8)",
    punct_model_meta: "约 62 MB — 语音标点",
    engine_health_whisper: "Whisper：内置引擎，按需启动",
    engine_health_parakeet_running: "Parakeet：服务器运行中（{provider}，端口 {port}）",
    engine_health_parakeet_stopped: "Parakeet：服务器未运行",
    local_model_title: "本地 Whisper 模型",
    local_model_desc: "配置本地语音识别引擎，确保数据完全私密。",
    local_model_label: "模型大小",
    model_meta_tiny: "~75 MB — 超快速",
    model_meta_base: "~145 MB — 推荐",
    model_meta_small: "~465 MB — 精准",
    model_meta_medium: "~1.5 GB — 高级",
    model_meta_turbo: "~1.6 GB — RU/EN 最佳精度",
    model_meta_turbo_q5: "~550 MB — 接近 Turbo，体积减半",
    hotkey_title: "全局快捷键",
    hotkey_desc: "按住选择的组合键开始录音，松开即可完成转文字并录入。",
    hotkey_label: "组合按键",
    hotkey_toggle_mode: "触发模式 (短按切换)",
    hotkey_toggle_mode_desc: "短按启动/停止录音，无需一直按住按键。",
    sound_title: "声音反馈",
    sound_desc: "录音状态切换时播放提示音。",
    sound_enable: "启用悬浮条声音反馈",
    sound_volume_label: "音量",
    sound_theme_label: "声音主题",
    sound_theme_zen: "禅宗 (颂钵音)",
    sound_theme_rhodes: "Rhodes (爵士电钢琴)",
    sound_theme_scifi: "科幻 (太空合成器)",
    sound_theme_classic: "铃声 (经典八音盒)",
    api_title: "API 密钥授权",
    api_desc: "输入您在 Gemini、OpenAI 或 Groq 云端服务的 API 密钥。",
    api_provider: "API 供应商",
    api_key: "API 密钥",
    api_key_placeholder: "在此输入您的 API 密钥...",
    hotkey_prompt: "按键...",
    key_saved_placeholder: "•••••••• (已安全保存)",
    key_placeholder: "输入 API 密钥",
    api_get_key: "获取 API 密钥",
    history_title: "听写历史记录",
    history_clear: "清空历史",
    history_desc: "您最近转换出的文字将缓存在本地。",
    history_empty: "历史记录为空。您听写的文字会显示在这里。",
    history_badge_cloud: "云端",
    history_badge_local: "本地",
    history_engine_whisper: "Whisper",
    history_engine_parakeet: "NVIDIA Parakeet",
    history_unit_ms: "毫秒",
    history_unit_sec: "秒",
    about_app_title: "Aura 智能语音输入",
    about_version: "v1.0.9",
    about_description: "适用于 Windows 的全局语音输入工具。本程序可以将语音转录为文本，并以自动格式和标点符号插入到任何活动窗口中。",
    status_ready: "就绪",
    btn_save: "保存设置",
    confirm_title: "确认",
    confirm_message: "您确定要执行此操作吗？",
    confirm_cancel: "取消",
    confirm_ok: "确认",
    status_loading: "正在加载设置...",
    status_modified: "设置已更改 (未保存)",
    status_saving: "正在保存设置...",
    status_saved: "设置保存成功！",
    status_error: "发生错误: ",
    model_status_ready: "已安装",
    model_action_download: "下载",
    model_action_delete: "删除",
    api_get_key_pattern: "在 {name} 获取密钥",
    status_loaded: "设置已加载",
    status_load_error: "加载设置失败: ",
    status_save_error: "保存设置失败: ",
    model_downloading_pattern: "正在启动模型 '{model}' 的下载...",
    model_download_error_pattern: "下载错误: {err}",
    delete_model_title: "删除模型",
    delete_model_confirm_pattern: "您确定要删除本地模型 '{model}' 吗？",
    delete_model_btn: "删除",
    model_deleting_pattern: "正在删除模型 '{model}'...",
    model_deleted_success: "模型删除成功",
    model_delete_error_pattern: "删除错误: {err}",
    model_downloaded_success_pattern: "模型 '{model}' 已下载！",
    confirm_clear_history_title: "清空历史",
    confirm_clear_history_msg: "您确定要清空所有听写历史记录吗？",
    general_ui_lang_title: "界面语言",
    general_ui_lang_desc: "选择设置和应用程序通知的语言。",
    update_checks_title: "更新检查",
    update_checks_desc: "Aura 仅在您手动检查或启用自动检查时连接 GitHub。",
    update_checks_checkbox: "启动时自动检查更新",
    update_check_now: "检查更新",
    cloud_data_desc: "所选云服务提供商会接收音频和转写文本；启用相关功能时，还会接收选中文本和自定义词典。本地模式不会发送这些数据。",
    update_current: "Aura 已是最新版本。",
    update_available_pattern: "Aura v{version} 可用。",
    update_check_error_pattern: "无法检查更新：{error}",
    update_installing: "正在下载、验证签名并安装更新...",
    update_installed_restarting: "更新已安装。正在重启...",
    update_install_error_open_release: "无法安装更新。正在打开发布页面...",
    gpu_accel_label: "本地硬件加速",
    gpu_accel_cpu_title: "CPU（无加速）",
    gpu_accel_cpu_desc: "标准模式。稳定可靠，但会占用更多处理器资源。",
    gpu_accel_cuda_title: "NVIDIA CUDA（最高速度）",
    gpu_accel_cuda_desc: "适用于 GeForce RTX/GTX 显卡。使用 Tensor Core。",
    gpu_accel_dml_title: "DirectML（通用）",
    gpu_accel_dml_desc: "适用于 AMD、Intel 和 NVIDIA 显卡。基础加速。",
    hotkey_reset_title: "重置为 Alt+V",
    local_engine_label: "识别引擎",
    local_engine_whisper: "Whisper.cpp (OpenAI Whisper)",
    local_engine_parakeet: "NVIDIA Parakeet (通过 sherpa-onnx)",
    parakeet_model_label: "Parakeet 模型",
    model_meta_parakeet: "~670 MB — NVIDIA 优化",
    model_cancel_download: "取消下载",
    model_download_cancelled: "下载已取消",
    update_available: "有可用更新",
    fallback_title: "云端不可用时自动切换",
    fallback_desc: "当云端 AI 不可用时（VPN、地区限制、无网络），自动使用已下载的本地模型进行本次录音识别。",
    fallback_checkbox: "启用自动回退至本地模型",
    copy_context_title: "编辑选中文本",
    copy_context_desc: "启用后，Aura 会发送 Ctrl+C，并将选中文本作为编辑指令的上下文传给所选云服务提供商。在终端中工作时请关闭此功能。",
    copy_context_checkbox: "允许捕获选区并在云端编辑",
    btn_copy_diagnostics: "复制诊断报告",
    toast_diagnostics_copied: "诊断报告已复制到剪贴板！",
    diag_speech_text_title: "记录语音文本（开发者模式）",
    diag_title: "诊断",
    diag_speech_text_desc: "在诊断日志中包含精确的语音转写文本。出于隐私原因默认禁用。",
    diag_speech_text_checkbox: "在日志中包含语音文本"
  },
  pt: {
    title_settings: "Configurações",
    tab_general: "Geral",
    tab_speech: "Voz",
    tab_hotkeys: "Teclas de atalho",
    tab_apikeys: "Cloud",
    section_cloud_functions: "Recursos em nuvem",
    section_engine: "Motor de reconhecimento",
    section_recognition: "Reconhecimento",
    section_input: "Entrada",
    section_dictionary: "Dicionário",
    tab_history: "Histórico",
    tab_about: "Sobre",
    general_autostart_title: "Inicialização",
    general_autostart_desc: "Iniciar o aplicativo automaticamente com o Windows.",
    general_autostart_checkbox: "Iniciar o Aura com o Windows",
    engine_title: "Tipo de processamento",
    engine_desc: "Escolha entre processamento na nuvem de alta qualidade ou reconhecimento de voz local 100% offline.",
    engine_cloud: "IA na Nuvem",
    engine_cloud_meta: "Gemini / OpenAI / Groq (chave API necessária)",
    engine_local: "IA Local",
    engine_local_meta: "Whisper / Parakeet (100% offline e privado)",
    lang_bias_title: "Idioma do Diktat",
    lang_bias_desc: "Forçar um idioma específico para a transcrição ou usar detecção automática.",
    lang_bias_label: "Selecionar idioma",
    lang_opt_auto: "Auto-detectar (padrão)",
    lang_opt_layout: "Seguir o teclado ativo",
    streaming_title: "Fluxo de texto",
    streaming_desc: "Escolha o método para exibir o texto transcrito.",
    streaming_checkbox: "Escrita em tempo real (experimental)",
    streaming_subdesc: "Se desativado: o texto é colado inteiro apenas ao soltar o atalho.",
    punct_title: "Pontuação inteligente",
    punct_desc: "Converter comandos de voz (ex. \"vírgula\", \"ponto\") em pontuação correspondente.",
    punct_checkbox: "Habilitar processamento de pontuação por voz",
    vocab_title: "Dicionário personalizado",
    vocab_desc: "Adicione termos específicos, nomes ou siglas separados por vírgula para melhorar o reconhecimento.",
    vocab_placeholder: "ex. Aura, commit, repositório...",
    punct_model_label: "Pontuação (para inglês)",
    punct_model_name: "CT-Transformer (zh-en, int8)",
    punct_model_meta: "~62 MB — pontuação por voz",
    engine_health_whisper: "Whisper: mecanismo integrado, iniciado sob demanda",
    engine_health_parakeet_running: "Parakeet: servidor em execução ({provider}, porta {port})",
    engine_health_parakeet_stopped: "Parakeet: servidor não em execução",
    local_model_title: "Modelo Whisper local",
    local_model_desc: "Configure um mecanismo local de reconhecimento de voz para manter total privacidade.",
    local_model_label: "Tamanho do modelo",
    model_meta_tiny: "~75 MB — super-rápido",
    model_meta_base: "~145 MB — recomendado",
    model_meta_small: "~465 MB — preciso",
    model_meta_medium: "~1.5 GB — avançado",
    model_meta_turbo: "~1.6 GB — melhor precisão RU/EN",
    model_meta_turbo_q5: "~550 MB — quase como Turbo, metade do tamanho",
    hotkey_title: "Teclas globais",
    hotkey_desc: "Segure o atalho para gravar, solte para transcrever.",
    hotkey_label: "Combinação",
    hotkey_toggle_mode: "Modo alternar (toque rápido)",
    hotkey_toggle_mode_desc: "Um toque rápido inicia/para a gravação sem segurar o botão.",
    sound_title: "Feedback sonoro",
    sound_desc: "Efeitos sonoros do overlay ao gravar.",
    sound_enable: "Habilitar sons do overlay",
    sound_volume_label: "Volume do som",
    sound_theme_label: "Tema sonoro",
    sound_theme_zen: "Zen (Tigelas cantantes)",
    sound_theme_rhodes: "Rhodes (Piano elétrico)",
    sound_theme_scifi: "Sci-Fi (Sintetizador espacial)",
    sound_theme_classic: "Sino (Clássico)",
    api_title: "Autorização de chaves API",
    api_desc: "Insira suas chaves API para os serviços Gemini, OpenAI ou Groq.",
    api_provider: "Provedor de API",
    api_key: "Chave API",
    api_key_placeholder: "Insira sua chave API...",
    hotkey_prompt: "Pressione as teclas...",
    key_saved_placeholder: "•••••••• (salvo com segurança)",
    key_placeholder: "Insira a chave da API",
    api_get_key: "Obter chave API",
    history_title: "Histórico de transcrição",
    history_clear: "Limpar histórico",
    history_desc: "As últimas frases ditadas são armazenadas localmente.",
    history_empty: "O histórico está vazio. Seus textos ditados aparecerão aqui.",
    history_badge_cloud: "Nuvem",
    history_badge_local: "Local",
    history_engine_whisper: "Whisper",
    history_engine_parakeet: "Parakeet NVIDIA",
    history_unit_ms: "ms",
    history_unit_sec: "s",
    about_app_title: "Ditado de voz Aura",
    about_version: "v1.0.9",
    about_description: "Ferramenta de entrada de voz global para Windows. O programa transcreve a fala em texto e a insere em qualquer janela ativa com formatação e pontuação automáticas.",
    status_ready: "Pronto",
    btn_save: "Salvar configurações",
    confirm_title: "Confirmação",
    confirm_message: "Tem certeza de que deseja executar esta ação?",
    confirm_cancel: "Cancelar",
    confirm_ok: "Confirmar",
    status_loading: "Carregando...",
    status_modified: "Configurações alteradas (não salvas)",
    status_saving: "Salvando...",
    status_saved: "Configurações salvas com sucesso!",
    status_error: "Erro: ",
    model_status_ready: "Instalado",
    model_action_download: "Baixar",
    model_action_delete: "Excluir",
    api_get_key_pattern: "Obter chave em {name}",
    status_loaded: "Configurações carregadas",
    status_load_error: "Falha ao carregar configurações: ",
    status_save_error: "Falha ao salvar configurações: ",
    model_downloading_pattern: "Iniciando download para o modelo '{model}'...",
    model_download_error_pattern: "Erro de download: {err}",
    delete_model_title: "Excluir modelo",
    delete_model_confirm_pattern: "Tem certeza de que deseja excluir o modelo local '{model}'?",
    delete_model_btn: "Excluir",
    model_deleting_pattern: "Excluindo modelo '{model}'...",
    model_deleted_success: "Modelo excluído com sucesso",
    model_delete_error_pattern: "Erro ao excluir: {err}",
    model_downloaded_success_pattern: "Modelo '{model}' baixado!",
    confirm_clear_history_title: "Limpar histórico",
    confirm_clear_history_msg: "Tem certeza de que deseja limpar todo o histórico de transcrições?",
    general_ui_lang_title: "Idioma da interface",
    general_ui_lang_desc: "Selecione o idioma para as configurações e notificações do aplicativo.",
    update_checks_title: "Verificação de atualizações",
    update_checks_desc: "A Aura só acessa o GitHub quando você verifica manualmente ou ativa as verificações automáticas.",
    update_checks_checkbox: "Verificar atualizações automaticamente ao iniciar",
    update_check_now: "Verificar atualizações",
    cloud_data_desc: "O provedor de nuvem selecionado recebe o áudio e a transcrição e, quando os recursos correspondentes estão ativados, o texto selecionado e o dicionário personalizado. O modo local não envia esses dados.",
    update_current: "A Aura está atualizada.",
    update_available_pattern: "A versão {version} da Aura está disponível.",
    update_check_error_pattern: "Não foi possível verificar atualizações: {error}",
    update_installing: "Baixando, verificando a assinatura e instalando a atualização...",
    update_installed_restarting: "Atualização instalada. Reiniciando...",
    update_install_error_open_release: "Não foi possível instalar a atualização. Abrindo a página da versão...",
    gpu_accel_label: "Aceleração de hardware local",
    gpu_accel_cpu_title: "CPU (sem aceleração)",
    gpu_accel_cpu_desc: "Modo padrão. Seguro, mas exige mais do processador.",
    gpu_accel_cuda_title: "NVIDIA CUDA (velocidade máxima)",
    gpu_accel_cuda_desc: "Para GPUs GeForce RTX/GTX. Usa Tensor Cores.",
    gpu_accel_dml_title: "DirectML (universal)",
    gpu_accel_dml_desc: "Para GPUs AMD, Intel e NVIDIA. Aceleração básica.",
    hotkey_reset_title: "Redefinir para Alt+V",
    local_engine_label: "Motor de reconhecimento",
    local_engine_whisper: "Whisper.cpp (OpenAI Whisper)",
    local_engine_parakeet: "NVIDIA Parakeet (via sherpa-onnx)",
    parakeet_model_label: "Modelo Parakeet",
    model_meta_parakeet: "~670 MB — otimizado pela NVIDIA",
    model_cancel_download: "Cancelar download",
    model_download_cancelled: "Download cancelado",
    update_available: "Atualização disponível",
    fallback_title: "Alternância automática quando a nuvem estiver indisponível",
    fallback_desc: "Se a IA na nuvem estiver indisponível (VPN, bloqueio regional, sem rede), usar automaticamente o modelo local já baixado para esta gravação.",
    fallback_checkbox: "Ativar fallback automático para modelo local",
    copy_context_title: "Editar texto selecionado",
    copy_context_desc: "Quando ativado, o Aura envia Ctrl+C e encaminha o texto selecionado ao provedor de nuvem escolhido como contexto para um comando de edição. Desative este recurso ao trabalhar em um terminal.",
    copy_context_checkbox: "Permitir captura da seleção e edição na nuvem",
    btn_copy_diagnostics: "Copiar relatório de diagnóstico",
    toast_diagnostics_copied: "Relatório de diagnóstico copiado para a área de transferência!",
    diag_speech_text_title: "Registrar texto de voz (Modo desenvolvedor)",
    diag_title: "Diagnóstico",
    diag_speech_text_desc: "Incluir texto de voz transcrito exato nos logs de diagnóstico. Desativado por padrão por privacidade.",
    diag_speech_text_checkbox: "Incluir texto de voz nos logs"
  },
  tr: {
    title_settings: "Ayarlar",
    tab_general: "Genel",
    tab_speech: "Ses",
    tab_hotkeys: "Kısayollar",
    tab_apikeys: "Bulut",
    section_cloud_functions: "Bulut özellikleri",
    section_engine: "Tanıma motoru",
    section_recognition: "Tanıma",
    section_input: "Giriş",
    section_dictionary: "Sözlük",
    tab_history: "Geçmiş",
    tab_about: "Hakkında",
    general_autostart_title: "Başlangıçta Çalıştır",
    general_autostart_desc: "Windows açıldığında uygulamayı otomatik olarak başlat.",
    general_autostart_checkbox: "Aura'yı sistem açılışında başlat",
    engine_title: "İşlem Türü",
    engine_desc: "Yüksek kaliteli bulut işleme veya tamamen çevrimdışı yerel konuşma tanıma arasında seçim yapın.",
    engine_cloud: "Bulut Yapay Zekası",
    engine_cloud_meta: "Gemini / OpenAI / Groq (API anahtarı gerekli)",
    engine_local: "Yerel Yapay Zeka",
    engine_local_meta: "Whisper / Parakeet (100% çevrimdışı ve gizli)",
    lang_bias_title: "Yazım Dili",
    lang_bias_desc: "Transkripsiyon için belirli bir dili zorlayın veya otomatik algılamayı kullanın.",
    lang_bias_label: "Dil Seçin",
    lang_opt_auto: "Otomatik Algıla (varsayılan)",
    lang_opt_layout: "Klavye Düzenine Göre",
    streaming_title: "Yazım Modu",
    streaming_desc: "Transkripsiyonu ekleme yöntemini seçin.",
    streaming_checkbox: "Gerçek zamanlı akışlı metin girişi (deneysel)",
    streaming_subdesc: "Kapatılırsa: metin sadece tuşu bıraktığınızda bir bütün olarak eklenir.",
    punct_title: "Akıllı Noktalama",
    punct_desc: "Konuşulan noktalama komutlarını (\"virgül\", \"nokta\") simgelere dönüştür.",
    punct_checkbox: "Sesli noktalama işaretlerini işlemeyi etkinleştir",
    vocab_title: "Özel Sözlük",
    vocab_desc: "Algılama kalitesini artırmak için özel terimleri, isimleri virgülle ayırarak girin.",
    vocab_placeholder: "örn. Aura, commit, depo...",
    punct_model_label: "Noktalama (İngilizce için)",
    punct_model_name: "CT-Transformer (zh-en, int8)",
    punct_model_meta: "~62 MB — sesli noktalama",
    engine_health_whisper: "Whisper: yerleşik motor, ihtiyaç halinde başlatılır",
    engine_health_parakeet_running: "Parakeet: sunucu çalışıyor ({provider}, port {port})",
    engine_health_parakeet_stopped: "Parakeet: sunucu çalışmıyor",
    local_model_title: "Yerel Whisper Modülü",
    local_model_desc: "Tam gizlilik için yerel bir konuşma tanıma motoru yapılandırın.",
    local_model_label: "Model Boyutu",
    model_meta_tiny: "~75 MB — süper hızlı",
    model_meta_base: "~145 MB — önerilen",
    model_meta_small: "~465 MB — hassas",
    model_meta_medium: "~1.5 GB — gelişmiş",
    model_meta_turbo: "~1.6 GB — RU/EN için en iyi doğruluk",
    model_meta_turbo_q5: "~550 MB — Turbo'ya yakın, yarı boyut",
    hotkey_title: "Global Kısayol",
    hotkey_desc: "Kayda başlamak için seçilen kombinasyonu basılı tutun, transkripsiyon için bırakın.",
    hotkey_label: "Kombinasyon",
    hotkey_toggle_mode: "Geçiş modu (kısa basma)",
    hotkey_toggle_mode_desc: "Kısa bir basış, basılı tutmadan kaydı başlatır veya durdurur.",
    sound_title: "Ses Geri Bildirimi",
    sound_desc: "Kayıt durumları değiştiğinde çalınacak ses efektleri.",
    sound_enable: "Overlay seslerini etkinleştir",
    sound_volume_label: "Ses Seviyesi",
    sound_theme_label: "Ses Teması",
    sound_theme_zen: "Zen (Nepal Çanakları)",
    sound_theme_rhodes: "Rhodes (Caz Elektro Piyano)",
    sound_theme_scifi: "Sci-Fi (Uzay Sentezleyici)",
    sound_theme_classic: "Zil (Klasik)",
    api_title: "API Anahtarları Yetkilendirme",
    api_desc: "Gemini, OpenAI veya Groq bulut hizmetleri için API anahtarlarınızı girin.",
    api_provider: "API Sağlayıcısı",
    api_key: "API Anahtarı",
    api_key_placeholder: "API anahtarınızı buraya girin...",
    hotkey_prompt: "Tuşlara basın...",
    key_saved_placeholder: "•••••••• (güvenle kaydedildi)",
    key_placeholder: "API anahtarını girin",
    api_get_key: "API Anahtarı Al",
    history_title: "Yazım Geçmişi",
    history_clear: "Geçmişi Temizle",
    history_desc: "Son sesli yazımlarınız yerel olarak saklanır.",
    history_empty: "Geçmiş boş. Yazdığınız metinler burada görünecektir.",
    history_badge_cloud: "Bulut",
    history_badge_local: "Yerel",
    history_engine_whisper: "Whisper",
    history_engine_parakeet: "NVIDIA Parakeet",
    history_unit_ms: "ms",
    history_unit_sec: "sn",
    about_app_title: "Aura Sesli Giriş",
    about_version: "v1.0.9",
    about_description: "Windows için genel sesli giriş aracı. Program, konuşmayı metne dönüştürür ve otomatik biçimlendirme ve noktalama işaretleriyle herhangi bir aktif pencereye ekler.",
    status_ready: "Hazır",
    btn_save: "Ayarları Kaydet",
    confirm_title: "Onay",
    confirm_message: "Bu işlemi gerçekleştirmek istediğinizden emin misiniz?",
    confirm_cancel: "İptal",
    confirm_ok: "Onayla",
    status_loading: "Ayarlar yükleniyor...",
    status_modified: "Ayarlar değiştirildi (kaydedilmedi)",
    status_saving: "Ayarlar kaydediliyor...",
    status_saved: "Ayarlar başarıyla kaydedildi!",
    status_error: "Hata: ",
    model_status_ready: "Yüklendi",
    model_action_download: "İndir",
    model_action_delete: "Sil",
    api_get_key_pattern: "{name} üzerinden anahtar al",
    status_loaded: "Ayarlar yüklendi",
    status_load_error: "Ayarlar yüklenemedi: ",
    status_save_error: "Ayarlar kaydedilemedi: ",
    model_downloading_pattern: "'{model}' modeli için indirme başlatılıyor...",
    model_download_error_pattern: "İndirme hatası: {err}",
    delete_model_title: "Modeli sil",
    delete_model_confirm_pattern: "Yerel '{model}' modelini silmek istediğinizden emin misiniz?",
    delete_model_btn: "Sil",
    model_deleting_pattern: "'{model}' modeli siliniyor...",
    model_deleted_success: "Model başarıyla silindi",
    model_delete_error_pattern: "Silme hatası: {err}",
    model_downloaded_success_pattern: "'{model}' modeli indirildi!",
    confirm_clear_history_title: "Geçmişi Temizle",
    confirm_clear_history_msg: "Tüm transkripsiyon geçmişini temizlemek istediğinizden emin misiniz?",
    general_ui_lang_title: "Arayüz Dili",
    general_ui_lang_desc: "Ayarlar ve uygulama bildirimleri için dili seçin.",
    update_checks_title: "Güncelleme denetimi",
    update_checks_desc: "Aura, GitHub’a yalnızca elle denetlediğinizde veya otomatik denetimleri etkinleştirdiğinizde bağlanır.",
    update_checks_checkbox: "Başlangıçta güncellemeleri otomatik olarak denetle",
    update_check_now: "Güncellemeleri denetle",
    cloud_data_desc: "Seçilen bulut sağlayıcısına ses ve transkript; ilgili özellikler etkinse seçili metin ve özel sözlük gönderilir. Yerel mod bu verileri göndermez.",
    update_current: "Aura güncel.",
    update_available_pattern: "Aura v{version} kullanılabilir.",
    update_check_error_pattern: "Güncellemeler denetlenemedi: {error}",
    update_installing: "Güncelleme indiriliyor, imza doğrulanıyor ve kuruluyor...",
    update_installed_restarting: "Güncelleme kuruldu. Yeniden başlatılıyor...",
    update_install_error_open_release: "Güncelleme kurulamadı. Sürüm sayfası açılıyor...",
    gpu_accel_label: "Yerel donanım hızlandırma",
    gpu_accel_cpu_title: "CPU (hızlandırma yok)",
    gpu_accel_cpu_desc: "Standart mod. Güvenlidir ancak işlemciyi daha fazla kullanır.",
    gpu_accel_cuda_title: "NVIDIA CUDA (en yüksek hız)",
    gpu_accel_cuda_desc: "GeForce RTX/GTX ekran kartları için. Tensor çekirdeklerini kullanır.",
    gpu_accel_dml_title: "DirectML (evrensel)",
    gpu_accel_dml_desc: "AMD, Intel ve NVIDIA ekran kartları için. Temel hızlandırma.",
    hotkey_reset_title: "Alt+V'ye Sıfırla",
    local_engine_label: "Tanıma Motoru",
    local_engine_whisper: "Whisper.cpp (OpenAI Whisper)",
    local_engine_parakeet: "NVIDIA Parakeet (sherpa-onnx ile)",
    parakeet_model_label: "Parakeet Modeli",
    model_meta_parakeet: "~670 MB — NVIDIA tarafından optimize edildi",
    model_cancel_download: "İndirmeyi iptal et",
    model_download_cancelled: "İndirme iptal edildi",
    update_available: "Güncelleme mevcut",
    fallback_title: "Bulut kullanılamadığında otomatik geçiş",
    fallback_desc: "Bulut yapay zekası kullanılamıyorsa (VPN, bölge engeli, ağ yok), bu kayıt için önceden indirilmiş yerel modeli otomatik olarak kullan.",
    fallback_checkbox: "Yerel modele otomatik geçişi etkinleştir",
    copy_context_title: "Seçili metni düzenle",
    copy_context_desc: "Etkinleştirildiğinde Aura, Ctrl+C gönderir ve seçili metni bir düzenleme komutu için bağlam olarak seçilen bulut sağlayıcısına iletir. Terminalde çalışırken bu özelliği devre dışı bırakın.",
    copy_context_checkbox: "Seçimi yakalamaya ve bulutta düzenlemeye izin ver",
    btn_copy_diagnostics: "Teşhis Raporunu Kopyala",
    toast_diagnostics_copied: "Teşhis raporu panoya kopyalandı!",
    diag_speech_text_title: "Konuşma Metnini Günlüğe Kaydet (Geliştirici Modu)",
    diag_title: "Teşhis",
    diag_speech_text_desc: "Teşhis günlüklerine tam transkribe edilmiş konuşma metnini dahil et. Gizlilik nedeniyle varsayılan olarak devre dışıdır.",
    diag_speech_text_checkbox: "Konuşma metnini günlüklere dahil et"
  }
};

let currentLanguage = "ru";
const SELECT_PREVIEW_TEXTS = {
  ru: {
    engine_whisper: "Локальный движок на базе OpenAI Whisper. Полностью офлайн, модель выбирается ниже.",
    engine_parakeet: "NVIDIA Parakeet — быстрый движок. Запускает собственный сервер на этом компьютере.",
    lang_auto: "Aura определит язык по речи автоматически. Подходит для большинства случаев.",
    lang_layout: "Язык выберется по активной раскладке клавиатуры в текущем окне.",
    lang_forced: "Распознавание будет вестись на языке «{lang}» независимо от произношения.",
    provider_gemini: "Gemini — высокая точность транскрипции и естественное редактирование текста.",
    provider_openai: "OpenAI Whisper для расшифровки и GPT для очистки и редактирования.",
    provider_groq: "Groq — быстрый Whisper на их серверах и Llama 3 для текста.",
    ui_lang: "Язык интерфейса: настройки и уведомления приложения.",
    theme_zen: "Поющие чаши — спокойный и мягкий звон.",
    theme_rhodes: "Тёплый тон джазового электропианино.",
    theme_scifi: "Синтезаторные сигналы с космическим настроением.",
    theme_classic: "Классический колокольчик."
  },
  en: {
    engine_whisper: "Local engine based on OpenAI Whisper. Fully offline; the model is selected below.",
    engine_parakeet: "NVIDIA Parakeet — a faster engine. Runs its own server on this machine.",
    lang_auto: "Aura detects the language from speech automatically. Good for most cases.",
    lang_layout: "The language follows the active keyboard layout of the current window.",
    lang_forced: "Recognition will be locked to «{lang}», no matter how you pronounce it.",
    provider_gemini: "Gemini — high transcription accuracy and natural text editing.",
    provider_openai: "OpenAI Whisper for transcription and GPT for cleaning and editing.",
    provider_groq: "Groq — fast Whisper on their servers and Llama 3 for text.",
    ui_lang: "UI language: app settings and notifications.",
    theme_zen: "Tibetan bowls — a calm, soft ring.",
    theme_rhodes: "Warm jazz electric-piano tone.",
    theme_scifi: "Synthesizer signals with a space mood.",
    theme_classic: "A classic doorbell chime."
  },
  de: {
    engine_whisper: "Lokale Engine auf Basis von OpenAI Whisper. Völlig offline; das Modell wird unten gewählt.",
    engine_parakeet: "NVIDIA Parakeet — schnellere Engine. Startet einen eigenen Server auf diesem Rechner.",
    lang_auto: "Aura erkennt die Sprache automatisch an der Stimme. Für die meisten Fälle geeignet.",
    lang_layout: "Die Sprache richtet sich nach dem aktiven Tastaturlayout des Fensters.",
    lang_forced: "Erkennung läuft fest auf «{lang}», unabhängig von der Aussprache.",
    provider_gemini: "Gemini — hohe Erkennungsgenauigkeit und natürliche Textbearbeitung.",
    provider_openai: "OpenAI Whisper für die Transkription und GPT für Bereinigung und Bearbeitung.",
    provider_groq: "Groq — schnelles Whisper auf ihren Servern und Llama 3 für Text.",
    ui_lang: "Oberflächensprache: Einstellungen und Benachrichtigungen.",
    theme_zen: "Klangschalen — ruhiger, sanfter Klang.",
    theme_rhodes: "Warmer Jazz-E-Piano-Sound.",
    theme_scifi: "Synthesizer-Signale mit Weltraumatmosphäre.",
    theme_classic: "Klassische Türklingel."
  },
  es: {
    engine_whisper: "Motor local basado en OpenAI Whisper. 100% sin conexión; el modelo se elige abajo.",
    engine_parakeet: "NVIDIA Parakeet — motor más rápido. Ejecuta su propio servidor en este equipo.",
    lang_auto: "Aura detecta el idioma desde la voz. Válido para la mayoría de los casos.",
    lang_layout: "El idioma seguirá la distribución de teclado activa de la ventana.",
    lang_forced: "El reconocimiento se fijará en «{lang}», sin importar la pronunciación.",
    provider_gemini: "Gemini — alta precisión de transcripción y edición natural del texto.",
    provider_openai: "OpenAI Whisper para transcribir y GPT para limpiar y editar.",
    provider_groq: "Groq — Whisper rápido en sus servidores y Llama 3 para texto.",
    ui_lang: "Idioma de la interfaz: configuración y notificaciones.",
    theme_zen: "Cuencos tibetanos: un sonido tranquilo y suave.",
    theme_rhodes: "Sonido cálido de piano eléctrico de Jazz.",
    theme_scifi: "Señales de sintetizador con ambiente espacial.",
    theme_classic: "Un timbre clásico de puerta."
  },
  fr: {
    engine_whisper: "Moteur local basé sur OpenAI Whisper. Entièrement hors ligne, le modèle se choisit ci-dessous.",
    engine_parakeet: "NVIDIA Parakeet — un moteur plus rapide. Lui-même un serveur sur cette machine.",
    lang_auto: "Aura détecte la langue de la parole. Convient à la plupart des situations.",
    lang_layout: "La langue suit la disposition de clavier active de la fenêtre.",
    lang_forced: "La reconnaissance restera fixée sur «{lang}», quelle que soit la prononciation.",
    provider_gemini: "Gemini — transcription précise et édition naturelle du texte.",
    provider_openai: "OpenAI Whisper pour la transcription et GPT pour le nettoyage et l’édition.",
    provider_groq: "Groq — Whisper rapide sur leurs serveurs et Llama 3 pour le texte.",
    ui_lang: "Langue de l’interface : paramètres et notifications.",
    theme_zen: "Bol chantant — un son calme et doux.",
    theme_rhodes: "Timbre chaud de piano électrique de jazz.",
    theme_scifi: "Signaux de synthétiseur à l’ambiance spatiale.",
    theme_classic: "Un carillon classique de porte."
  },
  it: {
    engine_whisper: "Motore locale basato su OpenAI Whisper. Totalmente offline, il modello si sceglie qui sotto.",
    engine_parakeet: "NVIDIA Parakeet — motore più veloce. Avvia un proprio server su questo computer.",
    lang_auto: "Aura rileva la lingua dalla parole. Adatto alla maggior parte dei casi.",
    lang_layout: "La lingua segue la disposizione tastiera attiva della finestra.",
    lang_forced: "Il riconoscimento sarà fissato su «{lang}», qualsiasi sia la pronuncia.",
    provider_gemini: "Gemini — alta precisione di trascrizione ed editing naturale del testo.",
    provider_openai: "OpenAI Whisper per la trascrizione e GPT per pulizia ed editing.",
    provider_groq: "Groq — Whisper veloce sui loro server e Llama 3 per il testo.",
    ui_lang: "Lingua interfaccia: impostazioni e notifiche.",
    theme_zen: "Ciottoli tibetani: un suono calmo e morbido.",
    theme_rhodes: "Tono caldo del piano elettrico jazz.",
    theme_scifi: "Segnali di synth con atmosfera spaziale.",
    theme_classic: "Un classico campanello da portone."
  },
  zh: {
    engine_whisper: "基于 OpenAI Whisper 的本地引擎。完全离线，模型在下方选择。",
    engine_parakeet: "NVIDIA Parakeet — 更快的引擎。在本机运行自己的服务器。",
    lang_auto: "Aura 会根据语音自动识别语言，适用于大多数情况。",
    lang_layout: "语言将跟随当前窗口的键盘布局。",
    lang_forced: "识别将固定在「{lang}」，与发音无关。",
    provider_gemini: "Gemini — 高精度转录和自然文本编辑。",
    provider_openai: "OpenAI Whisper 负责转录，GPT 负责清理和编辑。",
    provider_groq: "Groq — 其服务器上的快速 Whisper 以及用于文本的 Llama 3。",
    ui_lang: "界面语言：应用设置和通知。",
    theme_zen: "颂钵 — 平静柔和的声音。",
    theme_rhodes: "温暖的爵士电钢琴音色。",
    theme_scifi: "带有太空氛围的合成器信号。",
    theme_classic: "经典门铃铃声。"
  },
  pt: {
    engine_whisper: "Motor local baseado em OpenAI Whisper. 100% offline, o modelo escolhido abaixo.",
    engine_parakeet: "NVIDIA Parakeet — motor mais rápido. Roda o próprio servidor neste computador.",
    lang_auto: "Aura detecta o idioma pela voz. Bom para a maioria dos casos.",
    lang_layout: "O idioma segue a disposição de teclado ativa da janela.",
    lang_forced: "O reconhecimento será fixado em «{lang}», não importa como você fala.",
    provider_gemini: "Gemini — alta precisão de transcrição e edição natural.",
    provider_openai: "OpenAI Whisper para transcrição e GPT para limpeza e edição.",
    provider_groq: "Groq — Whisper rápido nos servidores ece Llama 3 para texto.",
    ui_lang: "Idioma da interface: definições e notificações.",
    theme_zen: "Tigelas tibetanas — som calmo e suave.",
    theme_rhodes: "Timbre quente de piano elétrico de jazz.",
    theme_scifi: "Sinais de sintetizador com atmosfera espacial.",
    theme_classic: "Um sino de porta clássico."
  },
  tr: {
    engine_whisper: "OpenAI Whisper tabanlı yerel motor. Tamamen çevrimdışı, model aşağıdan seçilir.",
    engine_parakeet: "NVIDIA Parakeet — daha hızlı motor. Bu bilgisayarda kendi sunucusunu çalıştırır.",
    lang_auto: "Aura dili konuşmadan otomatik algılar. Çoğu durum için uygundur.",
    lang_forced: "Tanıma, nasıl konuştuğunuzdan bağımsız olarak «{lang}» diline sabitlenir.",
    provider_gemini: "Gemini — yüksek doğruluklu transkripsiyon ve doğal metin düzenleme.",
    provider_openai: "Transkripsiyon için OpenAI Whisper, temizlik ve düzenleme için GPT.",
    provider_groq: "Groq — sunucularında hızlı Whisper ve metin için Llama 3.",
    ui_lang: "Arayüz dili: uygulama ayarları ve bildirimler.",
    theme_zen: "Tibet çanları — sakin ve yumuşak bir ses.",
    theme_rhodes: "Sıcak caz elektrikli piyano tınısı.",
    theme_scifi: "Uzay havası taşıyan synth sinyalleri.",
    theme_classic: "Klasik kapı zili sesi."
  }
};

const SELECT_PREVIEW_DEFS = {
  "select-local-engine": {
    byValue: { whisper: "engine_whisper", parakeet: "engine_parakeet" }
  },
  "select-ui-lang": {
    defaultKey: "ui_lang"
  },
  "select-language": {
    byValue: { auto: "lang_auto", layout: "lang_layout" },
    fallbackKey: "lang_forced",
    fallbackParam: () => ({ lang: selectedOptionLabel(document.getElementById("select-language")) })
  },
  "select-provider": {
    byValue: { gemini: "provider_gemini", openai: "provider_openai", groq: "provider_groq" }
  },
  "select-sound-theme": {
    byValue: { zen: "theme_zen", rhodes: "theme_rhodes", scifi: "theme_scifi", classic: "theme_classic" }
  }
};

function selectedOptionLabel(select) {
  if (!select) return "";
  return select.selectedOptions.length ? select.selectedOptions[0].textContent.trim() : "";
}

function updateSelectPreview(select) {
  const def = SELECT_PREVIEW_DEFS[select.id];
  const textEl = document.getElementById(select.dataset.preview);
  if (!def || !textEl) return;
  const dict = SELECT_PREVIEW_TEXTS[currentLanguage] || SELECT_PREVIEW_TEXTS.ru;
  let key;
  if (def.byValue && def.byValue[select.value] !== undefined) key = def.byValue[select.value];
  else if (def.defaultKey) key = def.defaultKey;
  else if (def.fallbackKey) key = def.fallbackKey;
  if (!key) return;
  let text = dict[key] ?? SELECT_PREVIEW_TEXTS.ru[key] ?? "";
  const params = def.fallbackParam ? def.fallbackParam() : {};
  for (const [name, value] of Object.entries(params)) {
    text = text.split(`{${name}}`).join(value);
  }
  textEl.textContent = text;
}

function updateAllSelectPreviews() {
  document.querySelectorAll("select.custom-select[data-preview]").forEach(updateSelectPreview);
}

function selectPanelCaption(select, value) {
  const def = SELECT_PREVIEW_DEFS[select.id];
  if (!def || !def.byValue || def.byValue[value] === undefined) return "";
  const dict = SELECT_PREVIEW_TEXTS[currentLanguage] || SELECT_PREVIEW_TEXTS.ru;
  const key = def.byValue[value];
  return dict[key] ?? SELECT_PREVIEW_TEXTS.ru[key] ?? "";
}

function syncPanelSelection(select) {
  const wrap = select.closest(".select-wrap");
  const panel = wrap && wrap.querySelector(".select-panel");
  if (!panel) return;
  const value = select.value;
  panel.querySelectorAll(".select-panel-item").forEach((item) => {
    item.classList.toggle("is-selected", item.dataset.value === value);
  });
}

function buildSelectPanel(select) {
  const wrap = select.closest(".select-wrap");
  const panel = wrap && wrap.querySelector(".select-panel");
  if (!panel) return;
  panel.textContent = "";
  for (const option of select.options) {
    if (option.disabled) continue;
    const item = document.createElement("div");
    item.className = "select-panel-item";
    item.dataset.value = option.value;
    item.setAttribute("role", "option");

    const main = document.createElement("div");
    main.className = "select-panel-item-main";

    const name = document.createElement("span");
    name.className = "select-panel-item-name";
    name.textContent = option.textContent.trim();
    main.appendChild(name);

    const caption = selectPanelCaption(select, option.value);
    if (caption) {
      const desc = document.createElement("span");
      desc.className = "select-panel-item-desc";
      desc.textContent = caption;
      main.appendChild(desc);
    }

    const check = document.createElement("span");
    check.className = "select-panel-item-check";
    check.setAttribute("aria-hidden", "true");
    check.innerHTML =
      '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>';

    item.appendChild(main);
    item.appendChild(check);
    item.addEventListener("click", () => {
      pickSelectValue(select, option.value);
    });
    panel.appendChild(item);
  }
  syncPanelSelection(select);
}

function rebuildSelectPanels() {
  document.querySelectorAll("select.custom-select[data-preview]").forEach(buildSelectPanel);
}

function closeSelectPanels() {
  document.querySelectorAll(".select-panel.open").forEach((panel) => {
    panel.classList.remove("open");
  });
  document.querySelectorAll("select.custom-select[aria-expanded]").forEach((el) => {
    el.setAttribute("aria-expanded", "false");
  });
}

function toggleSelectPanel(select) {
  const wrap = select.closest(".select-wrap");
  const panel = wrap && wrap.querySelector(".select-panel");
  if (!panel) return;
  const willOpen = !panel.classList.contains("open");
  closeSelectPanels();
  if (willOpen) {
    panel.classList.add("open");
    select.setAttribute("aria-expanded", "true");
  }
}

function pickSelectValue(select, value) {
  if (select.value === value) {
    closeSelectPanels();
    return;
  }
  select.value = value;
  select.dispatchEvent(new Event("change", { bubbles: true }));
  syncPanelSelection(select);
  closeSelectPanels();
}

function movePanelFocus(select, direction) {
  const panel = select.closest(".select-wrap").querySelector(".select-panel");
  if (!panel) return;
  const items = Array.from(panel.querySelectorAll(".select-panel-item"));
  const currentIndex = items.findIndex((item) => item.classList.contains("is-focused"));
  let next = currentIndex === -1 ? 0 : currentIndex + direction;
  next = (next + items.length) % items.length;
  items.forEach((item) => {
    item.classList.toggle("is-focused", item === items[next]);
    if (item === items[next] && typeof item.scrollIntoView === "function") {
      item.scrollIntoView({ block: "nearest" });
    }
  });
  return items[next];
}

function handleSelectMousedown(event) {
  if (event.button !== 0) return;
  event.preventDefault();
  toggleSelectPanel(event.currentTarget);
}

function handleSelectKeydown(event) {
  const select = event.currentTarget;
  const panel = select.closest(".select-wrap")?.querySelector(".select-panel");
  const isOpen = panel && panel.classList.contains("open");
  const KEY_OPEN = ["Enter", " ", "ArrowDown", "ArrowUp"];
  if (!isOpen) {
    if (KEY_OPEN.includes(event.key)) {
      event.preventDefault();
      toggleSelectPanel(select);
      movePanelFocus(select, 1);
    }
    return;
  }
  if (event.key === "Escape") {
    event.preventDefault();
    closeSelectPanels();
    select.focus();
    return;
  }
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    movePanelFocus(select, event.key === "ArrowDown" ? 1 : -1);
    return;
  }
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    const focused = panel.querySelector(".select-panel-item.is-focused");
    if (focused) pickSelectValue(select, focused.dataset.value);
    else closeSelectPanels();
  }
}

function initSelectPanels() {
  document.querySelectorAll("select.custom-select[data-preview]").forEach((select) => {
    select.addEventListener("mousedown", handleSelectMousedown);
    select.addEventListener("keydown", handleSelectKeydown);
  });
  document.addEventListener("mousedown", (event) => {
    if (!event.target.closest(".select-wrap")) {
      closeSelectPanels();
    }
  });
  rebuildSelectPanels();
}

function getTranslation(key, params = {}) {
  const dict = i18nDict[currentLanguage] || i18nDict.ru;
  let template = dict[key] || i18nDict.ru[key] || key;
  for (const [k, v] of Object.entries(params)) {
    template = template.replaceAll(`{${k}}`, v);
  }
  return template;
}

document.addEventListener("DOMContentLoaded", () => {
  // Navigation tabs follow the WAI-ARIA tab pattern and remain native buttons.
  const tabs = document.querySelectorAll(".nav-tab");
  const panels = document.querySelectorAll(".tab-panel");

  function activateTab(tab) {
    tabs.forEach((item) => {
      const selected = item === tab;
      item.classList.toggle("active", selected);
      item.setAttribute("aria-selected", String(selected));
      item.tabIndex = selected ? 0 : -1;
    });
    panels.forEach((panel) => {
      panel.style.display = panel.id === "panel-" + tab.dataset.tab ? "flex" : "none";
    });
    if (tab.dataset.tab === "history") loadHistoryList();
  }

  tabs.forEach((tab) => {
    tab.addEventListener("click", () => activateTab(tab));
  });
  bindTabKeyboardNavigation(tabs, activateTab);
  // Toggle API Key visibility
  const apiKeyInput = document.getElementById("input-api-key");
  const toggleKeyBtn = document.getElementById("btn-toggle-key");
  toggleKeyBtn.addEventListener("click", () => {
    if (apiKeyInput.type === "password") {
      apiKeyInput.type = "text";
      toggleKeyBtn.classList.add("visible");
    } else {
      apiKeyInput.type = "password";
      toggleKeyBtn.classList.remove("visible");
    }
  });

  // Engine change (Cloud vs Local) toggling Whisper card visibility
const radioCloud = document.getElementById("radio-cloud");
  const radioLocal = document.getElementById("radio-local");
  const selectLocalEngine = document.getElementById("select-local-engine");
  const groupWhisperModels = document.getElementById("group-whisper-models");
  const groupParakeetModels = document.getElementById("group-parakeet-models");

function updateEngineUI() {
    const vocabCard = document.getElementById("card-vocabulary");
    const vocabLabel = document.getElementById("section-label-dictionary");
    const fallbackCard = document.getElementById("card-cloud-fallback");
    const streamingCard = document.getElementById("card-streaming");
    const langCard = document.getElementById("card-language");
    const recognitionLabel = document.getElementById("section-label-recognition");
    const localEngineSection = document.getElementById("local-engine-section");
    if (radioLocal.checked) {
      if (localEngineSection) localEngineSection.style.display = "flex";
      updateLocalEngineUI();
      if (fallbackCard) fallbackCard.style.display = "none";
    } else {
      if (localEngineSection) localEngineSection.style.display = "none";
      if (vocabCard) vocabCard.style.display = "flex";
      if (vocabLabel) vocabLabel.style.display = "block";
      if (fallbackCard) fallbackCard.style.display = "flex";
      if (langCard) langCard.style.display = "flex";
      if (recognitionLabel) recognitionLabel.style.display = "block";
      if (streamingCard) streamingCard.style.display = "none";
    }
  }

function updateLocalEngineUI() {
    if (!selectLocalEngine || !groupWhisperModels || !groupParakeetModels) return;
    const isParakeet = selectLocalEngine.value === "parakeet";
    const vocabCard = document.getElementById("card-vocabulary");
    const vocabLabel = document.getElementById("section-label-dictionary");
    if (vocabCard) vocabCard.style.display = isParakeet ? "none" : "flex";
    if (vocabLabel) vocabLabel.style.display = isParakeet ? "none" : "block";
    const langCard = document.getElementById("card-language");
    if (langCard) langCard.style.display = isParakeet ? "none" : "flex";
    const recognitionLabel = document.getElementById("section-label-recognition");
    if (recognitionLabel) recognitionLabel.style.display = isParakeet ? "none" : "block";
    const streamingCard = document.getElementById("card-streaming");
    if (streamingCard) streamingCard.style.display = isParakeet ? "flex" : "none";
    const gpuSettings = document.getElementById("gpu-acceleration-settings");
    if (gpuSettings) {
      gpuSettings.style.display = isParakeet ? "block" : "none";
    }
    if (isParakeet) {
      groupWhisperModels.style.display = "none";
      groupParakeetModels.style.display = "block";
      selectModelCard("parakeet-v3");
    } else {
      groupWhisperModels.style.display = "block";
      groupParakeetModels.style.display = "none";
      if (selectedModelName === "parakeet-v3") {
        selectModelCard("base");
      }
    }
  }

async function refreshEngineHealth() {
    const chip = document.getElementById("engine-health-chip");
    if (!chip) return;
    try {
      const health = await invoke("get_engine_health");
      if (health.engine === "whisper" || health.engine === "parakeet-local-fallback") {
        chip.textContent = getTranslation("engine_health_whisper") || "Whisper: in-process";
        chip.classList.add("health-ok");
        chip.classList.remove("health-warn");
      } else if (health.running) {
        chip.textContent = getTranslation("engine_health_parakeet_running", {
          provider: health.provider || "cpu",
          port: health.port ?? "?",
        });
        chip.classList.add("health-ok");
        chip.classList.remove("health-warn");
      } else {
        chip.textContent = getTranslation("engine_health_parakeet_stopped");
        chip.classList.remove("health-ok");
        chip.classList.add("health-warn");
      }
    } catch (e) {
      console.error(e);
    }
  }

  if (selectLocalEngine) {
    selectLocalEngine.addEventListener("change", () => {
      updateLocalEngineUI();
      markSettingsModified();
      refreshEngineHealth();
    });
  }

  document.addEventListener("change", (event) => {
    if (event.target.matches("select.custom-select[data-preview]")) {
      updateSelectPreview(event.target);
    }
  });

  radioCloud.addEventListener("change", updateEngineUI);
  radioLocal.addEventListener("change", updateEngineUI);

  // Dynamic API Key Links
  const linkGetKey = document.getElementById("link-get-key");
  const providerLinks = {
    gemini: { url: "https://aistudio.google.com/", name: "Google AI Studio" },
    openai: { url: "https://platform.openai.com/api-keys", name: "OpenAI Platform" },
    groq: { url: "https://console.groq.com/keys", name: "Groq Console" }
  };
  function updateApiKeyLink() {
    const prov = selectProvider ? selectProvider.value : "gemini";
    const info = providerLinks[prov] || providerLinks.gemini;
    if (linkGetKey) {
      linkGetKey.href = info.url;
      linkGetKey.textContent = getTranslation("api_get_key_pattern", { name: info.name });
    }
  }

  // Settings elements
  const selectProvider = document.getElementById("select-provider");
  const selectHotkey = document.getElementById("input-hotkey");
  const selectLanguage = document.getElementById("select-language");
  const textareaDictionary = document.getElementById("textarea-dictionary");
  const checkboxToggle = document.getElementById("checkbox-toggle");
  const checkboxPunctuation = document.getElementById("checkbox-punctuation");
  const checkboxCloudFallback = document.getElementById("checkbox-cloud-fallback");
  const checkboxAutostart = document.getElementById("checkbox-autostart");
  const checkboxAutomaticUpdateChecks = document.getElementById("checkbox-automatic-update-checks");
const btnSaveSettings = document.getElementById("btn-save-settings");
  // NOTE: the click binding for the Save button lives once, in the "Bind Events"
  // block below — it must not be re-registered here (would double-save).
  
  const checkboxSounds = document.getElementById("checkbox-sounds");
  const checkboxCopyContext = document.getElementById("checkbox-selection-edit-enabled");
  const selectSoundTheme = document.getElementById("select-sound-theme");
  const rangeVolume = document.getElementById("range-sound-volume");
  const volumeLabel = document.getElementById("volume-value-label");

  if (rangeVolume) {
    rangeVolume.addEventListener("input", () => {
      if (volumeLabel) {
        volumeLabel.textContent = `${rangeVolume.value}%`;
      }
    });
  }

  // Hotkey Recorder Widget Events
  const btnResetHotkey = document.getElementById("btn-reset-hotkey");
  let isRecordingHotkey = false;
  let hasRecordedThisSession = false;

  const allowedSpecialKeys = {
    "Space": "Space",
    " ": "Space",
    "CapsLock": "Caps Lock",
    "Tab": "Tab",
    "F1": "F1", "F2": "F2", "F3": "F3", "F4": "F4", "F5": "F5", "F6": "F6",
    "F7": "F7", "F8": "F8", "F9": "F9", "F10": "F10", "F11": "F11", "F12": "F12"
  };

  if (selectHotkey) {
    selectHotkey.addEventListener("focus", () => {
      isRecordingHotkey = true;
      hasRecordedThisSession = false;
      selectHotkey.value = getTranslation("hotkey_prompt") || "Press keys...";
      selectHotkey.classList.add("recording");
    });

    selectHotkey.addEventListener("blur", () => {
      isRecordingHotkey = false;
      selectHotkey.classList.remove("recording");
      // Restore current settings value on blur ONLY if user didn't record a new combination
      if (!hasRecordedThisSession) {
        invoke("get_settings").then(settings => {
          if (settings) {
            selectHotkey.value = settings.hotkey || "Alt+V";
          }
        }).catch(() => {
          selectHotkey.value = "Alt+V";
        });
      }
    });

    selectHotkey.addEventListener("keydown", (e) => {
      if (!isRecordingHotkey) return;
      e.preventDefault();
      e.stopPropagation();

      const key = e.key;
      const code = e.code;

      // Ignore modifiers themselves
      if (key === "Control" || key === "Alt" || key === "Shift" || key === "Meta" ||
          code === "ControlLeft" || code === "ControlRight" ||
          code === "AltLeft" || code === "AltRight" ||
          code === "ShiftLeft" || code === "ShiftRight") {
        return;
      }

      let modifier = "";
      if (e.ctrlKey) modifier = "Ctrl";
      else if (e.altKey) modifier = "Alt";
      else if (e.shiftKey) modifier = "Shift";

      let keyName = "";
      if (code.startsWith("Key")) {
        // Physical letter keys, e.g. "KeyV" -> "V"
        keyName = code.substring(3).toUpperCase();
      } else if (code.startsWith("Digit")) {
        // Physical number keys, e.g. "Digit1" -> "1"
        keyName = code.substring(5);
      } else if (code.startsWith("F") && code.length >= 2 && !isNaN(code.substring(1))) {
        // Function keys, e.g. "F8" -> "F8"
        keyName = code;
      } else {
        // Map common physical layout codes
        const codeMap = {
          "Space": "Space",
          "CapsLock": "Caps Lock",
          "Tab": "Tab"
        };
        if (codeMap[code]) {
          keyName = codeMap[code];
        } else {
          // If e.code is empty or unrecognized, fallback to e.key for basic alphanumeric
          if (key.length === 1 && /[a-zA-Z0-9]/.test(key)) {
            keyName = key.toUpperCase();
          } else {
            return;
          }
        }
      }

      const hotkeyStr = modifier ? `${modifier}+${keyName}` : keyName;
      hasRecordedThisSession = true; // Mark as successfully recorded
      selectHotkey.value = hotkeyStr;
      isRecordingHotkey = false;
      selectHotkey.classList.remove("recording");
      selectHotkey.blur();

      // Trigger modified state
      selectHotkey.dispatchEvent(new Event("change", { bubbles: true }));
    });
  }

  if (btnResetHotkey) {
    btnResetHotkey.addEventListener("click", () => {
      if (selectHotkey) {
        selectHotkey.value = "Alt+V";
        selectHotkey.dispatchEvent(new Event("change", { bubbles: true }));
      }
    });
  }


  function updateSoundUI() {
    const themeGroup = document.getElementById("sound-theme-group");
    const volumeGroup = document.getElementById("sound-volume-group");
    const show = (checkboxSounds && checkboxSounds.checked) ? "flex" : "none";
    if (themeGroup) {
      themeGroup.style.display = show;
    }
    if (volumeGroup) {
      volumeGroup.style.display = show;
    }
  }
  if (checkboxSounds) {
    checkboxSounds.addEventListener("change", updateSoundUI);
  }
  
  let apiKeys = {
    gemini: "",
    openai: "",
    groq: ""
  };
  let apiKeyPresent = {
    gemini: false,
    openai: false,
    groq: false
  };
  let apiKeyDirty = {
    gemini: false,
    openai: false,
    groq: false
  };
  let previousSelProvider = selectProvider.value;

  function renderProviderKeyInput() {
    const provider = selectProvider.value;
    apiKeyInput.value = apiKeys[provider] || "";
const providerDict = i18nDict[currentLanguage] || i18nDict.ru;
    apiKeyInput.placeholder = apiKeyPresent[provider]
      ? providerDict.key_saved_placeholder || "•••••••• (saved securely)"
      : providerDict.key_placeholder || "Enter API key";
  }

  apiKeyInput.addEventListener("input", () => {
    const provider = selectProvider.value;
    apiKeys[provider] = apiKeyInput.value;
    apiKeyDirty[provider] = true;
  });
  
  const footerStatusText = document.getElementById("footer-status-text");

  let selectedModelName = "base";
  let activeLocalAcceleration = "cpu";

  let settingsModified = false;
  let isSettingsLoaded = false;

  function markSettingsModified() {
    if (!isSettingsLoaded) return;
    if (!settingsModified) {
      settingsModified = true;
      showStatus(getTranslation("status_modified"), false, true);
    }
  }

  function bindSettingsChangeListeners() {
    const checkboxStreaming = document.getElementById("checkbox-streaming");
    const checkboxLogSpeechText = document.getElementById("setting-log-speech-text");
    const inputs = [
      radioCloud, radioLocal, selectProvider, apiKeyInput, selectHotkey,
      selectLanguage, textareaDictionary, checkboxToggle, checkboxPunctuation, checkboxCloudFallback,
      checkboxAutostart, checkboxAutomaticUpdateChecks, checkboxStreaming, checkboxSounds,
      selectSoundTheme, rangeVolume, selectLocalEngine, checkboxCopyContext, checkboxLogSpeechText
    ];
    inputs.forEach(input => {
      if (input) {
        input.addEventListener("change", markSettingsModified);
        input.addEventListener("input", markSettingsModified);
      }
    });
  }
  const modelCards = document.querySelectorAll(".model-card[data-model]");

  // WAI-ARIA radio group: arrow keys move between cards within the same group
  const arrowDirection = { ArrowUp: -1, ArrowLeft: -1, ArrowRight: 1, ArrowDown: 1 };

  modelCards.forEach(card => {
    card.addEventListener("click", (e) => {
      if (e.target.closest("[data-static]")) {
        return;
      }
      // Prevent selection trigger when clicking delete/download buttons inside the card
      if (e.target.closest(".btn-delete-card-model") || e.target.closest(".btn-download-card-model") || e.target.closest(".btn-cancel-download")) {
        return;
      }
      selectModelCard(card.dataset.model);
    });

    card.addEventListener("keydown", (e) => {
      if (e.target.closest("[data-static]")) {
        return;
      }
      // Inner buttons (delete/download/cancel) handle their own keys
      if (e.target.closest("button")) {
        return;
      }
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        selectModelCard(card.dataset.model);
        return;
      }
      const direction = arrowDirection[e.key];
      if (direction === undefined) {
        return;
      }
      e.preventDefault();
      const group = card.parentElement;
      if (!group) {
        return;
      }
      const groupCards = Array.from(group.querySelectorAll(".model-card[data-model]"));
      const index = groupCards.indexOf(card);
      if (index === -1) {
        return;
      }
      const next = groupCards[(index + direction + groupCards.length) % groupCards.length];
      next.focus();
      selectModelCard(next.dataset.model);
    });
  });

function selectModelCard(model) {
    if (model === "punctuation") {
      return;
    }
    if (model === "parakeet-v3") {
      const parakeetOpt = selectLocalEngine?.querySelector('option[value="parakeet"]');
      if (parakeetOpt?.disabled) return;
    }
    if (selectedModelName !== model) {
      selectedModelName = model;
      markSettingsModified();
    }
    modelCards.forEach(c => {
      const isCurrent = c.dataset.model === model;
      c.classList.toggle("active", isCurrent);
      c.setAttribute("aria-checked", isCurrent ? "true" : "false");
    });
  }

  // Load Settings from Backend
  async function loadSettings(preFetchedSettings = null) {
    try {
      const dict = i18nDict[currentLanguage] || i18nDict.ru;
      showStatus(dict.status_loading || "Загрузка настроек...");
      const settings = preFetchedSettings || await invoke("get_settings");
      
      if (settings) {
        if (settings.transcription_mode === "local") {
          radioLocal.checked = true;
        } else {
          radioCloud.checked = true;
        }
        
        selectModelCard(settings.model_name || "base");
        apiKeys = { gemini: "", openai: "", groq: "" };
        apiKeyDirty = { gemini: false, openai: false, groq: false };
        apiKeyPresent = {
          gemini: !!settings.has_api_key_gemini,
          openai: !!settings.has_api_key_openai,
          groq: !!settings.has_api_key_groq
        };

        selectProvider.value = settings.api_provider || "gemini";
        previousSelProvider = selectProvider.value;
        renderProviderKeyInput();
        updateApiKeyLink();
        if (selectHotkey) {
          selectHotkey.value = settings.hotkey || "Alt+V";
        }
        if (selectLanguage) {
          selectLanguage.value = settings.language || "auto";
        }
        if (selectLocalEngine) {
          selectLocalEngine.value = settings.local_engine || "whisper";
        }
        updateEngineUI();
        if (textareaDictionary) {
          textareaDictionary.value = settings.dictionary || "";
        }
        if (checkboxToggle) {
          checkboxToggle.checked = !!settings.toggle_enabled;
        }
        if (checkboxPunctuation) {
          checkboxPunctuation.checked = !!settings.voice_punctuation;
        }
        if (checkboxCloudFallback) {
          checkboxCloudFallback.checked = settings.cloud_fallback_enabled !== false;
        }
        if (checkboxAutostart) {
          checkboxAutostart.checked = !!settings.autostart;
        }        if (checkboxAutomaticUpdateChecks) {
          checkboxAutomaticUpdateChecks.checked = !!settings.automatic_update_checks;
        }

 
        const checkboxStreaming = document.getElementById("checkbox-streaming");
        if (checkboxStreaming) {
          checkboxStreaming.checked = !!settings.streaming_enabled;
        }
 
  if (checkboxSounds) {
    checkboxSounds.checked = settings.overlay_sounds !== false;
  }
  if (checkboxCopyContext) {

    checkboxCopyContext.checked = !!settings.copy_context_on_start;
  }
        const checkboxLogSpeechText = document.getElementById("setting-log-speech-text");
        if (checkboxLogSpeechText) {
          checkboxLogSpeechText.checked = !!settings.log_speech_text;
        }
        activeLocalAcceleration = settings.local_acceleration || "cpu";
        if (selectSoundTheme) {
          selectSoundTheme.value = settings.overlay_sound_theme || "zen";
        }
        if (rangeVolume) {
          const volumeVal = typeof settings.overlay_sound_volume === "number" ? Math.round(settings.overlay_sound_volume * 100) : 80;
          rangeVolume.value = volumeVal;
          if (volumeLabel) {
            volumeLabel.textContent = `${volumeVal}%`;
          }
        }
        updateSoundUI();
 
        updateEngineUI();
        await refreshDownloadedModels();

activeLocalAcceleration = settings.local_acceleration || "cpu";
        selectGpuProvider(activeLocalAcceleration);
        await updateGpuCardStates();
        
        isSettingsLoaded = true;
        settingsModified = false;
        
        refreshEngineHealth();
        setInterval(() => {
          if (document.hasFocus()) {
            refreshEngineHealth();
          }
        }, 10000);
        
        showStatus(getTranslation("status_loaded"));
        
        bindSettingsChangeListeners();
      }
    } catch (err) {
      console.error(err);
      showStatus(`${getTranslation("status_load_error")}${err}`, true);
    }
  }

  async function refreshDownloadedModels() {
    try {
      const downloaded = await invoke("get_downloaded_models");
      const parakeetOption = selectLocalEngine?.querySelector('option[value="parakeet"]');
      if (parakeetOption) {
        const parakeetInstalled = downloaded.includes("parakeet-v3");
        parakeetOption.disabled = !parakeetInstalled;
        if (!parakeetInstalled && selectLocalEngine.value === "parakeet") {
          selectLocalEngine.value = "whisper";
          updateLocalEngineUI();
          markSettingsModified();
        }
      }
      const dict = i18nDict[currentLanguage] || i18nDict.ru;
modelCards.forEach(card => {
        const model = card.dataset.model;
        // Never tear down the UI of a download that is still running
        // (a refresh triggered by a sibling download must not do it either).
        if (inFlightModelDownloads.has(model)) {
          return;
        }
        const isDownloaded = downloaded.includes(model);
        const actionEl = document.getElementById(`action-${model}`);

        // Always restore a clean, non-downloading state. Without this, a cancelled
        // download leaves the progress bar frozen and the action button hidden.
        const progressEl = document.getElementById(`progress-${model}`);
        if (progressEl) {
          const cancelBtn = progressEl.querySelector(".btn-cancel-download");
          if (cancelBtn) cancelBtn.remove();
          progressEl.style.display = "none";
        }
        if (actionEl) actionEl.style.display = "flex";

        if (isDownloaded) {
          actionEl.innerHTML = `
            <span class="status-ready-badge">
              <svg class="status-ready-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12"></polyline>
              </svg>
              <span data-i18n="model_status_ready">${dict.model_status_ready || "Установлено"}</span>
            </span>
            <button type="button" class="btn-delete-card-model" title="${dict.model_action_delete || "Удалить"}" data-model="${model}">
              <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path><line x1="10" y1="11" x2="10" y2="17"></line><line x1="14" y1="11" x2="14" y2="17"></line></svg>
            </button>
          `;
          // Bind click to the delete button
          actionEl.querySelector(".btn-delete-card-model").addEventListener("click", () => deleteModelCard(model));
        } else {
          actionEl.innerHTML = `
            <button type="button" class="btn-download-card-model" data-model="${model}">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="btn-icon"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>
              ${dict.model_action_download || "Скачать"}
            </button>
          `;
// Bind click to the download button
          actionEl.querySelector(".btn-download-card-model").addEventListener("click", () => downloadModelCard(model));
        }
      });
      refreshEngineHealth();
    } catch (err) {
      console.error("Failed to check downloaded models", err);
    }
  }

  // Save Settings to Backend
  let saveInFlight = false;

  async function saveSettings() {
    // Guard against re-entrancy: rapid clicks or duplicate bindings must not
    // start a second save while one is already running.
    if (saveInFlight) {
      return;
    }
    saveInFlight = true;
    if (btnSaveSettings) {
      btnSaveSettings.disabled = true;
    }
    try {
      const dict = i18nDict[currentLanguage] || i18nDict.ru;
      showStatus(dict.status_saving || "Сохранение настроек...");
      
      const checkboxStreaming = document.getElementById("checkbox-streaming");
      
      // Update apiKeys cache from active input first:
      apiKeys[selectProvider.value] = apiKeyInput.value.trim();

      const soundVolFloat = rangeVolume ? parseFloat(rangeVolume.value) / 100 : 0.8;

      const settings = {
        transcription_mode: radioLocal.checked ? "local" : "cloud",
        api_provider: selectProvider.value,

        model_name: selectedModelName,
        hotkey: selectHotkey ? selectHotkey.value : "Alt+V",
        streaming_enabled: checkboxStreaming ? checkboxStreaming.checked : false,
        toggle_enabled: checkboxToggle ? checkboxToggle.checked : false,
        language: selectLanguage ? selectLanguage.value : "auto",
        dictionary: textareaDictionary ? textareaDictionary.value : "",
        voice_punctuation: checkboxPunctuation ? checkboxPunctuation.checked : false,
        cloud_fallback_enabled: checkboxCloudFallback ? checkboxCloudFallback.checked : true,
        autostart: checkboxAutostart ? checkboxAutostart.checked : false,
        automatic_update_checks: checkboxAutomaticUpdateChecks ? checkboxAutomaticUpdateChecks.checked : false,
        local_engine: selectLocalEngine ? selectLocalEngine.value : "whisper",
        local_acceleration: activeLocalAcceleration,
        overlay_sounds: checkboxSounds ? checkboxSounds.checked : true,
    overlay_sound_theme: selectSoundTheme ? selectSoundTheme.value : "zen",
    overlay_sound_volume: soundVolFloat,
    copy_context_on_start: checkboxCopyContext ? checkboxCopyContext.checked : false,
    log_speech_text: (() => {
      const el = document.getElementById("setting-log-speech-text");
      return el ? el.checked : false;
    })()
  };

await invoke("set_settings", { settings });
      const failedProviders = [];
      for (const provider of ["gemini", "openai", "groq"]) {
        if (apiKeyDirty[provider]) {
          const key = apiKeys[provider].trim();
          try {
            await invoke("set_provider_key", { provider, key });
            apiKeyPresent[provider] = key.length > 0;
            apiKeyDirty[provider] = false;
            apiKeys[provider] = "";
          } catch (keyErr) {
            // Save the rest of the keys anyway; only the failed provider
            // stays dirty so the next save retries it.
            console.error(`Failed to save ${provider} key:`, keyErr);
            failedProviders.push(provider);
          }
        }
      }
      renderProviderKeyInput();
      settingsModified = false;
      if (failedProviders.length > 0) {
        showStatus(
          `${getTranslation("status_save_error")} (${failedProviders.join(", ")})`,
          true
        );
      } else {
        showStatus(dict.status_saved || "Настройки успешно сохранены!");
      }
      
      // Temporary success animation in footer status
      setTimeout(() => {
        if (!settingsModified) {
          const currentDict = i18nDict[currentLanguage] || i18nDict.ru;
          showStatus(currentDict.status_ready || "Готово");
        }
      }, 3000);
} catch (err) {
      console.error(err);
      showStatus(`${getTranslation("status_save_error")}${err}`, true);
    } finally {
      saveInFlight = false;
      if (btnSaveSettings) {
        btnSaveSettings.disabled = false;
      }
    }
  }

async function downloadModelCard(model) {
    if (inFlightModelDownloads.has(model)) {
      return;
    }
    inFlightModelDownloads.add(model);
    try {
      showStatus(getTranslation("model_downloading_pattern", { model }));
      const actionEl = document.getElementById(`action-${model}`);
      const progressEl = document.getElementById(`progress-${model}`);
      const fillEl = document.getElementById(`fill-${model}`);
      const pctEl = document.getElementById(`pct-${model}`);

      // Hide actions, show progress
      actionEl.style.display = "none";
      progressEl.style.display = "flex";
      fillEl.style.width = "0%";
      pctEl.textContent = "0%";

      // Add a fresh cancel (×) button while downloading
      if (progressEl) {
        const oldBtn = progressEl.querySelector(".btn-cancel-download");
        if (oldBtn) oldBtn.remove();
        const cancelBtn = document.createElement("button");
        cancelBtn.type = "button";
        cancelBtn.className = "btn-cancel-download";
        const cancelLabel = getTranslation("model_cancel_download") || "Отменить загрузку";
        cancelBtn.title = cancelLabel;
        cancelBtn.setAttribute("aria-label", cancelLabel);
        cancelBtn.innerHTML = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>';
        cancelBtn.addEventListener("click", (e) => {
          e.stopPropagation();
          cancelBtn.disabled = true;
          invoke("cancel_model_download", { modelName: model }).catch(e2 => console.error(e2));
        });
        progressEl.appendChild(cancelBtn);
      }

await invoke("download_model_command", { modelName: model });
      refreshDownloadedModels();
    } catch (err) {
      console.error(err);
      const errStr = String(err).toLowerCase();
      if (errStr.includes("cancel")) {
        showStatus(getTranslation("model_download_cancelled") || "Загрузка отменена");
      } else {
showStatus(getTranslation("model_download_error_pattern", { err }), true);
      }
      refreshDownloadedModels();
    } finally {
      inFlightModelDownloads.delete(model);
    }
  }

  async function deleteModelCard(model) {
    const confirmTitle = getTranslation("delete_model_title");
    const confirmMsg = getTranslation("delete_model_confirm_pattern", { model });
    const confirmBtn = getTranslation("delete_model_btn");
    const cancelBtn = getTranslation("confirm_cancel");

    const confirmed = await showConfirm(
      confirmTitle,
      confirmMsg,
      confirmBtn,
      cancelBtn
    );
    if (!confirmed) {
      return;
    }
    try {
      showStatus(getTranslation("model_deleting_pattern", { model }));
      await invoke("delete_model_command", { modelName: model });
      
showStatus(getTranslation("model_deleted_success"));
      if (model === "parakeet-v3" && selectLocalEngine?.value === "parakeet") {
        selectLocalEngine.value = "whisper";
        updateLocalEngineUI();
        markSettingsModified();
      }
      await refreshDownloadedModels();
    } catch (err) {
      console.error(err);
      showStatus(getTranslation("model_delete_error_pattern", { err }), true);
    }
  }

  // Listen to model-download-progress events from Rust
  listen("model-download-progress", (event) => {
    const payload = event.payload;
    if (!payload) return;

    const model = payload.model;
    const percent = Math.round(payload.percentage);
    
    const fillEl = document.getElementById(`fill-${model}`);
    const pctEl = document.getElementById(`pct-${model}`);
    const progressEl = document.getElementById(`progress-${model}`);
    const actionEl = document.getElementById(`action-${model}`);
    
    if (fillEl && pctEl) {
      fillEl.style.width = `${percent}%`;
      pctEl.textContent = `${percent}%`;
    }

    if (payload.done) {
      showStatus(getTranslation("model_downloaded_success_pattern", { model }));
      if (progressEl) {
        const cancelBtn = progressEl.querySelector(".btn-cancel-download");
        if (cancelBtn) cancelBtn.remove();
        progressEl.style.display = "none";
      }
      if (actionEl) actionEl.style.display = "flex";
      refreshDownloadedModels();
    }
  });

  // --- Local GPU Acceleration Logic ---
  async function checkGpuInstalled(provider) {
    if (provider === "cpu") return true;
    try {
      return await invoke("check_gpu_downloaded", { provider });
    } catch (e) {
      console.error(e);
      return false;
    }
  }

  function selectGpuProvider(provider) {
    if (activeLocalAcceleration !== provider) {
      activeLocalAcceleration = provider;
      markSettingsModified();
    }
    document.querySelectorAll("[data-gpu]").forEach(card => {
      const isSelected = card.getAttribute("data-gpu") === provider;
      card.setAttribute("aria-checked", isSelected ? "true" : "false");
      card.classList.toggle("active", isSelected);
    });
  }

  const activeGpuDownloads = new Set();
  // Whisper-model downloads in flight; refreshDownloadedModels must leave
  // their progress UI untouched until they finish or fail.
  const inFlightModelDownloads = new Set();

  async function updateGpuCardStates() {
    const providers = ["cuda"];
    const dict = i18nDict[currentLanguage] || i18nDict.ru;
    for (const provider of providers) {
      if (activeGpuDownloads.has(provider)) {
        continue;
      }
      const isDownloaded = await checkGpuInstalled(provider);
      const actionEl = document.getElementById(`action-gpu-${provider}`);
      const progressEl = document.getElementById(`progress-gpu-${provider}`);
      if (!actionEl) continue;
      if (progressEl) {
        const cancelBtn = progressEl.querySelector(".btn-cancel-download");
        if (cancelBtn) cancelBtn.remove();
        progressEl.style.display = "none";
      }
      actionEl.style.display = "flex";

      if (isDownloaded) {
        actionEl.innerHTML = `
          <span class="status-ready-badge">
            <svg class="status-ready-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
            <span data-i18n="model_status_ready">${dict.model_status_ready || "Установлено"}</span>
          </span>
          <button type="button" class="btn-delete-card-model" title="${dict.model_action_delete || "Удалить"}" data-gpu="${provider}">
            <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path><line x1="10" y1="11" x2="10" y2="17"></line><line x1="14" y1="11" x2="14" y2="17"></line></svg>
          </button>
        `;
        actionEl.querySelector(".btn-delete-card-model").addEventListener("click", (e) => {
          e.stopPropagation();
          deleteGpuBinaries(provider);
        });
      } else {
        actionEl.innerHTML = `
          <button type="button" class="btn-download-card-model" data-gpu="${provider}">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="btn-icon"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>
            ${dict.model_action_download || "Скачать"}
          </button>
        `;
        actionEl.querySelector(".btn-download-card-model").addEventListener("click", (e) => {
          e.stopPropagation();
          downloadGpuBinaries(provider);
        });
      }
    }
  }

  async function downloadGpuBinaries(provider) {
    if (activeGpuDownloads.has(provider)) {
      return;
    }

    const actionEl = document.getElementById(`action-gpu-${provider}`);
    const progressEl = document.getElementById(`progress-gpu-${provider}`);
    const fillEl = document.getElementById(`fill-gpu-${provider}`);
    const percentEl = document.getElementById(`pct-gpu-${provider}`);
    
    activeGpuDownloads.add(provider);
    if (actionEl) actionEl.style.display = "none";
    if (progressEl) {
      progressEl.style.display = "flex";
      const oldBtn = progressEl.querySelector(".btn-cancel-download");
      if (oldBtn) oldBtn.remove();
      const cancelBtn = document.createElement("button");
      cancelBtn.type = "button";
      cancelBtn.className = "btn-cancel-download";
      const cancelLabel = getTranslation("model_cancel_download") || "Отменить загрузку";
      cancelBtn.title = cancelLabel;
      cancelBtn.setAttribute("aria-label", cancelLabel);
      cancelBtn.innerHTML = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>';
cancelBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        cancelBtn.disabled = true;
        invoke("cancel_gpu_download", { provider }).catch((e2) => {
          console.error(e2);
          cancelBtn.disabled = false;
        });
      });
      progressEl.appendChild(cancelBtn);
    }
    if (fillEl) fillEl.style.width = "0%";
    if (percentEl) percentEl.textContent = "0%";

    try {
      showStatus(getTranslation("model_downloading_pattern", { model: provider.toUpperCase() }));
      await invoke("download_gpu_binaries", { provider });
    } catch (err) {
      console.error(err);
      const errStr = String(err).toLowerCase();
      if (errStr.includes("cancel")) {
        showStatus(getTranslation("model_download_cancelled") || "Загрузка отменена");
      } else {
        showStatus(`${getTranslation("status_error")}${err}`, true);
      }
    } finally {
      activeGpuDownloads.delete(provider);
      await updateGpuCardStates();
    }
  }

  async function deleteGpuBinaries(provider) {
    const dict = i18nDict[currentLanguage] || i18nDict.ru;
    const title = dict.delete_model_title || "Удаление";
    const message = dict.confirm_message || "Вы действительно хотите выполнить это действие?";
    const confirmText = dict.confirm_ok || "Удалить";
    const cancelText = dict.confirm_cancel || "Отмена";
    
    const confirmed = await showConfirm(title, message, confirmText, cancelText);
    if (confirmed) {
      try {
        await invoke("delete_gpu_binaries", { provider });
        if (activeLocalAcceleration === provider) {
          selectGpuProvider("cpu");
        }
        await updateGpuCardStates();
      } catch (err) {
        console.error(err);
        showStatus(getTranslation("model_delete_error_pattern", { err: String(err) }), true);
      }
    }
  }

  // Listen to GPU download progress events
  listen("gpu-download-progress", event => {
    const progress = event.payload;
    if (!progress) return;
    const fillEl = document.getElementById(`fill-gpu-${progress.provider}`);
    const percentEl = document.getElementById(`pct-gpu-${progress.provider}`);
    const progressEl = document.getElementById(`progress-gpu-${progress.provider}`);
    const actionEl = document.getElementById(`action-gpu-${progress.provider}`);

    if (fillEl && percentEl) {
      const percentage = typeof progress.percentage === 'number' ? Math.round(progress.percentage) : 0;
      fillEl.style.width = `${percentage}%`;
      percentEl.textContent = `${percentage}%`;
    }

    if (progress.done) {
      if (progressEl) {
        const cancelBtn = progressEl.querySelector(".btn-cancel-download");
        if (cancelBtn) cancelBtn.remove();
        progressEl.style.display = "none";
      }
      if (actionEl) actionEl.style.display = "flex";
      updateGpuCardStates();
    }
  });

  // Bind GPU card event listeners
  document.querySelectorAll("[data-gpu]").forEach(card => {
    card.addEventListener("click", async (e) => {
      // Prevent selection trigger when clicking delete/download buttons inside the card
      if (e.target.closest(".btn-delete-card-model") || e.target.closest(".btn-download-card-model")) {
        return;
      }
      const provider = card.getAttribute("data-gpu");
      const installed = await checkGpuInstalled(provider);
      if (installed) {
        selectGpuProvider(provider);
      }
    });

card.addEventListener("keydown", async (e) => {
      if (e.target.tagName === "BUTTON" || e.target.closest("button")) {
        return;
      }
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        const provider = card.getAttribute("data-gpu");
        const installed = await checkGpuInstalled(provider);
        if (installed) {
          selectGpuProvider(provider);
        }
        return;
      }
      const direction = arrowDirection[e.key];
      if (direction === undefined) {
        return;
      }
      e.preventDefault();
      const group = card.parentElement;
      if (!group) {
        return;
      }
      const groupCards = Array.from(group.querySelectorAll("[data-gpu]"));
      const index = groupCards.indexOf(card);
      if (index === -1) {
        return;
      }
      let next = groupCards[(index + direction + groupCards.length) % groupCards.length];
      // Skip disabled/hidden cards (e.g. DirectML)
      while (next !== card && (next.hasAttribute("hidden") || next.getAttribute("aria-disabled") === "true")) {
        const nextIndex = groupCards.indexOf(next);
        next = groupCards[(nextIndex + direction + groupCards.length) % groupCards.length];
      }
      if (next === card) {
        return;
      }
      next.focus();
      const provider = next.getAttribute("data-gpu");
      const installed = await checkGpuInstalled(provider);
      if (installed) {
        selectGpuProvider(provider);
      }
    });
  });

  // --- Asynchronous native confirmation dialog ---
  function showConfirm(title, message, confirmText = "ОК", cancelText = "Отмена") {
    return new Promise((resolve) => {
      const modal = document.getElementById("custom-confirm-modal");
      const titleEl = document.getElementById("confirm-modal-title");
      const msgEl = document.getElementById("confirm-modal-message");
      const btnOk = document.getElementById("btn-confirm-ok");
      const btnCancel = document.getElementById("btn-confirm-cancel");
      if (!(modal instanceof HTMLDialogElement) || !titleEl || !msgEl || !btnOk || !btnCancel) {
        resolve(false);
        return;
      }

      titleEl.textContent = title;
      msgEl.textContent = message;
      btnOk.textContent = confirmText;
      btnCancel.textContent = cancelText;
      if (modal.open) modal.close();
      modal.showModal();
      requestAnimationFrame(() => modal.classList.add("active"));

      let settled = false;
      function cleanUp(result) {
        if (settled) return;
        settled = true;
        modal.classList.remove("active");
        btnOk.removeEventListener("click", onOk);
        btnCancel.removeEventListener("click", onCancel);
        modal.removeEventListener("cancel", onDialogCancel);
        setTimeout(() => {
          if (modal.open) modal.close();
          resolve(result);
        }, 200);
      }
      function onOk() { cleanUp(true); }
      function onCancel() { cleanUp(false); }
      function onDialogCancel(event) {
        event.preventDefault();
        cleanUp(false);
      }
      btnOk.addEventListener("click", onOk);
      btnCancel.addEventListener("click", onCancel);
      modal.addEventListener("cancel", onDialogCancel);
    });
  }
  function showStatus(msg, isError = false, isModified = false) {
    footerStatusText.textContent = msg;
    const footerStatus = footerStatusText.closest(".footer-status");
    
    if (footerStatus) {
      footerStatus.classList.remove("modified", "error", "success");
      const iconEl = document.getElementById("footer-status-icon");
      if (isError) {
        footerStatus.classList.add("error");
        footerStatusText.style.color = "var(--status-error)";
        if (iconEl) {
          iconEl.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line><circle cx="12" cy="12" r="10"></circle></svg>`;
        }
      } else if (isModified) {
        footerStatus.classList.add("modified");
        footerStatusText.style.color = "var(--status-modified)";
        if (iconEl) {
          iconEl.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"></path><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4L16.5 3.5z"></path></svg>`;
        }
      } else {
        footerStatus.classList.add("success");
        footerStatusText.style.color = "";
        if (iconEl) {
          iconEl.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>`;
        }
      }
    }
  }

  // Bind Events
  if (btnSaveSettings) {
    btnSaveSettings.addEventListener("click", saveSettings);
  }
  selectProvider.addEventListener("change", () => {
    // Keep only unsaved drafts in memory; stored keys are never returned by IPC.
    apiKeys[previousSelProvider] = apiKeyInput.value;
    previousSelProvider = selectProvider.value;
    renderProviderKeyInput();
    updateApiKeyLink();
  });

  // Window controls via Tauri IPC commands
  const btnWindowMinimize = document.getElementById("btn-window-minimize");
  const btnWindowClose = document.getElementById("btn-window-close");
  
  if (btnWindowMinimize) {
btnWindowMinimize.addEventListener("click", () => invoke("minimize_window").catch(e => console.error(e)));
  }
  if (btnWindowClose) {
    btnWindowClose.addEventListener("click", () => invoke("close_window").catch(e => console.error(e)));
  }

  // Window dragging via mousedown on header (bypasses click-through/drag bugs in Webview2)
  const appHeader = document.querySelector(".app-header");
  if (appHeader) {
    appHeader.addEventListener("mousedown", (e) => {
      // Only trigger drag on left click and avoid dragging when clicking on control buttons or select elements
if (e.button === 0 && !e.target.closest(".window-control-btn") && !e.target.closest("button") && !e.target.closest("select")) {
        invoke("start_dragging_command").catch(e => console.error(e));
      }
    });
  }

  // Translations Helper
  function applyLanguage(lang) {
    currentLanguage = lang;
    document.documentElement.lang = i18nDict[lang] ? lang : "ru";
    const dict = i18nDict[lang] || i18nDict.ru;
    
    // Update data-i18n elements
    const elements = document.querySelectorAll("[data-i18n]");
    elements.forEach(el => {
      const key = el.getAttribute("data-i18n");
      const text = dict[key] || i18nDict.ru[key];
      if (text) {
        el.textContent = text;
      }
    });

    const selectUiLang = document.getElementById("select-ui-lang");
    if (selectUiLang) {
      selectUiLang.setAttribute("aria-label", dict.general_ui_lang_title || "UI Language");
    }

    const btnReset = document.getElementById("btn-reset-hotkey");
    if (btnReset) {
      btnReset.setAttribute("title", dict.hotkey_reset_title || "Сбросить на Alt+V");
    }

    
    // Update inputs and placeholders
    const apiInput = document.getElementById("input-api-key");
    if (apiInput) {
      apiInput.placeholder = dict.api_key_placeholder || "";
    }
    const dictionaryTextarea = document.getElementById("textarea-dictionary");
    if (dictionaryTextarea) {
      dictionaryTextarea.placeholder = dict.vocab_placeholder || "";
    }
    
    // Update dynamic link text
    updateApiKeyLink();
    
    // Refresh model cards status/actions
    refreshDownloadedModels();
    updateGpuCardStates();
    
    // If settings modified status is showing, update it
    if (settingsModified) {
      showStatus(dict.status_modified, false, true);
    }
    
    // Reload history list if the active panel is panel-history
    const historyTab = document.getElementById("tab-btn-history");
    if (historyTab && historyTab.classList.contains("active")) {
      loadHistoryList();
    }

    updateAllSelectPreviews();
    rebuildSelectPanels();
  }

  // --- History List & Clear Interactions ---
  const historyContainer = document.getElementById("history-items-container");
  const btnClearHistory = document.getElementById("btn-clear-history");

  async function loadHistoryList() {
    if (!historyContainer) return;
    try {
      const history = await invoke("get_history");
      const dict = i18nDict[currentLanguage] || i18nDict.ru;
      
      if (!history || history.length === 0) {
        historyContainer.innerHTML = `<div class="history-empty-state" id="history-empty-text" data-i18n="history_empty">${dict.history_empty}</div>`;
        return;
      }

historyContainer.innerHTML = "";
      const fragment = document.createDocumentFragment();

      function formatHistoryDuration(ms) {
        if (!ms) return "";
        if (ms < 1000) return `${ms} ${dict.history_unit_ms || "ms"}`;
        const secs = ms >= 10000 ? Math.round(ms / 1000) : Math.round((ms / 1000) * 10) / 10;
        return `${secs} ${dict.history_unit_sec || "s"}`;
      }

      history.forEach(entry => {
        const date = new Date(entry.timestamp_ms);
        const timeStr = date.toLocaleTimeString(currentLanguage, { hour: '2-digit', minute: '2-digit', second: '2-digit' });
        const dateStr = date.toLocaleDateString(currentLanguage, { month: 'short', day: 'numeric' });
        const displayTime = `${dateStr}, ${timeStr}`;

const itemEl = document.createElement("div");
        itemEl.className = "history-item";

        let badgeHtml;
        const engineLabel = dict[`history_engine_${entry.engine}`];
        if (engineLabel) {
          const durationHtml = entry.processing_ms
            ? `<span class="history-item-duration">${formatHistoryDuration(entry.processing_ms)}</span>`
            : "";
          badgeHtml =
            `<span class="history-item-badge badge-local">${escapeHtml(engineLabel)}</span>${durationHtml}`;
        } else if (entry.mode === "cloud") {
          badgeHtml = `<span class="history-item-badge badge-cloud">${escapeHtml(dict.history_badge_cloud || "Cloud")}</span>`;
        } else {
          badgeHtml = `<span class="history-item-badge badge-local">${escapeHtml(dict.history_badge_local || "Local")}</span>`;
        }

        itemEl.innerHTML = `
          <div class="history-item-body">
            <div class="history-item-meta">
              <span class="history-item-time">${displayTime}</span>
              ${badgeHtml}
            </div>
            <div class="history-item-text">${escapeHtml(entry.text)}</div>
          </div>
          <button type="button" class="btn-copy-history" title="Copy to clipboard">
            <svg class="copy-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
            </svg>
            <svg class="check-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display: none; color: var(--accent-color);">
              <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
          </button>
        `;

        // Bind copy event
        const btnCopy = itemEl.querySelector(".btn-copy-history");
        const copyIcon = itemEl.querySelector(".copy-icon");
        const checkIcon = itemEl.querySelector(".check-icon");

        btnCopy.addEventListener("click", async () => {
          try {
            await invoke("copy_to_clipboard", { text: entry.text });
            
            // Hide copy icon, show checkmark SVG
            copyIcon.style.display = "none";
            checkIcon.style.display = "block";
            
            if (btnCopy._copyTimeout) {
              clearTimeout(btnCopy._copyTimeout);
            }
            
            btnCopy._copyTimeout = setTimeout(() => {
              checkIcon.style.display = "none";
              copyIcon.style.display = "block";
              btnCopy._copyTimeout = null;
            }, 1500);
          } catch (err) {
            console.error("Failed to copy", err);
          }
        });

        fragment.appendChild(itemEl);
      });
      historyContainer.appendChild(fragment);
    } catch (err) {
      console.error("Failed to load history", err);
    }
  }

  function escapeHtml(text) {
    return (text || "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");
  }

  const btnCopyDiagnostics = document.getElementById("btn-copy-diagnostics");
  if (btnCopyDiagnostics) {
    btnCopyDiagnostics.addEventListener("click", async () => {
      try {
        const report = await invoke("get_diagnostic_report");
        try {
          await invoke("copy_to_clipboard", { text: report });
        } catch (e) {
          if (navigator.clipboard && navigator.clipboard.writeText) {
            await navigator.clipboard.writeText(report);
          } else {
            throw e;
          }
        }
        showStatus(getTranslation("toast_diagnostics_copied"));
      } catch (err) {
        console.error("Failed to copy diagnostic report", err);
        showStatus(`${getTranslation("status_error")}${err}`, true);
      }
    });
  }

  if (btnClearHistory) {
    btnClearHistory.addEventListener("click", async () => {
      const dict = i18nDict[currentLanguage] || i18nDict.ru;
      const confirmed = await showConfirm(
        dict.confirm_clear_history_title,
        dict.confirm_clear_history_msg,
        dict.confirm_ok,
        dict.confirm_cancel
      );
      if (confirmed) {
        try {
          await invoke("clear_history");
          loadHistoryList();
        } catch (err) {
          console.error("Failed to clear history", err);
        }
      }
    });
  }

  let updateAvailable = false;

  async function checkForUpdates(announceNoUpdate = false) {
    const checkButton = document.getElementById("btn-check-updates");
    const badge = document.getElementById("update-badge");
    const badgeText = document.getElementById("update-badge-text");
    const navDot = document.getElementById("update-dot");
    if (checkButton) checkButton.disabled = true;
    try {
      const info = await invoke("check_for_app_update");
      updateAvailable = !!info;
      if (!info) {
        if (badge) badge.style.display = "none";
        if (navDot) navDot.style.display = "none";
        if (announceNoUpdate) showStatus(getTranslation("update_current"));
        return;
      }
      const label = getTranslation("update_available") || "Доступно обновление";
      if (badgeText) badgeText.textContent = label + " (v" + info.version + ")";
      if (badge) badge.style.display = "inline-flex";
      if (navDot) navDot.style.display = "inline-block";
      if (announceNoUpdate) showStatus(getTranslation("update_available_pattern", { version: info.version }));
    } catch (error) {
      console.error("Update check failed", error);
      if (announceNoUpdate) showStatus(getTranslation("update_check_error_pattern", { error: String(error) }), true);
    } finally {
      if (checkButton) checkButton.disabled = false;
    }
  }

  async function installAvailableUpdate() {
    if (!updateAvailable) {
      await checkForUpdates(true);
      if (!updateAvailable) return;
    }
    try {
      showStatus(getTranslation("update_installing"));
      await invoke("install_app_update");
      showStatus(getTranslation("update_installed_restarting"));
      await invoke("relaunch_app");
    } catch (error) {
      console.error("Update installation failed", error);
      showStatus(getTranslation("update_install_error_open_release"), true);
      invoke("open_url", {
        url: "https://github.com/malashkadev/aura/releases/latest"
      }).catch((openError) => console.error("Failed to open release page", openError));
    }
  }

  const checkUpdatesButton = document.getElementById("btn-check-updates");
  if (checkUpdatesButton) {
    checkUpdatesButton.addEventListener("click", () => checkForUpdates(true));
  }
  const updateBadge = document.getElementById("update-badge");
  if (updateBadge) {
    updateBadge.addEventListener("click", installAvailableUpdate);
  }
  // ---- Custom dropdown panels ----
  // WebView2 renders <select> options as an unstyleable OS popup, so every
  // .custom-select opens a styled panel instead. Native semantics are kept
  // (value + "change" event), so existing handlers work unchanged.
  let selectPanelEl = null;
  let selectPanelOwner = null;
  let selectPanelItem = -1;

  function closeSelectPanel() {
    if (!selectPanelEl) return;
    if (selectPanelOwner) {
      selectPanelOwner.setAttribute("aria-expanded", "false");
    }
    selectPanelEl.remove();
    selectPanelEl = null;
    selectPanelOwner = null;
    selectPanelItem = -1;
  }

  function refreshSelectHighlight(item) {
    selectPanelEl.querySelectorAll(".cs-option.active").forEach((option) => {
      option.classList.remove("active");
    });
    if (item) {
      item.classList.add("active");
      item.scrollIntoView({ block: "nearest" });
    }
  }

  function stepSelectHighlight(select, direction) {
    const itemEls = Array.from(selectPanelEl.querySelectorAll(".cs-option"));
    if (!itemEls.length) return;
    let index = selectPanelItem;
    if (index < 0 || index >= itemEls.length) {
      index = select.selectedIndex >= 0 ? select.selectedIndex : 0;
    }
    let next = index;
    for (let stepCount = 0; stepCount < itemEls.length; stepCount += 1) {
      next = (next + direction + itemEls.length) % itemEls.length;
      if (!itemEls[next].classList.contains("disabled")) {
        selectPanelItem = next;
        refreshSelectHighlight(itemEls[next]);
        return;
      }
    }
  }

  function jumpSelectHighlight(select, position) {
    const itemEls = Array.from(selectPanelEl.querySelectorAll(".cs-option"));
    const isFirst = position === "start";
    const bounds = isFirst ? itemEls : itemEls.slice().reverse();
    for (const item of bounds) {
      if (!item.classList.contains("disabled")) {
        selectPanelItem = itemEls.indexOf(item);
        refreshSelectHighlight(item);
        return;
      }
    }
  }

  function commitSelectOption(select, item) {
    if (!item || item.classList.contains("disabled")) return;
    if (select.value !== item.dataset.value) {
      select.value = item.dataset.value;
      select.dispatchEvent(new Event("change", { bubbles: true }));
    }
    closeSelectPanel();
  }

  function repositionOpenSelectPanel() {
    if (!selectPanelEl || !selectPanelOwner) return;
    const rect = selectPanelOwner.getBoundingClientRect();
    if (rect.width < 1 || rect.height < 1 || rect.bottom < 0 || rect.top > window.innerHeight) {
      // The select's page has scrolled out of view: the panel must not
      // float detached on screen, so it closes instead.
      closeSelectPanel();
      return;
    }
    placeSelectPanel(selectPanelOwner, selectPanelEl);
  }

  function placeSelectPanel(select, panel) {
    const rect = select.getBoundingClientRect();
    // Real, post-layout height: an estimated height (options x 40px) made
    // the panel jump far off for tall lists capped by max-height.
    const width = Math.min(Math.max(rect.width, 180), window.innerWidth - 16);
    panel.style.width = `${width}px`;
    panel.style.left = `${Math.max(8, Math.min(rect.left, window.innerWidth - width - 8))}px`;
    const height = panel.getBoundingClientRect().height;
    const fitsBelow = rect.bottom + 6 + height <= window.innerHeight;
    panel.style.top = fitsBelow
      ? `${rect.bottom + 6}px`
      : `${Math.max(8, rect.top - height - 6)}px`;
  }

  function openSelectPanel(select) {
    closeSelectPanel();
    const panel = document.createElement("div");
    panel.className = "custom-select-panel";
    panel.setAttribute("role", "listbox");

    let selectedIndex = -1;
    Array.from(select.options).forEach((option, index) => {
      const item = document.createElement("div");
      item.className = "cs-option";
      if (option.disabled) item.classList.add("disabled");
      if (option.selected) {
        item.classList.add("selected");
        selectedIndex = index;
      }
      item.dataset.value = option.value;
      item.textContent = option.textContent;
      item.setAttribute("role", "option");
      item.setAttribute("aria-selected", String(option.selected));
      item.addEventListener("mouseenter", () => {
        selectPanelItem = index;
        refreshSelectHighlight(item);
      });
      item.addEventListener("click", () => commitSelectOption(select, item));
      panel.appendChild(item);
    });

    document.body.appendChild(panel);
    placeSelectPanel(select, panel);
    selectPanelEl = panel;
    selectPanelOwner = select;
    select.setAttribute("aria-expanded", "true");
    if (selectedIndex >= 0 && !panel.children[selectedIndex].classList.contains("disabled")) {
      selectPanelItem = selectedIndex;
      refreshSelectHighlight(panel.children[selectedIndex]);
    }
  }

  function onSelectKeydown(event) {
    const select = event.currentTarget;
    if (selectPanelOwner !== select) {
      if (["ArrowDown", "ArrowUp", "Enter", " ", "Spacebar"].includes(event.key)) {
        event.preventDefault();
        openSelectPanel(select);
      }
      return;
    }
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        stepSelectHighlight(select, 1);
        break;
      case "ArrowUp":
        event.preventDefault();
        stepSelectHighlight(select, -1);
        break;
      case "Home":
        event.preventDefault();
        jumpSelectHighlight(select, "first");
        break;
      case "End":
        event.preventDefault();
        jumpSelectHighlight(select, "last");
        break;
      case "Enter":
      case " ":
      case "Spacebar": {
        event.preventDefault();
        const active = selectPanelEl.querySelector(".cs-option.active") ||
          selectPanelEl.querySelector(".cs-option.selected");
        commitSelectOption(select, active);
        break;
      }
      case "Escape":
        event.preventDefault();
        closeSelectPanel();
        break;
      default:
        break;
    }
  }

  document.querySelectorAll("select.custom-select").forEach((select) => {
    select.setAttribute("aria-haspopup", "listbox");
    select.setAttribute("aria-expanded", "false");
    select.addEventListener("mousedown", (event) => {
      event.preventDefault();
    });
    select.addEventListener("click", () => {
      if (selectPanelOwner === select) {
        closeSelectPanel();
        return;
      }
      select.focus();
      openSelectPanel(select);
    });
    select.addEventListener("keydown", onSelectKeydown);
  });
  document.addEventListener("pointerdown", (event) => {
    if (!selectPanelEl) return;
    if (event.target === selectPanelOwner || selectPanelEl.contains(event.target)) return;
    closeSelectPanel();
  });
    document.addEventListener("scroll", repositionOpenSelectPanel, true);
  window.addEventListener("resize", () => closeSelectPanel());

  // Initialize UI language and Settings
  (async () => {
    let settings = null;
    try {
      settings = await invoke("get_settings");
    } catch (err) {
      console.error(err);
    }

    let savedUiLang = localStorage.getItem("aura_ui_lang");
    if (savedUiLang === null) {
      savedUiLang = localStorage.getItem("ui-language");
    }

    const supportedLangs = ["ru", "en", "de", "es", "fr", "it", "zh", "pt", "tr"];
    if (savedUiLang === null || !supportedLangs.includes(savedUiLang)) {
      if (settings && settings.language && supportedLangs.includes(settings.language)) {
        savedUiLang = settings.language;
      } else {
        savedUiLang = "ru";
      }
    }

    // UI Language Selector Setup
    const selectUiLang = document.getElementById("select-ui-lang");
    if (selectUiLang) {
      selectUiLang.value = savedUiLang;
      
      selectUiLang.addEventListener("change", (e) => {
        const selectedLang = e.target.value;
        localStorage.setItem("aura_ui_lang", selectedLang);
        localStorage.setItem("ui-language", selectedLang);
        applyLanguage(selectedLang);
      });
    }
    
    // Apply initial language choice outside the if block so translations initialize even if #select-ui-lang is missing
    applyLanguage(savedUiLang);

    initSelectPanels();

    // Initialize Settings
    await loadSettings(settings);

    if (settings?.automatic_update_checks) {
      await checkForUpdates(false);
    }

  })();
});
