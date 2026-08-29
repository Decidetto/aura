# Aura 2.0 — Website Design System & Architecture Specification

> **Version**: 2.4.0  
> **Status**: Production Ready  
> **Aesthetic Axis**: Tactical Obsidian Glass & Acoustic Kinetic Plasma  
> **Core Philosophy**: Zero Shadows & Glows · Zero AI Slop · Emil Kowalski Tactile Physics · Single Typeface Harmony · High-Contrast Benefit Scanning · Strict Typographic Rhythm  

---

## 1. Философия и дизайн-манифест

Дизайн нового сайта **Aura** построен на принципах бескомпромиссного премиального продуктового инжиниринга (в духе *Apple, Linear, Teenage Engineering, Nothing*):
1. **Zero Shadows & Zero Glows Rule (Канон абсолютного отсутствия теней и свечений)**: Ни у каких элементов, карточек, кнопок, иконок, инпутов, тултипов, модалок или контейнеров **не должно быть ни внешних теней (`box-shadow`, `drop-shadow`), ни цветных/неоновых свечений (`glow`, `--accent-glow`, `text-shadow`)** при **любом из состояний** (состояние покоя, наведение `:hover`, активное `:active`, фокус `:focus`, пульсация). Допустим исключительно деликатный внутренний верхний фасеточный блик в 1px (`inset 0 1px 0 rgba(255, 255, 255, ...)`), обозначающий физическую грань обсидианового стекла.
2. **Zero AI Slop**: Никаких фиолетово-синих абстрактных градиентов, летающих 3D-иконок, хаотичного шума или шаблонных карточек с прыжками вверх (`translateY`).
3. **Unified Obsidian Glass Surface**: Все блоки, карточки, бенто-сетка и модули сайта принадлежат к единой семье темного обсидианового стекла (`#030305` + `rgba(20, 20, 26, 0.65)`) с одинаковым радиусом скругления (`20px`), единым тонким нейтральным контуром (`1px solid rgba(255, 255, 255, 0.08)`) и фасеточным верхним световым бликом.
4. **Single Typeface Harmony**: Отказ от разнобоя шрифтов в пользу единого гармоничного неогротеска **`Onest`** с нативной поддержкой 100% кириллических и латинских глифов.
5. **Focused Acoustic Energy**: Фирменный оранжевый акцент (`#FF4200`) используется строго функционально: для фокусных CTA-кнопок, звуковых волн эквалайзера и шейдерных акустических горизонтов в Hero (без неонового мыла и ореолов).

---

## 2. Цветовая палитра и Design Tokens

### 2.1. Базовая палитра (Color Tokens)

| Токен | Значение | Описание и назначение |
| :--- | :--- | :--- |
| `--bg` | `#030305` | Глубокий бархатный космический обсидиан (основной фон страницы) |
| `--surface` | `#0a0a0c` | Темная подложка для сегментов и внутренних панелей |
| `--surface-card` | `rgba(20, 20, 26, 0.65)` | Верхняя граница градиента стеклянных карточек |
| `--surface-card-bottom`| `rgba(10, 10, 14, 0.85)` | Нижняя граница градиента стеклянных карточек |
| `--surface-glass` | `rgba(16, 16, 20, 0.70)` | Полупрозрачное стекло навигационной панели |
| `--accent` | `#ff4200` | Фирменное пламя Aura (CTA-кнопки, волны звука, активные маркеры) |
| `--accent-hover` | `#e63b00` | Состояние наведения для основных кнопок |
| `--accent-glow` | `none` | **Отключено по канону**: нулевое свечение во всех состояниях |
| `--shadow-sm` | `none` | **Отключено по канону**: нулевые внешние тени |
| `--shadow-md` | `none` | **Отключено по канону**: нулевые внешние тени |
| `--shadow-lg` | `none` | **Отключено по канону**: нулевые внешние тени |
| `--border` | `rgba(255, 255, 255, 0.08)` | Нейтральная тонкая граница карточек и разделителей |
| `--border-hover` | `rgba(255, 255, 255, 0.16)` | Контур при наведении |
| `--text-primary` | `#f5f7fa` | Заголовки, жирный текст и основной контрастный контент |
| `--text-secondary` | `#9ba6b3` | Поясняющие абзацы и подписи |
| `--text-muted` | `#64748b` | Метаданные, лейблы и второстепенные сноски |

