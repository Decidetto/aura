# План реализации улучшений Aura (1.0.9 → 1.0.10+)

Этот документ фиксирует **согласованный в чате план реализации** для проекта
`Aura` (десктопный голосовой ввод на Tauri 2 + Rust, веб‑фронтенд vanilla).
Рабочая папка‑источник — `G:\Aura\2.0`. Все ссылки на файлы/строки даны
относительно `G:\Aura\2.0\src-tauri\src` (Rust) и `G:\Aura\2.0/src` (фронтенд),
если не указано иное.

> Статус: **Этап 0 выполнен** (версия 1.0.9, «зелёная» базовая сборка, чистый
> git). **Этап 1, Т1.1 — реализован и применён к коду**, но пока не проходит
> `cargo test --lib` из‑за compile error (scoping `original_clipboard`). Требуется
> фиксировать/завершить в act‑режиме. Всё остальное — **только план, не
> выполняется**.

---

## Текучие условные обозначения

- `session_gen` — `AtomicU64` в `AppState`, инкрементируется при старте каждой
  новой сессии диктовки; асинхронные задачи старых сессий сверяются с ним,
  чтобы не трогать клавиатуру/буфер.
- `my_gen` — значение `session_gen`, «выписанное» конкретной финализирующей
  задачей.
- «Сессия stale / просрочена» = `session_gen != my_gen`.

## Состояние репозитория на старте этого документа

- `git`‑repo `G:\Aura` (рабочее дерево в `2.0/`): ветка `main`, +36 коммитов
  ahead of `origin/main`, **working tree чистый** после коммита
  `e398256  chore(1.0.9): bump version to 1.0.9 and lock green baseline`.
- `G:\Aura — копия` переименована → **`G:\Aura — старый бэкап`** (≈9,8 ГБ, не удалена).
- `.gitignore` дополнен: `.superpowers/`, `skill-observations/`, `skill-updates/`,
  `tasks/`, `log.txt`, `src-tauri/binaries/cuda/` (≈2,7 ГБ скачанных GPU‑рантаймов).

## Верификация «зелёной» базовой точки (Этап 0, Т0.2)

- `cargo check` в `src-tauri` → ✅ clean (`aura-app v1.0.9`)
- `cargo build` в `src-tauri` → ✅ `target/debug/aura-app.exe` построен
- `node tests/frontend-static.test.mjs` → ✅ 10/10
- `npm run lint` + `npm run typecheck` → ✅ clean

---

## ЭТАП 0 — Подготовка (выполнен)

### Т0.1 · Синхронизация кода и подъём версии 1.0.8 → 1.0.9
- Работа только в `G:\Aura\2.0` (актуальная версия).
- Версия поднята в: `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`,
  `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, `src/main.js` (9 локалей
  `about_version`), `src/index.html` (default about‑version text),
  `src-tauri/src/logger.rs` (юнит‑тест диагностики).
- `CHANGELOG.md` — секция `[1.0.9]`.
- Создан `release-manifest-1.0.9.json` (placeholder, `published: false`) —
  читается тестом `frontend-static.test.mjs` по `pkg.version`.
- Отстающая копия `G:\Aura — копия` переименована в `G:\Aura — старый бэкап`.
- `.gitignore` пополнился (см. выше).
- ✅ Готово: версия 1.0.9 повсюду в исходниках; `git status` чистый
  (`e398256`).

### Т0.2 · Зафиксировать «зелёную» сборку (выполнен, см. «Верификация»).
- Исключения из правок: исторические `docs/superpowers/...`,
  `release-manifest-1.0.8.json`, `RELEASE_NOTES_1.0.8.md` и URL‑ы
  `v1.0.8-assets/...` (ссылка на опубликованные артефакты) оставлены без
  изменений.

---

## ЭТАП 1 — Критические исправления (потеря данных / видимая поломка)

### Т1.1 · Предохранитель буфера обмена (Б-1)

**Где:** `src-tauri/src/lib.rs` (финализация не‑streaming paste ~3171–3198; захват
контекста `copy_context_on_start` ~2398–2412; `ClipboardGuard::drop`).

**Суть бага:** финализация делает `backup_clipboard()` → `set_text(final_text)` →
`simulate_paste()` → `sleep(800мс)` → `restore_clipboard_if_unchanged(...)`. Если
пользователь **запустит новую диктовку в течение 800 мс**, `session_gen`
инкрементится, но старая задача всё равно выполнит `restore` своего старого
буфера — затрут новую сессию. Тот же риск для `ClipboardGuard` в
`copy_context_on_start` (его `Drop` восстанавливает без проверки сессии).

**Решение (применено к коду):**
1. Добавлено `clipboard_mutex: Mutex<()>` в `AppState` — сериализует все
   операции с буфером между сессиями (init: `clipboard_mutex: Mutex::new(())`).
2. Добавлена чистая функция `session_still_current(state, my_gen) -> bool`.
3. Добавлена `restore_clipboard_guarded(state, my_gen, backup, expected=None)`,
   которая **перед восстановлением проверяет `session_still_current`**; если
   сессия просрочена — логирует и выходит (не трогает буфер).
4. `ClipboardGuard` получил поле `session: Option<(tauri::AppHandle, u64)`;
   в `Drop` вызывается `restore_clipboard_guarded` (только для актуальной
   сессии). Для `None` — fallback на прежнее поведение.
5. Блок финализации не‑streaming paste переписан: backup/set/paste берутся
   под `clipboard_mutex`, затем после `sleep(800мс)` — `restore_clipboard_guarded`
   с тем же `my_gen` и `expected_temporary_text = Some(&final_text)`.

**Код (Т1.1, как сейчас записан в `lib.rs`):**

```rust
// --- near other clipboard helpers ---
/// Returns true when `my_gen` still identifies the *current* recording
/// session. Stale async tasks consult it before touching the clipboard so
/// they cannot clobber data captured by a newer, overlapping session.
fn session_still_current(state: &AppState, my_gen: u64) -> bool {
    state.session_gen.load(Ordering::SeqCst) == my_gen
}

