# AGENTS.md

Гид для AI-агентов и разработчиков по проекту **music.xima** — офлайновый персональный
музыкальный плеер для Android.

Полная продуктовая спецификация — в [`promt.md`](./promt.md). Этот файл описывает
**как писать код**, а не что строить.

---

## 1. Что это за проект

Локальный музыкальный плеер под Android:

- полностью offline, без аккаунтов, серверов и сетевых запросов;
- музыка лежит в shared storage телефона, **не внутри APK**;
- воспроизведение — нативное (Media3/ExoPlayer), UI — WebView (Tauri 2 + SolidJS);
- фоновая музыка, экран блокировки, Bluetooth/гарнитура работают через `MediaSessionService`.

---

## 2. Стек

| Слой | Технологии |
| --- | --- |
| Оболочка | Tauri 2 (Android target) |
| UI | SolidJS, TypeScript (strict), Tailwind CSS v4 |
| UI-стили | **HeroUI v3 — `@heroui/styles`** (standalone CSS, framework-agnostic) |
| UI-поведение | `@kobalte/core` (headless, a11y), Lucide Icons |
| Core | Rust — `serde`, `thiserror`, `tokio` |
| БД | SQLite через `tauri-plugin-sql` (`sqlx`), FTS5 для поиска |
| Настройки | `tauri-plugin-store` |
| Аудио | Kotlin, AndroidX Media3, ExoPlayer, MediaSession, MediaSessionService |
| Доступ к файлам | MediaStore.Audio, Storage Access Framework (`ACTION_OPEN_DOCUMENT_TREE`) |
| Плагины Tauri | `plugin-sql`, `plugin-store`, `plugin-fs`, custom `tauri-plugin-player` |

### HeroUI v3 — как именно используем

HeroUI v3 (`@heroui/react`) — это **React**-библиотека. У нас Solid, поэтому:

- ставим **только `@heroui/styles`** — это чистый CSS поверх Tailwind v4, без JS-зависимостей;
- берём оттуда: CSS-переменные темы, OKLCH-палитру, BEM-классы компонентов, utilities;
- поведение (фокус, клавиатура, ARIA, порталы, оверлеи) — на **Kobalte**;
- `@heroui/react` **не устанавливать** — он потянет React в бандл.

Импорты в главном CSS:

```css
@import "tailwindcss";
@import "@heroui/styles/themes/default.css" layer(base);
@import "@heroui/styles/components/button.css" layer(components);
/* ...подключать компоненты точечно, по мере надобности */
```

Правило: **сначала ищем готовый HeroUI-класс/переменную, только потом пишем свой Tailwind.**
Кастомные цвета мимо темы HeroUI — запрещены; расширяем тему через CSS-переменные.

---

## 3. Архитектура: три слоя

```
SolidJS UI
    ↓  Tauri IPC (invoke / events)
Rust Core  (domain / application / infrastructure)
    ↓  tauri-plugin-player
Kotlin / Android  (Media3, ExoPlayer, MediaSessionService)
```

Границы жёсткие. Пересекать их «наискосок» нельзя.

### Что где живёт

- **Kotlin** — и только Kotlin — владеет воспроизведением: play/pause/seek/next/prev,
  shuffle/repeat, очередь на уровне плеера, нотификация, аудиофокус.
- **Rust** — владеет данными: библиотека, плейлисты, история, статистика, поиск, SQLite.
- **Solid** — владеет только представлением и локальным UI-состоянием (открытые модалки,
  скролл, таб). Источник истины по треку/прогрессу — нативный плеер, пробрасывается событиями.

### Запрещено

- `<audio>` / `HTMLAudioElement` / Web Audio API для воспроизведения библиотеки — **никогда**.
- SQL-запросы из фронтенда напрямую через `plugin-sql` — весь SQL живёт в Rust-репозиториях.
- Хранение библиотеки/плейлистов в Tauri Store — там только мелкие настройки.
- Складывать mp3 в `src-tauri/` или в ассеты APK.

---

## 4. Структура репозитория

```
music.xima/
├── src/                          # SolidJS
│   ├── app/                      # роутинг, провайдеры, entry
│   ├── features/                 # library, player, playlists, search, stats
│   │   └── <feature>/
│   │       ├── ui/               # компоненты
│   │       ├── model/            # сигналы/сторы фичи
│   │       └── api/              # обёртки над invoke
│   ├── shared/
│   │   ├── ui/                   # атомы поверх Kobalte + HeroUI-классов
│   │   ├── ipc/                  # типизированный invoke + типы из Rust
│   │   └── lib/
│   └── styles/index.css          # Tailwind + HeroUI импорты
│
├── src-tauri/
│   ├── src/
│   │   ├── domain/               # track, album, artist, playlist, queue
│   │   ├── application/          # library_service, playlist_service, ...
│   │   ├── infrastructure/
│   │   │   ├── sqlite/           # реализации репозиториев, миграции
│   │   │   ├── android/          # мост к плагину плеера
│   │   │   └── repositories/     # трейты-интерфейсы
│   │   └── commands/             # #[tauri::command] — тонкий слой
│   └── gen/android/
│
└── tauri-plugin-player/          # custom плагин
    ├── src/                      # Rust-сторона
    ├── guest-js/                 # TS-биндинги
    └── android/                  # Kotlin: ExoPlayer, MediaSessionService
```

---

## 5. Правила по слоям

### 5.1 Rust

Поток вызова всегда один:

```
Tauri Command → Application Service → Repository (трейт) → SQLite impl
```