```css
:root {
  --bg: #030305;
  --surface: #0a0a0c;
  --surface-glass: rgba(16, 16, 20, 0.7);
  --surface-card: rgba(20, 20, 26, 0.65);
  --border: rgba(255, 255, 255, 0.08);
  --border-hover: rgba(255, 255, 255, 0.16);
  --accent: #ff4200;
  --accent-hover: #e63b00;
  --accent-glow: none;
  --shadow-sm: none;
  --shadow-md: none;
  --shadow-lg: none;
  --text-primary: #f5f7fa;
  --text-secondary: #9ba6b3;
  --text-muted: #64748b;

  --font-display: 'Onest', system-ui, -apple-system, sans-serif;
  --font-body: 'Onest', system-ui, -apple-system, sans-serif;
  --font-mono: 'Onest', system-ui, -apple-system, sans-serif;

  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 20px;
  --radius-full: 9999px;

  --ease-out: cubic-bezier(0.23, 1, 0.32, 1);
  --ease-spring: cubic-bezier(0.16, 1, 0.3, 1);
  --transition-fast: 160ms var(--ease-out);
  --transition-normal: 240ms var(--ease-out);
}
```

---

## 3. Типографика и правила верстки текста (Single Typeface Harmony)

Вся типографика сайта стандартизирована на гарнитуре **`Onest`** (Google Fonts).

### 3.1. Иерархия начертаний и размеров

| Уровень | Тег / Класс | Размер (rem/vw) | Вес (Weight) | Letter-spacing | Line-height | Назначение |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Hero Title** | `.hero-title` | `clamp(2.35rem, 4.1vw, 3.75rem)` | 800 (ExtraBold) | `-0.03em` | `1.12` | Главный слоган первого экрана |
| **Section Title**| `.section-title` | `clamp(1.8rem, 3vw, 2.5rem)` | 700 (Bold) | `-0.02em` | `1.2` | Заголовки секций |
| **Card Title** | `.bento-card-title`, `.pipeline-title` | `1.25rem – 1.4rem` | 700 (Bold) | `-0.015em` | `1.25` | Заголовки внутри карточек |
| **Body Large** | `.hero-subtext` | `1.1rem` | 400 (Regular) | `0` | `1.6` | Лид-текст под заголовком Hero |
| **Body Regular**| `.section-desc`, `p` | `0.95rem – 1.05rem` | 400 (Regular) | `0` | `1.6` | Описания и абзацы |
| **Meta / Spec** | `.hero-trust-row`, `.spec-tag` | `0.85rem` | 500 (Medium) | `0.01em` | `1.6` | Преимущества, системные требования |
| **Badge / Tag** | `.version-badge`, `.lang-switch` | `0.75rem – 0.8rem` | 600 (SemiBold) | `0.02em` | `1.0` | Версия софта, переключатель языков |
| **Micro Label** | `.engine-feat-label` | `0.75rem` | 600 (SemiBold) | `0.04em` | `1.0` | Капслочные технические лейблы |

### 3.2. Инвариант 2-строчного заголовка Hero (Desktop 2-Line Invariant)
Главный заголовок в Hero-секции **всегда** должен отображаться ровно в **2 строки** на всех десктопных разрешениях (от 1024px до 4K), не распадаясь на 3–4 строки:
* **HTML-разметка**:
  ```html
  <h1 class="hero-title">
    <span class="hero-title-line">Печатайте голосом.</span>
    <span class="hero-title-line">Мгновенно и&nbsp;везде.</span>
  </h1>
  ```
  *(Для английской версии: `Type with your voice.` и `Instantly everywhere.`)*
* **CSS-правило**:
  ```css
  .hero-title-line {
    display: block;
    white-space: nowrap;
  }
  @media (max-width: 640px) {
    .hero-title-line {
      white-space: normal;
    }
  }
  ```
