# xima.music

Персональный музыкальный плеер для Android. Полностью офлайн: без аккаунтов,
серверов и сетевых запросов. Музыка лежит в памяти телефона, а не внутри APK.

Воспроизведение — нативное (Media3/ExoPlayer в `MediaSessionService`), интерфейс —
WebView на Tauri 2 + SolidJS. В вебе нет ни одного `<audio>`: звук, нотификация,
экран блокировки и Bluetooth — целиком на стороне Android.

## Скачать

Готовый APK лежит на странице [Releases](https://github.com/proxima812/music.xima/releases).
Файл один на все архитектуры (arm64, arm32, x86, x86_64):

```bash
adb install xima.music-1.0.1.apk
```

Либо перенесите его на телефон и откройте в проводнике — Android спросит
разрешение на установку из неизвестного источника.

APK подписан самодельным ключом, поэтому обновления встают только поверх
сборок с той же подписью. Отпечаток сертификата (SHA-256):

```
274960300cd6383aebfd9b609ef9026860a5f43c322b53be92c63e5b456eadbe
```

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

### Релиз

```bash
npm run android:build -- --apk                    # release, все ABI, без подписи
MX_KEYSTORE_PASS='…' npm run release:sign         # выравнивание, подпись, проверка
```

Подпись — отдельным шагом, а не через `signingConfig` в Gradle: конфиг пришлось
бы держать в `src-tauri/gen/android`, а эта папка перезаписывается каждым
`tauri android init` (BUGS.md, B1). Ключ по умолчанию —
`~/.android/music-xima-release.jks`, ключ `music-xima`; путь и имя переопределяются
переменными `MX_KEYSTORE` и `MX_KEY_ALIAS`. **Хранилище и пароль в репозиторий не
попадают и восстановлению не подлежат** — потеряете пароль, обновить установленное
приложение уже нечем.

Музыку в репозиторий не кладём. Для теста достаточно закинуть файлы на телефон:

```bash
adb push track.mp3 /sdcard/Music/
adb shell content call --uri content://media/external --method scan_volume --arg external_primary
```

Разрешение на чтение музыки приложение запрашивает само при первом
сканировании. Если диалог был отклонён и больше не показывается:

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

## Как помочь

Правки и форки приветствуются. Перед тем как писать код:

- [AGENTS.md](./AGENTS.md) — принятые в проекте правила; слои и границы важнее
  стиля;
- [docs/CONTRACTS.md](./docs/CONTRACTS.md) — типы, команды и события между
  Rust, Kotlin и вебом. Мост строковый, компилятор опечатки не ловит;
- [docs/BUGS.md](./docs/BUGS.md) — открытое и непроверенное. Там же две ловушки
  окружения, на которые уходит день, если не знать: `src-tauri/gen/android`
  перезаписывается каждым `tauri android init` (B1), а кириллица в пути к
  проекту ломает Tauri CLI (B2).

Перед пул-реквестом:

```bash
npm run typecheck && npm run test:ui-model
npm run rust:lint && npm run rust:test
```

Тесты сейчас есть только у Rust-ядра и у моделей плеера — если добавите свои,
станет только лучше.

## Лицензия

[MIT](./LICENSE). Музыка, обложки и теги остаются вашими: приложение ничего
никуда не отправляет.

## Статус

Собирается и работает на Pixel 8a: сканирование библиотеки, списки,
воспроизведение, умные плейлисты, подписанные release-сборки. Честный список
непроверенного и открытых багов — в [docs/BUGS.md](./docs/BUGS.md).
