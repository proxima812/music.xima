# music.xima

Персональный музыкальный плеер для Android. Полностью офлайн: без аккаунтов,
серверов и сетевых запросов. Музыка лежит в памяти телефона, а не внутри APK.

Воспроизведение — нативное (Media3/ExoPlayer в `MediaSessionService`), интерфейс —
WebView на Tauri 2 + SolidJS. В вебе нет ни одного `<audio>`: звук, нотификация,
экран блокировки и Bluetooth — целиком на стороне Android.

## Что умеет

- Библиотека из общей медиатеки (MediaStore) и своих папок (SAF): песни, альбомы,
  исполнители, жанры, папки; 11 сортировок, виртуализованные списки.
- Очередь: «играть следующим», перестановка, восстановление после перезапуска.
- Плейлисты и **умные плейлисты** — правила хранятся как данные и компилируются
  в SQL (11 типов правил, 7 пресетов из коробки).
- Поиск через SQLite FTS5, история прослушиваний, счётчики, избранное.
- Полноэкранный плеер, мини-плеер, фон и экран блокировки.
- Нет обложки — рисуется детерминированный glow-градиент, один и тот же для
  одного альбома (палитра — [color.xima.work](https://color.xima.work/collection/glow/)).

Тема только тёмная. Статистики, Android Auto, сон-таймера, эквалайзера и текстов
песен в продукте нет — сознательно.

## Стек

| Слой | Технологии |
| --- | --- |
| Оболочка | Tauri 2 (Android) |
| UI | SolidJS, TypeScript (strict), Tailwind v4, `@heroui/styles`, Kobalte |
| Ядро | Rust: домен, сервисы, репозитории |
| БД | SQLite (`sqlx`), FTS5, версионированные миграции |
| Звук | Kotlin, Media3/ExoPlayer, `MediaSessionService` |

Три слоя, границы жёсткие: Kotlin владеет воспроизведением, Rust — данными,
Solid — только представлением. Подробности — в [AGENTS.md](./AGENTS.md).

## Сборка

Нужны Node 22+, Rust с таргетом `aarch64-linux-android`, Android SDK, NDK 27 и JDK 21.

```bash
npm install

npm run typecheck                 # tsc --noEmit
npm run rust:test                 # тесты ядра
npm run rust:lint                 # clippy -D warnings

npm run android:dev               # дев-сборка с hot reload на устройстве
npm run android:build -- --debug --apk --target aarch64
```

APK появится в `src-tauri/gen/android/app/build/outputs/apk/universal/debug/`.

Музыку в репозиторий не кладём. Для теста достаточно закинуть файлы на телефон:

```bash
adb push track.mp3 /sdcard/Music/
adb shell content call --uri content://media/external --method scan_volume --arg external_primary
```

На Android 13+ приложение пока не запрашивает `READ_MEDIA_AUDIO` само
(см. BUGS.md B6), поэтому для первого запуска:

```bash
adb shell pm grant com.xima.music android.permission.READ_MEDIA_AUDIO
```

## Документация

| Файл | О чём |
| --- | --- |
| [AGENTS.md](./AGENTS.md) | как писать код в этом проекте |
| [docs/CONTRACTS.md](./docs/CONTRACTS.md) | типы, команды, события, схема БД — источник истины |
| [docs/FEATURES.md](./docs/FEATURES.md) | что реализовано и в каком статусе |
| [docs/BUGS.md](./docs/BUGS.md) | починенное, открытое и непроверенное |
| [docs/PROPOSALS.md](./docs/PROPOSALS.md) | что делать дальше |
| [docs/HEROUI.md](./docs/HEROUI.md) | токены и классы UI-темы |
| [promt.md](./promt.md) | исходная продуктовая спека |

## Статус

Собирается и работает на Pixel 8a: сканирование библиотеки, списки,
воспроизведение, умные плейлисты. Release-сборки и подписи APK пока нет —
только debug. Честный список непроверенного — в [docs/BUGS.md](./docs/BUGS.md).