* **Пропорции сетки**: `.hero-grid { grid-template-columns: 1.18fr 1fr; }` — левая колонка гарантированно вмещает ширину заголовка с запасом.

### 3.3. Система сканирующего жирного выделения преимуществ (High-Contrast Value Hierarchy)
Для мгновенного считывания преимуществ при беглом взгляде внедрено правило повышенного контраста жирного начертания:
```css
strong, b {
  color: var(--text-primary); /* #f5f7fa (яркий белый против #9ba6b3 у обычного текста) */
  font-weight: 600;
}
```
* **Правило применения**: Выделять строго главные ценностные триггеры, а не случайные слова:
  - **Hero**: `<strong>Alt + V</strong>`, `<strong>сразу вставляют готовый текст</strong>`, `<strong>без задержек</strong>`.
  - **Принцип работы**: `<strong>не перехватывая системный фокус</strong>`, `<strong>прямо на вашем ПК</strong>`, `<strong>с расставленной пунктуацией</strong>`, `<strong>там, где стоит курсор</strong>`.
  - **Преимущества**: `<strong>не покидают компьютер</strong>`, `<strong>прямо под курсор</strong>`, `<strong>в любое активное окно Windows</strong>`, `<strong>обычном процессоре любого ПК</strong>`, `<strong>в один клик</strong>`, `<strong>без рекламы, скрытых подписок и ограничений</strong>`, `<strong>под лицензией AGPL-3.0</strong>`.
  - **CTA скачивания**: `<strong>со скоростью речи</strong>`.

### 3.4. Правило защиты от висячих предлогов («Типографический канон»)
Строгий запрет на висячие предлоги, союзы и частицы на концах строк:
1. **Неразрывные пробелы (`&nbsp;`)**: Все одно- и двухбуквенные предлоги/союзы (`и`, `в`, `на`, `с`, `к`, `о`, `у`, `за`, `по`, `из`, `от`, `до`, `не`, `без`, `со`, а в английском `in`, `on`, `at`, `to`, `by`, `of`, `for`, `and`, `or`, `with`) **обязательно** связываются с последующим словом через `&nbsp;`.
2. **Неделимые строки**: `Alt&nbsp;+&nbsp;V`, `Windows&nbsp;10&nbsp;/&nbsp;11`, `от&nbsp;4&nbsp;ГБ`, `AGPL-3.0`.
3. **Защита границ HTML-тегов (Tag Boundary Protection)**:
   При выделении жирным предлог должен находиться **внутри** тега `<strong>` вместе с ключевым словом:
   - ❌ Ошибка (Chromium ломает строку перед тегом): `с&nbsp;<strong>расставленной...</strong>`
   - ✅ Корректно: `<strong>с&nbsp;расставленной пунктуацией</strong>`, `<strong>в&nbsp;любое активное окно</strong>`, `<strong>под&nbsp;лицензией AGPL-3.0</strong>`.

---

## 4. Шейдерный движок фона (Acoustic WebGL Engine)

В фоне Hero-секции работает нативный аппаратный WebGL-шейдер (без внешних зависимостей) на полноэкранном холсте `<canvas id="aura-fluid-canvas">`.

### Ключевые алгоритмические особенности:
1. **Acoustic Tensor Waves (3D-гармоники звука)**:
   - Построен на базе 4-октавного фрактального шума (FBM) и интерполяции тригонометрических волн.
   - Плавная кинематика (`u_time * 0.2`) создает эффект медленно переливающихся глубоких звуковых волн.
2. **Left-side Text Shield (Защита читаемости текста)**:
   - В шейдере вычисляется градиентная маска по горизонтальной оси `p.x`:
     `textShield = smoothstep(-0.85, 0.25, p.x)`
   - Левая половина экрана (где расположены заголовок, описание, кнопки и плашки преимуществ) **гарантированно погружена в чистую бархатную обсидиановую темноту (`#020204`)**. Оранжевые энергетические волны живут справа — за окном демонстрации.