- Команда — **тонкая**: распарсить вход, дёрнуть сервис, вернуть DTO. Никакого SQL и логики.
- `domain/` не знает ни про SQLite, ни про Tauri, ни про Android. Чистые типы и правила.
- Репозитории объявляются трейтами в `infrastructure/repositories/`, реализуются в `sqlite/`.
- Ошибки — `thiserror`, отдельный enum на слой; наружу в IPC отдаём сериализуемый вариант.
- Никаких `unwrap()`/`expect()` в путях, зависящих от пользовательских данных.
- Функция-«бог» вида `fn do_everything()` — сразу отклоняется на ревью.

### 5.2 TypeScript

`tsconfig.json` — максимальный strict:

```json
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "noImplicitOverride": true,
    "noFallthroughCasesInSwitch": true,
    "noImplicitReturns": true,
    "useUnknownInCatchVariables": true
  }
}
```

Запрещено: `any`, `as any`, `Record<string, any>`, `@ts-ignore`.
Для непроверенных данных — `unknown` + сужение.

IPC-типы **синхронизируются с Rust**, а не пишутся руками параллельно
(генерация из `serde`-структур). Расхождение `Rust Track` ↔ `TS Track` — баг.

### 5.3 SolidJS

- Реактивность через сигналы/сторы; не разрушать её деструктуризацией пропсов
  (`props.track`, а не `const { track } = props`).
- Списки — `<For>`/`<Index>`, ветвление — `<Show>`/`<Switch>`. Не `.map()` в JSX.
- Длинные списки библиотеки (10k+ треков) — виртуализация обязательна.
- Глобальное состояние плеера — один стор, наполняемый событиями из нативного слоя.
- `Tauri Store` читается на старте, пишется дебаунсом.

### 5.4 Kotlin / Android

- Один `MediaSessionService` на приложение; `ExoPlayer` создаётся и живёт в нём.
- Команды из Rust идут в плеер только через плагин; не дублировать состояние плеера в Rust.
- Состояние обратно во фронт — событиями (`Player.Listener` → Tauri event), не поллингом.
- Пути к файлам — content URI (SAF/MediaStore), не абсолютные пути. Persistable permission
  для выбранной пользователем папки берём и сохраняем.
- Учитывать аудиофокус, обрыв наушников, `foregroundServiceType="mediaPlayback"`.

---

## 6. База данных

Таблицы: `tracks`, `artists`, `albums`, `playlists`, `playlist_tracks`, `favorites`,
`history`, `play_counts` (поля — см. [`promt.md`](./promt.md)).

Правила:

- Миграции версионированные, только вперёд, в `infrastructure/sqlite/migrations/`.
  Менять уже применённую миграцию нельзя — добавлять новую.
- Поиск — виртуальная FTS5-таблица, синхронизируемая триггерами.
- Индексы под каждый экран сортировки/фильтрации (artist_id, album_id, date_added,
  last_played_at).
- Smart Playlists — это **правила**, хранимые как данные, компилируемые в SQL в Rust.
  Не хардкодить отдельный метод под каждый пресет.
- Ничего не пишем в БД из UI-потока синхронно; всё через `tokio`.

---

## 7. Команды

> Пока проект не инициализирован — блок обновить после `create-tauri-app`.

```bash
pnpm install                   # зависимости
pnpm dev                       # веб-дев UI в браузере
pnpm tauri android dev         # дев на устройстве/эмуляторе
pnpm tauri android build       # APK
pnpm typecheck                 # tsc --noEmit
pnpm lint
cargo fmt --all && cargo clippy --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Перед тем как считать задачу готовой: `pnpm typecheck`, `cargo clippy` без ворнингов,
`cargo test` зелёный.

---

## 8. Definition of Done для агента

1. Изменение лежит в правильном слое и не пробивает границы (см. §3).
2. Нет `any` в TS и `unwrap()` в Rust на пользовательских данных.
3. Новые UI-элементы используют HeroUI-классы/переменные, а не самописные цвета.
4. Схема БД менялась → добавлена миграция, а не правка старой.
5. Новая логика в `domain/`/`application/` покрыта тестом.
6. Typecheck, clippy и тесты прогнаны, результат назван честно.
7. Не выдумывать API Tauri/Media3/HeroUI по памяти — сверяться с документацией.

---

## 9. Продуктовые ориентиры

- Навигация снизу: Home / Library / Search / Playlists.
- Главная — вся музыка одним списком, без каруселей и подборок.
- Тема только тёмная; переключателя темы и размера сетки в настройках нет.
- Глубина — четыре слоя: фон экрана → `depth-raised` → `depth-floating` → `depth-overlay`,
  липкие панели поверх контента — `depth-bar` (см. `src/styles/index.css`).
- Mini-player всегда виден; тап → fullscreen-плеер (artwork, seek, queue).
- Старт трека из любого списка сразу разворачивает fullscreen-плеер.
- Нет обложки — рисуем детерминированный mesh-градиент по семени
  (`shared/lib/mesh.ts`), а не иконку-ноту.
- Режимы библиотеки: Songs / Albums / Artists / Genres / Folders.
- Подборки на главной: Recently Played, Recently Added, Most Played, Never Played, Forgotten.
- Вне продукта: статистика, Android Auto, сон-таймер, эквалайзер, тексты песен.
- Ориентир по UI — что-то между Spotify, Apple Music, Poweramp и YouTube Music,
  но заметно чище. Не копия Spotify.