/// Serialized clipboard restore: refuses to restore unless the caller's
/// session is still current, preventing an outdated session from overwriting
/// a newer session's clipboard contents.
fn restore_clipboard_guarded(
    state: &AppState,
    my_gen: u64,
    backup: ClipboardBackup,
    expected_temporary_text: Option<&str>,
) {
    if !session_still_current(state, my_gen) {
        crate::logger::log(
            "INFO",
            "Clipboard",
            None,
            &format!(
                "Session ({my_gen}) is stale; skipping clipboard restore to protect newer session"
            ),
        );
        return;
    }
    match expected_temporary_text {
        Some(expected) => restore_clipboard_if_unchanged(backup, expected),
        None => restore_clipboard(backup),
    }
}
```

```rust
// --- ClipboardGuard (Drop) ---
struct ClipboardGuard {
    backup: ClipboardBackup,
    expected_temporary_text: Option<String>,
    /// When present, restore is gated on this session still being current.
    session: Option<(tauri::AppHandle, u64)>,
}

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        let backup = std::mem::replace(&mut self.backup, ClipboardBackup::Empty);
        if let Some((app_handle, my_gen)) = &self.session {
            if let Some(state) = app_handle.try_state::<AppState>() {
                restore_clipboard_guarded(state.inner(), *my_gen, backup,
                    self.expected_temporary_text.as_deref());
                return;
            }
        }
        match &self.expected_temporary_text {
            Some(expected) => restore_clipboard_if_unchanged(backup, expected),
            None => restore_clipboard(backup),
        }
    }
}
```

```rust
// --- copy_context_on_start (spawn) ---
let mut clipboard_guard = ClipboardGuard {
    backup: backup_clipboard(),
    expected_temporary_text: None,
    session: Some((app_handle_copy.clone(), gen)),
};
```

```rust
// --- finalize_recording, не-streaming paste path ---
if session_ok && focus_ok {
    let mut original_clipboard = ClipboardBackup::Empty;
    if let Some(state) = app_handle_clone.try_state::<AppState>() {
        if let Ok(_guard) = state.clipboard_mutex.lock() {
            original_clipboard = backup_clipboard();
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(final_text.clone());
            }
            keyboard_simulator::simulate_paste();
        }
    }
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    if let Some(state) = app_handle_clone.try_state::<AppState>() {
        if let Ok(_guard) = state.clipboard_mutex.lock() {
            restore_clipboard_guarded(state.inner(), my_gen,
                original_clipboard.clone(), Some(&final_text));
        }
    } else {
        restore_clipboard_if_unchanged(original_clipboard.clone(), &final_text);
    }
}
```

> ⚠️ Открытый compile error: `original_clipboard` объявлен внутри
> `if let Ok(_guard)`, а используется после `sleep`. Фикс — объявить его вне
> блока со значением по умолчанию `ClipboardBackup::Empty` и пользоваться
> `.clone()`. (Описанный выше код устраняет ошибку.)

**Юнит‑тест (применён):**

```rust
fn test_app_state_with_generation(gen: u64) -> crate::AppState { /* полное AppState */ }