3. **Scroll Fade-Out (Плавное затухание в чистый черный фон)**:
   - При скролле страницы вниз прозрачность холста плавно падает до нуля:
     `opacity = max(0, min(1, 1 - (scrollY / (heroHeight * 0.45))))`
   - Под всеми остальными секциями (`#how-it-works`, `#features`, `#engines`, `#download`) фон становится абсолютно черным (`#030305`), не отвлекая от чтения.
4. **Оптимизация производительности**:
   - Автоматическая пауза анимации (`cancelAnimationFrame`) при неактивной вкладке браузера (`document.hidden`).
   - Пропуск вычислений шейдера, если страница прокручена ниже первого экрана (`scrollY > innerHeight * 1.1`).

---

## 5. Библиотека компонентов

### 5.1. Unified Obsidian Glass Card (Единая стеклянная карточка)
Все карточки на сайте (`.pipeline-card`, `.bento-card`, `.engine-matrix-wrap`, `.download-card`, `.comparison-table-wrap`) используют единый CSS-шаблон с **абсолютно нулевыми внешними тенями и свечениями**:

```css
.pipeline-card,
.bento-card,
.engine-matrix-wrap,
.download-card,
.comparison-table-wrap {
  background: linear-gradient(160deg, rgba(20, 20, 26, 0.65) 0%, rgba(10, 10, 14, 0.85) 100%);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: var(--radius-lg);
  position: relative;
  overflow: hidden;
  box-shadow: inset 0 1px 0 0 rgba(255, 255, 255, 0.05); /* Деликатный 1px верхний блик грани */
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  transition: border-color 0.25s var(--ease-out);
}

.pipeline-card:hover,
.bento-card:hover,
.engine-matrix-wrap:hover,
.download-card:hover {
  border-color: rgba(255, 255, 255, 0.16);
  box-shadow: inset 0 1px 0 0 rgba(255, 255, 255, 0.08);
}
```

### 5.2. Floating Segmented Control (Переключатель движков архитектуры)
Парящий контроллер в стиле *macOS / Linear*:
- Подложка: `background: rgba(0, 0, 0, 0.45); border: 1px solid rgba(255, 255, 255, 0.06); border-radius: 12px;`
- Кнопки: скругление `8px`, центрированный текст, иконка.
- Активный сегмент: `background: rgba(255, 255, 255, 0.08); border: 1px solid rgba(255, 255, 255, 0.12); box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.1);`. Внешние тени и свечения: **НОЛЬ**.

### 5.3. Voice Dictation Monolith (Демо-окно в Hero)
Интерактивный симулятор голосового набора:
- **Поверхность**: `linear-gradient(155deg, rgba(18, 18, 25, 0.72) 0%, rgba(8, 8, 12, 0.94) 100%)`, `backdrop-filter: blur(24px)`, `box-shadow: inset 0 1px 1px 0 rgba(255, 255, 255, 0.15);` (никаких фоновых черных теней и свечений вокруг окна).
- **Звуковая капсула (`.mock-overlay-pill`)**: Анимированный 10-полосный эквалайзер с задержками фаз звуковых волн.
- **Акустическая мембрана (`.ripple-circle`)**: 3 концентрические расширяющиеся звуковые волны, пульсирующие в момент диктовки (тонкий векторный контур без размытого неона).
- **Кинетическая смена токенов (`.kinetic-token`)**: Мягкое появление оранжевого цвета при наборе каждого нового слова с плавным переходом в чистый белый цвет (`color: var(--text-primary)`), без `text-shadow` и без неонового ореола.

### 5.4. Тактильные кнопки (Variant 4 Solid Primary + Variant A Pure Ghost Secondary)
- **Primary CTA (`.btn-primary`)**:
  - Градиент: `background: linear-gradient(180deg, #ff4e10 0%, #eb3b00 100%); color: #ffffff;`
  - Контур: `border: 1px solid rgba(255, 255, 255, 0.16);`
  - Hover: `background: linear-gradient(180deg, #ff5c22 0%, #f04000 100%); border-color: rgba(255, 255, 255, 0.28);`
  - Стрелка: `.dl-arrow { transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1); }` с мягким скольжением вниз на `2px` при наведении.
  - Нажатие (`:active`): `transform: scale(0.975) translateY(1px); background: linear-gradient(180deg, #e63e05 0%, #d83500 100%);`
  - Внешние тени / свечение / фасетки: **НОЛЬ** (чистый благородный цвет без AI-стекла, мыла и `--accent-glow`).
