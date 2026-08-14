Да. Для такого проекта я бы **не делал аудиоплеер чисто через `<audio>` в WebView**. UI — Tauri/WebView, а само воспроизведение — нативное Android. Это даст нормальную фоновую музыку, управление с экрана блокировки, Bluetooth/гарнитуру и системную медиасессию. Android сейчас рекомендует Media3/ExoPlayer + `MediaSessionService` для фонового воспроизведения. ([Android Developers][1])

## Стек

```text
Application
├── Tauri 2
│   ├── Rust
│   └── Android / APK
│
├── Frontend
│   ├── SolidJS
│   ├── TypeScript strict
│   ├── Tailwind CSS v4
│   ├── @kobalte/core
│   └── Lucide Icons
│
├── Native Audio Engine
│   ├── Kotlin
│   ├── AndroidX Media3
│   ├── ExoPlayer
│   ├── MediaSession
│   └── MediaSessionService
│
├── Music Library
│   ├── Android MediaStore
│   ├── Storage Access Framework
│   ├── SQLite
│   └── Tauri SQL plugin
│
├── Rust Core
│   ├── serde
│   ├── thiserror
│   ├── tokio
│   └── domain/services/repositories
│
└── State
    ├── Solid signals/stores
    └── Tauri Store — только настройки
```

Tauri 2 официально поддерживает Android и позволяет писать мобильные плагины с нативной Android-частью на Kotlin, поэтому такая архитектура ложится на него нормально. ([Tauri][2])

### Почему **SolidJS**, а не Astro

Для музыкального приложения Astro тебе здесь практически ничего не даёт. У тебя постоянно меняется состояние:

* текущий трек;
* progress;
* queue;
* shuffle/repeat;
* библиотека;
* поиск;
* плейлисты;
* favorites;
* история;
* bottom-player;
* fullscreen-player.

Поэтому:

**SolidJS + TypeScript + Tailwind v4** здесь будет намного естественнее.

И при этом синтаксически и архитектурно он достаточно лёгкий.

---

# Главная архитектура

Я бы разделил приложение на **3 слоя**.

```text
┌────────────────────────────────────┐
│             SolidJS UI             │
│                                    │
│ Library / Albums / Artists         │
│ Search / Playlists / Player        │
└────────────────┬───────────────────┘
                 │
             Tauri IPC
                 │
┌────────────────▼───────────────────┐
│             Rust Core              │
│                                    │
│ Library                            │
│ Playlists                          │
│ History                            │
│ Statistics                         │
│ Search                             │
│ SQLite                             │
└────────────────┬───────────────────┘
                 │
          Tauri Mobile Plugin
                 │
┌────────────────▼───────────────────┐
│          Kotlin / Android          │
│                                    │
│ Media3 / ExoPlayer                 │
│ MediaSessionService                │
│ MediaStore                         │
│ Android Notifications              │
│ Bluetooth / Headset controls       │
└────────────────────────────────────┘
```

## 1. Audio Engine — Kotlin

Вот эту часть я бы **специально не переносил в JS**.

```text
AndroidX Media3
ExoPlayer
MediaSession
MediaSessionService
```

Media3 является текущим Android media stack, а `MediaSession` позволяет системе и внешним устройствам управлять плеером. ([Android Developers][3])

Получишь:

```text
▶ Play
⏸ Pause
⏭ Next
⏮ Previous

seek
shuffle
repeat

background playback

lock screen controls
notification controls

Bluetooth controls
headset controls
```

И главное:

**экран выключился → музыка продолжает играть.**

Android прямо предусматривает `MediaSessionService` для такого сценария. ([Android Developers][4])

---

# 2. Где хранить музыку

Я бы **не запихивал MP3 внутрь APK**.

Например:

```text
300 песен × 8 MB
≈ 2.4 GB
```

Получишь гигантский APK и любое изменение библиотеки будет требовать пересборки приложения.

Лучше:

```text
/storage/.../Music/
```

или пользователь один раз выбирает:

```text
Выбрать папку с музыкой
        ↓
Android SAF
        ↓
/Music/MyMusic/
```

Android предоставляет `ACTION_OPEN_DOCUMENT_TREE`, через который приложение получает доступ к выбранной пользователем директории и её содержимому. ([Android Developers][5])

А общую музыкальную библиотеку телефона можно индексировать через:

```text
MediaStore.Audio
```

который предназначен именно для `audio/*`. ([Android Developers][6])

---

# 3. SQLite

Никакого Supabase.

Полностью локально.

```text
app.db
```

Tauri имеет официальный SQL plugin с SQLite через `sqlx`. ([Tauri][7])

Пример структуры:

```text
tracks
------
id
uri
title
artist_id
album_id
duration_ms
track_number
disc_number
year
genre
bitrate
sample_rate
size
format
cover_key
date_added
last_modified


artists
-------
id
name


albums
------
id
artist_id
title
year
cover_key


playlists
---------
id
name
created_at
updated_at


playlist_tracks
---------------
playlist_id
track_id
position


favorites
---------
track_id
created_at


history
-------
track_id
played_at
duration_played_ms


play_counts
-----------
track_id
count
last_played_at
```

Это уже позволит делать интересную аналитику:

```text
Most Played
Recently Played
Never Played
Forgotten Songs
Top Artists
Top Albums
Top Genres

Today
This Week
This Month
All Time
```

---

# 4. Tauri Store

SQLite — данные.

`@tauri-apps/plugin-store` — только маленькие настройки. Это persistent key-value storage от Tauri. ([Tauri][8])