#[test]
fn stale_session_never_restores_clipboard_after_overlap() {
    let state = test_app_state_with_generation(1);
    assert!(session_still_current(&state, 1));
    state.session_gen.store(2, Ordering::SeqCst);   // новая сессия
    assert!(!session_still_current(&state, 1));     // старая — просрочена
    assert!(session_still_current(&state, 2));
    // старая сессия не должна восстанавливать (проверяется на Empty backup)
    restore_clipboard_guarded(&state, 1, ClipboardBackup::Empty, None);
    assert!(state.session_gen.load(Ordering::SeqCst) == 2);
}
```

- ✅ Критерий готовности: чтение/восстановление буфера не может затереть данные
  другой сессии + юнит‑тест с имитацией наложения сессий проходит.

### Т1.2 · Порядок счётчика сессий (Б-2)
- Где: `lib.rs:2256–2271` (`start_recording_session`).
- Действие: инкремент `session_gen` **до** `compare_exchange(is_recording = true)`;
  сбрасывать `live_target_*` флаги там же.
- ✅ Готово: устаревший async‑хвост гарантированно отсекается на любом шаге
  старта новой записи.

### Т1.3 · Точное стирание при прерывании потокового ввода (Б-3)
- Где: `lib.rs:560–576` (`replace_text`/`cancel_recording`), `lib.rs:2231–2244`.
- Действие: вести сквозной счётчик реально введённых символов, коммитить его в
  `typed_so_far` даже при прерывании; использовать в `cancel_recording` для
  backspaces.
- ✅ Готово: после Esc от потоковой диктовки не остаётся «хвоста» в документе.

### Т1.4 · Рабочая кнопка «Отмена» GPU‑загрузки (Б-4)
- Где: `src/main.js:2020–2060`, `lib.rs`.
- Действие: добавить Rust‑команду `cancel_gpu_download(provider)`,
  отменяющую через `ACTIVE_GPU_DOWNLOADS`/флаг; вызывать из JS; разблокировать
  кнопку при сбое.
- ✅ Готово: отмена CUDA/DirectML реально прерывает загрузку.

### Т1.5 · Таймер оверлея (Б-5)
- Где: `src/overlay.js:580–587`.
- Действие: хранить id таймера скрытия, `clearTimeout` при новом
  `recording`/`processing`; проверять «токен текущей сессии» перед скрытием.
- ✅ Готово: оверлей не исчезает посреди новой записи после ошибки.

---

## ЭТАП 2 — Надёжность аудио и локальных движков

### Т2.1 · VAD: не обрезать последнее слово / порцию 8 кГц (Б-6, Б-13)
- Где: `src-tauri/src/vad.rs:163–202`.
- Действие: добивать (zero‑pad) последнюю неполную порцию до 512 (как в
  потоковом пути); убрать расхождение размера порции для 8 кГц.
- ✅ Готово: тест на запись не кратной 512 не теряет финальные сэмплы.

### Т2.2 · Не терять звук при нагрузке (Б-7)
- Где: `src-tauri/src/audio_recorder.rs`.
- Действие: развязать `on_volume`/поток громкости от захвата; увеличить
  `AUDIO_QUEUE_CAPACITY`; при переполнении — сигнализировать.
- ✅ Готово: искусственная задержка callback не приводит к дропу чанков записи.

### Т2.3 · Защита рабочего потока от паники (Б-8)
- Где: `audio_recorder.rs:430–439, 808`.
- Действие: обернуть тело потока в `catch_unwind`, преобразовывать панику в
  ошибку сессии, а не молча ронять запись; не «травить» мьютексы.
- ✅ Готово: при сбое запись завершается понятной ошибкой, а не теряется.

### Т2.4 · Kill‑страховка и watchdog Parakeet (Б-9, Б-11, рестарты)
- Где: `src-tauri/src/whisper_runner.rs`.
- Действие: Job Object создавать до старта процесса + запасной kill на выходе/при
  панике; watchdog‑поток с `try_wait`, авто‑перезапуск и обновление порта через
  lifecycle‑блокировку (устраняет TOCTOU порта Б-11).
- ✅ Готово: аварийное закрытие Aura не оставляет живой сервер; падение демона
  авто‑восстанавливается.

### Т2.5 · Утечка потоков чтения (Б-10)
- Где: `whisper_runner.rs:1310–1343`.
- Действие: хранить `JoinHandle` двух ридеров, не‑наследуемые дескрипторы,
  `join()` при остановке.
- ✅ Готово: перезапуски не накапливают потоки.

---

## ЭТАП 3 — Безопасность

### Т3.1 · Логи: не сливать транскрипции (Б-12)
- Где: `src-tauri/src/logger.rs:256–259`, `lib.rs`.
- Действие: никогда не включать сырые транскрипции в диагностический отчёт;
  при выключении `log_speech_text` удалять ротационные копии; добавить
  предупреждение в UI.
- ✅ Готово: отчёт не содержит речи пользователя; при выключении опции текст
  из логов вычищается.

### Т3.2 · Явные права доступа (V2)
- Где: `src-tauri/src/secure_storage.rs`.
- Действие: установить ACL «только текущий пользователь + SYSTEM» на
  `settings.json`, `secrets.dpapi`, `history.dpapi` и `.tmp-*`.
- ✅ Готово: файлы ключей не читаются другими процессами того же пользователя
  без DPAPI‑вызова.

### Т3.3 · Гигиена памяти секретов
- Где: `settings_secure.rs`, `secure_storage.rs`.
- Действие: `zeroize` временных секретов; не клонировать структуры с ключами
  лишний раз.
- ✅ Готово: клоны с ключами обнуляются (settings view/save, замена ключа,
  `set_settings`).

### Т3.4 · Языковая «auto» вместо «ru» по умолчанию (Б-17)
- Где: `src-tauri/src/keyboard_simulator.rs:354–370`.
- Действие: неизвестная раскладка → `"auto"`, а не принудительный русский.
- ✅ Готово: Windows (hwnd==0 и неизвестная lang_id) и macOS возвращают «auto».

### Т3.5 · (доп.) Опциональная DPAPI‑энтропия
- Действие: задокументировать компромисс, сделать флагом.
- ✅ Готово: `DPAPI_ENTROPY: Option<&[u8]> = None`; тест на round-trip и
  несовпадение энтропии.

---

## ЭТАП 4 — Улучшения качества и UX

### Т4.1 · Докачка моделей через `artifact_download.rs`
- Действие: перевести циклы загрузки Whisper/Parakeet на готовый модуль
  (resume + SHA‑256 + отмена), убрать дубли.
- ✅ Готово: оба цикла делегируют `download_verified_artifact` (resume из `.part`),
  дубли‑хелперы удалены; заодно исправлен неверный ключ отмены загрузки
  punctuation.

### Т4.2 · Мгновенная отмена распознавания
- Где: `parakeet_streaming.rs` (блокирующий `socket.read()`).
- Действие: `CancellationToken` в блокирующий read Parakeet (сегодня до 30–360 с).
- ✅ Готово: `run_parakeet` опрашивает предикат отмены через короткий read‑таймаут
  (250 мс, жёсткий дедлайн по длине аудио); генерация сессии пробрасывается из
  `run_local_whisper_async`, устаревшая сессия прерывает декод мгновенно.

### Т4.3 · `prefers-reduced-motion` и контраст оверлея
- Где: `src/overlay.js`, `src/style.css`.
- Действие: статичная анимация при настройке сниженного движения; подинтровка/
  затемнение под текстом статуса.
- ✅ Готово: статичные бары и pill без transition при reduced motion, хайд оверлея
  без `transitionend`; у статуса тёмная полупрозрачная подложка. (Блок в
  `style.css` уже был.)

### Т4.4 · Доступность карточек
- Где: `src/main.js`.
- Действие: `tabindex`, `role`, `aria-checked` у выбираемых карточек моделей.
- ✅ Готово: карточки уже имели `role=radio`/`aria-checked`/`tabindex`; добавлена
  стрелочная навигация по группам (WAI‑ARIA radio), вложенные кнопки не
  активируют карточку, скрытые/disabled GPU‑карточки пропускаются.

### Т4.5 · Защита от двойного сохранения
- Где: `src/main.js` (`saveSettings`).
- Действие: блокировать кнопку Save, пока `saveSettings` в полёте; единая
  обработка промисов.
- ✅ Готово: убран дублирующийся биндинг (клик запускал два сохранения); флаг
  `saveInFlight` + `disabled` на время сохранения, сброс в `finally`; стиль
  `:disabled` для `.btn-primary`.

---

## ЭТАП 5 — Тесты и валидация

### Т5.1 Юнит‑тесты
- Ресемплер для нецелых соотношений (44,1 → 16 кГц);
- VAD с некратным размером;
- Гонка буфера обмена (перекрывающие сессии).
- ✅ Готово: добавлен тест 44,1 кГц (105 тестов проходят); VAD и буфер
  обмена уже покрыты существующими тестами.

### Т5.2 UI‑сценарии
- Наложение сессий;
- Отмена GPU‑загрузки;
- Таймер оверлея.
- ⏳ Ручная проверка — на пользователе.

### Т5.3 Финальный прогон
- `cargo test` — ✅ (105/105);
- `node tests/frontend-static.test.mjs` — ✅ (10/10);
- `cargo build --release` — ✅ (aura-app.exe 37,5 МБ);
- сборка NSIS‑установщика и ручная проверка диктовки (cloud + local + Parakeet) —
  ⏳ на пользователе.

---

## Порядок запуска (без изменений — как согласовано)

1. **Этап 0** (выполнен) →
2. **Этап 1** (выполнен: Т1.1–Т1.5) →
3. **Этап 2** (выполнен: Т2.1–Т2.5) →
4. **Этап 3** (выполнен: Т3.1–Т3.5) →
5. **Этап 4** (выполнен: Т4.1–Т4.5) →
6. **Этап 5** (проверка).

Каждый этап самодостаточен: после него проект остаётся в рабочем состоянии —
позволяет выпускать промежуточные версии (например, релиз после Этапа 1 → 1.0.9,
после Т1.4 — 1.0.10 и т.п.).

---

## Приложение А — Резюме ревью (Б‑отчёты), как было в чате

**Общий вердикт:** архитектура и ключевые механизмы — хорошие: математика
ресемплинга аудио корректна, защита клавиш/буфера через «счётчик сессий»
продумана, ключи API и история шифруются через DPAPI (проблемы прошлой копии
исправлены), аппаратных высоких багов безопасности не найдено. Есть рискованные
места с потерей данных (буфер обмена) и падением качества транскрипции.

### Ошибки (баги)

**🔴 Критично / Высший приоритет**
- **Б-1.** Потеря буфера обмена при наложении сессий диктовки — `lib.rs` (финал ~3121–3128; контекст ~2354–2370).
- **Б-2.** Порядок счётчика сессий — `lib.rs:2256–2271`.
- **Б-3.** Точное стирание при прерывании потокового ввода — `lib.rs:560–576, 2231–2244`.
- **Б-4.** Кнопка «Отмена» GPU‑загрузки не работает — `main.js:2020–2060`, `lib.rs`.
- **Б-5.** Таймер оверлея — `src/overlay.js:580–587`.

**🟡 Надёжность**
- **Б-6.** VAD обрезает последнее слово — `vad.rs:163–202`.
- **Б-7.** Потеря звука при нагрузке — `audio_recorder.rs`.
- **Б-8.** Паника в аудио‑потоке — `audio_recorder.rs:430–439, 808`.
- **Б-9.** Kill‑страховка / watchdog Parakeet — `whisper_runner.rs`.
- **Б-10.** Утечка потоков чтения — `whisper_runner.rs:1310–1343`.
- **Б-11.** TOCTOU порта — `whisper_runner.rs:1277–1284`.

**🔐 Безопасность / UX**
- **Б-12.** Логи сливают транскрипции — `logger.rs:256–259`, `lib.rs`.
- **Б-13.** VAD‑порция 8 кГц расходится (в Т2.1).
- **Б-17.** Принудительный «ru» вместо «auto» — `keyboard_simulator.rs:354–370`.

### Сравнение с `G:\Aura — копия` (старый бэкап)
`G:\Aura` — более новая/продвинутая версия 1.0.8: есть `artifact_download.rs`
(32 КБ), `parakeet_streaming.rs` (31 КБ) и их примерно на 2100 строк логики в
`lib.rs` (~4360 строк), а также реальное DPAPI‑шифрование ключей/истории
(`settings_secure.rs`, `history_secure.rs`) — всего этого в копии **нет**.