- **Secondary Ghost Actions (`.btn-secondary`, `.btn-github-action`, `.btn-icon-github`)**:
  - Прозрачный Ghost: `background-color: transparent; border: 1px solid rgba(255, 255, 255, 0.22); color: #f5f7fa;`
  - Причина: полное исключение сходства с тёмным телом контентных карточек (`.pipeline-card`, `.bento-card`).
  - Hover: `background-color: rgba(255, 255, 255, 0.08); border-color: rgba(255, 255, 255, 0.45); color: #ffffff;`
  - Нажатие (`:active`): `transform: scale(0.975) translateY(1px); background-color: rgba(255, 255, 255, 0.04);`
- **Плавающая кнопка наверх (`.back-to-top`)**:
  - `background-color: rgba(20, 20, 26, 0.85); border: 1px solid rgba(255, 255, 255, 0.12); backdrop-filter: blur(12px);` с мягким высветлением при hover.

### 5.5. Послойные 3D-микровзаимодействия (Tactile Layered 3D Icons)
Вместо шаблонных плоских векторных иконок в разделе «Принцип работы» (Pipeline) используется послойная псевдо-3D кинетика:
- **Шаг 1 («Глобальный хоткей»)**: Свитч механической клавиатуры. База переключателя (`keycap_base.png`, 56px) статична. Кейкап (`keycap_cap.png`, 50px) в покое сидит высоко на штоке (`top: -10px`), а при hover мягко утапливается на `3.5px` (`translateY(3.5px)`), не проваливаясь в корпус.
- **Шаг 2 («Локальный отклик»)**: Процессор на текстолите. В покое плата (`chip_base.png`) и чип (`chip_die.png`) образуют бесшовный монолит 1:1 (`62px`, `top: 6px; left: -2px;`). При hover плата остаётся неподвижной, а чип левитирует вверх (`translateY(-6px)`), эффектно открывая посадочное гнездо.
- **Шаг 3 («Печать под курсор»)**: Текстовый курсор и буква. В покое видна только центрированная каретка (`cursor_caret.png`, 46px). При hover каретка смещается вправо (`translateX(13px)`), а слева проявляется буква «А» (`cursor_letter_a.png`, `opacity: 1`, `translateX(-12px)`).
- **Zero Shadows & Zero Glows**: `filter: none;` — полный отказ от черных дроп-теней (`drop-shadow(...)`) и неоновых цветных ореолов. Чистая матовая физика материалов.

---

## 6. Международная поддержка (i18n & Cyrillic Alignment)

Сайт поддерживает две полноценные версии с идентичной структурой DOM:
* **Русская версия (`index_ru.html`)**: Основная посадочная страница для русскоязычной аудитории.
* **Английская версия (`index.html`)**: Международная версия.

Благодаря гарнитуре `Onest` все заголовки, знаки препинания, тире, сноски и технические термины (`CUDA`, `Whisper.cpp`, `AGPL-3.0`, `~250 ms`, `~250 мс`) имеют идентичную оптическую высоту и интерлиньяж на обоих языках.

---

## 7. Файловая структура сайта

```
g:/Aura/2.0/website/
├── index_ru.html       # Русская версия лендинга
├── index.html          # Английская версия лендинга
├── style.css           # Полная дизайн-система и стили (CSS Variables, Glass Surfaces, Responsive)
├── main.js             # Логика симулятора, WebGL-шейдер, скролл-фейд, табы, бенчмарк-анимация
├── DESIGN_SYSTEM.md    # Данная спецификация дизайн-системы
└── README.md           # Описание структуры и руководство по запуску
```