Например:

```ts
type AppSettings = {
  theme: 'dark' | 'light' | 'system'
  volume: number
  repeat: RepeatMode
  shuffle: boolean
  sort: LibrarySort
  gridSize: GridSize
  rememberQueue: boolean
}
```

---

# UI / UX

Я бы ориентировался не на Spotify-клон один-в-один, а сделал что-то между:

```text
Spotify
Apple Music
Poweramp
YouTube Music
```

но максимально чистое.

### Bottom navigation

```text
Home
Library
Search
Playlists
```

### Mini Player

Всегда снизу:

```text
[cover] Song Name
        Artist

        ◀  ▶/❚❚  ▶
```

Тап:

```text
        artwork

       Track Name
         Artist

────────────●────────────
1:42                  3:51

     ↶   ◀   ▶   ▶   ↷

        queue
```

---

# Library

```text
Library

Songs       312
Albums       43
Artists      71
Playlists     8

Recently added

[cover] [cover] [cover]
```

И обязательные режимы:

```text
Songs
Albums
Artists
Genres
Folders
```

---

# Очень мощная функция

### Smart Playlists

Не просто обычные playlists.

Например:

```text
Recently Added

Not Played
    playCount = 0

Forgotten
    lastPlayed > 60 days

Favorites
    favorite = true

Most Played
    ORDER BY playCount DESC

2020s
    year BETWEEN 2020 AND 2029

High Quality
    bitrate >= 320000
```

То есть пользователь может собирать **динамический playlist через правила**.

---

# Поиск

Локальный.

```text
search
 ↓
songs
artists
albums
playlists
```

Для ~300–10 000 песен обычного SQLite более чем достаточно.

Если захочешь сделать красиво архитектурно:

```text
SQLite FTS5
```

и поиск станет практически мгновенным.

---

# Rust

Я бы Rust использовал не просто как прокладку.

Пускай там находится **domain layer**:

```text
src-tauri/src/

domain/
├── track.rs
├── album.rs
├── artist.rs
├── playlist.rs
└── queue.rs

application/
├── library_service.rs
├── playlist_service.rs
├── history_service.rs
└── statistics_service.rs

infrastructure/
├── sqlite/
├── android/
└── repositories/

commands/
├── library.rs
├── playlist.rs
└── statistics.rs
```

И не делать:

```rust
#[tauri::command]
fn do_everything() {}
```

с SQL прямо внутри команды.

Лучше:

```text
UI
↓
Tauri Command
↓
Application Service
↓
Repository interface
↓
SQLite implementation
```

Это уже хорошо сочетается с твоим желанием держать архитектуру ближе к **EO / object-oriented boundaries**, а не превращать приложение в набор функций.

---

# TypeScript

Максимальный strict.

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

Никаких:

```ts
any
as any
Record<string, any>
```

IPC модели лучше генерировать/синхронизировать с Rust, чтобы:

```text
Rust Track
    ↕
TypeScript Track
```

не начинали жить отдельно.

---

# Итоговый стек

**Я бы зафиксировал проект вот так:**

```text
Tauri 2

Frontend
├── SolidJS
├── TypeScript strict
├── Tailwind CSS v4
├── Kobalte
└── Lucide

Core
├── Rust
├── serde
├── thiserror
└── tokio

Database
├── SQLite
└── Tauri SQL / sqlx

Android
├── Kotlin
├── AndroidX Media3
├── ExoPlayer
├── MediaSession
├── MediaSessionService
├── MediaStore
└── Storage Access Framework

Tauri
├── plugin-sql
├── plugin-store
├── plugin-fs
└── custom tauri-plugin-player

Storage
├── Music → Android shared storage
├── metadata → SQLite
├── settings → Tauri Store
└── cache/artwork → app cache
```

### И главное решение

**Не:**

```text
Tauri
└── <audio src="song.mp3">
```

**А:**

```text
SolidJS UI
       ↓
Tauri
       ↓
Rust
       ↓
custom tauri-plugin-player
       ↓
Kotlin
       ↓
Media3 / ExoPlayer
       ↓
Android Audio System
```

Так из этого можно сделать не игрушечный APK, а реально хороший персональный музыкальный плеер: быстрый, полностью offline, без аккаунтов и серверов, с нормальным background playback и огромным количеством своих функций. ([Tauri][9])

[1]: https://developer.android.com/media/media3/session/background-playback?utm_source=chatgpt.com "Background playback with a MediaSessionService"
[2]: https://v2.tauri.app/develop/plugins/?utm_source=chatgpt.com "Plugin Development"
[3]: https://developer.android.com/media/media3/session/control-playback?utm_source=chatgpt.com "Control and advertise playback using a MediaSession"
[4]: https://developer.android.com/media/media3/session/player?utm_source=chatgpt.com "The Player Interface | Android media"
[5]: https://developer.android.com/guide/topics/providers/document-provider?utm_source=chatgpt.com "Open files using the Storage Access Framework"
[6]: https://developer.android.com/reference/kotlin/android/provider/MediaStore.Audio?utm_source=chatgpt.com "MediaStore.Audio | API reference"
[7]: https://v2.tauri.app/plugin/sql/?utm_source=chatgpt.com "SQL"
[8]: https://v2.tauri.app/plugin/store/?utm_source=chatgpt.com "Store"
[9]: https://v2.tauri.app/develop/plugins/develop-mobile/?utm_source=chatgpt.com "Mobile Plugin Development"
