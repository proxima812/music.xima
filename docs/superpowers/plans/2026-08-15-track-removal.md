# Track Hiding and Permanent File Deletion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add reversible hiding, restoration, and Android-confirmed permanent deletion of library tracks without breaking queue, playlist, favorite, history, or scan behavior.

**Architecture:** A new Rust `TrackRemovalService` orchestrates repository tombstones, native file deletion, queue cleanup, and recovery. SQLite remains the source of truth for visible/hidden/pending state; Kotlin alone touches MediaStore/SAF files; SolidJS only invokes typed commands and renders dialogs, Undo, and the hidden-songs screen.

**Tech Stack:** Rust + sqlx/SQLite, Tauri 2 mobile plugin, Kotlin/Android MediaStore and SAF, SolidJS strict TypeScript, Kobalte, HeroUI/Tailwind v4.

---

## Global constraints

- Work directly on the existing `main` checkout, as explicitly requested; do not create a branch or worktree.
- Preserve every pre-existing dirty file and build on the current `FullPlayer.tsx`/gesture/UI changes.
- Do not expose SQL or Android file APIs to SolidJS.
- Hiding preserves the track row and all playlist/favorite/history/play-count relationships.
- Permanent deletion is successful only after native deletion succeeds; cancellation is a normal result, not an error.
- No new frontend dependency is needed.
- Use only HeroUI theme/component classes and existing tokens for new UI.

## Public contracts to add

Rust domain DTOs in `src-tauri/src/domain/track_removal.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HiddenTrack {
    pub track: Track,
    pub hidden_at: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeleteTrackResult {
    Deleted,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDeleteOutcome {
    Deleted,
    Cancelled,
}
```

Repository operations in `src-tauri/src/infrastructure/repositories/track.rs`:

```rust
async fn hide(&self, track_id: i64, hidden_at: i64) -> CoreResult<()>;
async fn restore(&self, track_id: i64) -> CoreResult<()>;
async fn hidden(&self) -> CoreResult<Vec<HiddenTrack>>;
async fn begin_deletion(&self, track_id: i64, requested_at: i64) -> CoreResult<PendingDeletion>;
async fn cancel_deletion(&self, track_id: i64) -> CoreResult<()>;
async fn mark_file_deleted(&self, track_id: i64) -> CoreResult<()>;
async fn finalize_deletion(&self, track_id: i64) -> CoreResult<()>;
async fn pending_deletions(&self) -> CoreResult<Vec<PendingDeletion>>;
```

Native boundary in `src-tauri/src/application/track_removal_service.rs`:

```rust
#[async_trait::async_trait]
pub trait TrackFilePort: Send + Sync {
    async fn delete(&self, uri: &str) -> CoreResult<FileDeleteOutcome>;
    async fn exists(&self, uri: &str) -> CoreResult<bool>;
}
```

Tauri commands:

```text
track_hide { id } -> void
track_restore { id } -> void
track_hidden -> HiddenTrack[]
track_delete_file { id } -> "deleted" | "cancelled"
```

## Task 1: Add the forward-only database state

**Files:**

- Create: `src-tauri/src/infrastructure/sqlite/migrations/0003_track_removal.sql`
- Modify: `src-tauri/src/infrastructure/sqlite/pool.rs` only if migrations are registered explicitly there
- Test: `src-tauri/src/infrastructure/sqlite/track_repo.rs`

**Step 1: Write failing repository schema tests**

Add tests that connect to a fresh test DB and assert:

- hiding creates exactly one tombstone keyed by both `track_id` and `uri`;
- hiding twice is idempotent and refreshes neither track data nor relationships;
- pending deletion is unique per track/URI;
- deleting a track cascades both removal-state rows.

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml infrastructure::sqlite::track_repo -- --nocapture
```

Expected: FAIL because the tables do not exist.

**Step 2: Add migration `0003_track_removal.sql`**

```sql
CREATE TABLE hidden_tracks (
  track_id INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
  uri TEXT NOT NULL UNIQUE,
  hidden_at INTEGER NOT NULL
);

CREATE TABLE pending_deletions (
  track_id INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
  uri TEXT NOT NULL UNIQUE,
  requested_at INTEGER NOT NULL,
  file_deleted INTEGER NOT NULL DEFAULT 0 CHECK (file_deleted IN (0, 1))
);

CREATE INDEX idx_hidden_tracks_hidden_at ON hidden_tracks(hidden_at DESC);
CREATE INDEX idx_pending_deletions_file_deleted ON pending_deletions(file_deleted, requested_at);
```

Do not edit `0001_init.sql` or `0002_ru_smart_presets.sql`.

**Step 3: Re-run the focused test**

Expected: schema setup passes; behavior tests may still fail until Task 2.

**Step 4: Commit**

```bash
git add src-tauri/src/infrastructure/sqlite/migrations/0003_track_removal.sql src-tauri/src/infrastructure/sqlite/pool.rs src-tauri/src/infrastructure/sqlite/track_repo.rs
git commit -m "feat: add track removal state"
```

## Task 2: Implement tombstones and deletion transactions

**Files:**

- Create: `src-tauri/src/domain/track_removal.rs`
- Modify: `src-tauri/src/domain/mod.rs`
- Modify: `src-tauri/src/infrastructure/repositories/track.rs`
- Modify: `src-tauri/src/infrastructure/sqlite/track_repo.rs`
- Modify: `src-tauri/src/infrastructure/sqlite/mod.rs`
- Test: `src-tauri/src/infrastructure/sqlite/track_repo.rs`

**Step 1: Add failing tests for the repository contract**

Cover:

- `hide` copies the canonical URI from `tracks`; an unknown ID returns `NOT_FOUND`;
- `hidden` returns full `Track` metadata newest-first;
- `restore` removes only the tombstone;
- `begin_deletion` records the current URI;
- `mark_file_deleted` persists recovery state;
- `cancel_deletion` leaves the track and relationships intact;
- `finalize_deletion` deletes the track in one transaction and relies on existing foreign-key cascades/orphan cleanup conventions.

**Step 2: Implement DTOs and repository methods**

Use `INSERT ... SELECT ... ON CONFLICT(track_id) DO NOTHING` for hiding so the repository, not the command, resolves the URI. Wrap `finalize_deletion` in a SQL transaction. Return a typed `NOT_FOUND` error when `rows_affected() == 0` for required track operations.

**Step 3: Make scan upserts tombstone-aware**

In `SqliteTrackRepository::upsert_many`, exclude any `ScannedTrack.uri` present in `hidden_tracks`. Keep full-scan `keep_uris` unchanged: if a hidden file is still on disk its original row survives; if it disappears, `delete_missing` removes the row and cascades the stale tombstone.

Add a regression test:

1. Insert a track.
2. Hide it.
3. Upsert the same scanned URI with changed metadata.
4. Assert it remains hidden and absent from visible queries.

**Step 4: Run focused tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml infrastructure::sqlite::track_repo -- --nocapture
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src-tauri/src/domain src-tauri/src/infrastructure/repositories/track.rs src-tauri/src/infrastructure/sqlite
git commit -m "feat: persist hidden and pending tracks"
```

## Task 3: Exclude hidden tracks from every visible surface

**Files:**

- Modify: `src-tauri/src/infrastructure/sqlite/sql.rs`
- Modify: `src-tauri/src/infrastructure/sqlite/track_repo.rs`
- Modify: `src-tauri/src/infrastructure/sqlite/album_repo.rs`
- Modify: `src-tauri/src/infrastructure/sqlite/artist_repo.rs`
- Modify: `src-tauri/src/infrastructure/sqlite/favorite_repo.rs`
- Modify: `src-tauri/src/infrastructure/sqlite/history_repo.rs`
- Modify: `src-tauri/src/infrastructure/sqlite/playlist_repo.rs`
- Modify: `src-tauri/src/infrastructure/sqlite/search_repo.rs`
- Modify: `src-tauri/src/infrastructure/sqlite/smart_repo.rs`
- Modify: `src-tauri/src/infrastructure/sqlite/statistics_repo.rs`
- Modify: `src-tauri/src/infrastructure/sqlite/taxonomy_repo.rs`
- Test: the corresponding repository test modules

**Step 1: Add one hidden-track fixture to each repository family**

Before changing SQL, assert a hidden track does not appear in:

- direct track pages and recently-added;
- album/artist track lists or aggregate counts;
- favorites and playlist reads while their relationship rows remain;
- history-derived lists and statistics;
- search/FTS results;
- smart playlists;
- genres and folders.

Also assert an album/artist/genre/folder with no visible tracks is not emitted.

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml infrastructure::sqlite -- --nocapture
```

Expected: FAIL with hidden rows still visible.

**Step 2: Centralize the visibility predicate**

Add a shared SQL fragment/helper in `sql.rs`, then apply an anti-join consistently:

```sql
NOT EXISTS (
  SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = tracks.id
)
```

Do not delete playlist/favorite/history relations. Do not modify FTS triggers: filter the joined track at query time so restoring is immediate.

**Step 3: Re-run all SQLite tests**

Expected: PASS.

**Step 4: Commit**

```bash
git add src-tauri/src/infrastructure/sqlite
git commit -m "feat: hide tracks from library surfaces"
```

## Task 4: Add queue-safe application orchestration

**Files:**

- Create: `src-tauri/src/application/track_removal_service.rs`
- Modify: `src-tauri/src/application/mod.rs`
- Modify: `src-tauri/src/application/player_service.rs`
- Modify: `src-tauri/src/state.rs`
- Test: `src-tauri/src/application/track_removal_service.rs`
- Test: `src-tauri/src/application/player_service.rs`

**Step 1: Write failing PlayerService tests**

Add `PlayerService::remove_track(track_id)` and test:

- all duplicate occurrences are removed in descending index order;
- a non-current item leaves the current item stable;
- removing the current item delegates to Media3 removal so native playback advances;
- an absent ID is an idempotent no-op;
- the local mirror matches the native queue after success and is unchanged after a port error.

**Step 2: Implement queue removal**

Read `player.queue_ids()`, collect matching indexes, call `remove_queue_item` highest-to-lowest, and update the mirror only after each successful native removal. Do not clear/rebuild the whole queue because that would reset playback position.

**Step 3: Write failing TrackRemovalService tests with fakes**

Required scenarios:

- hide: repository tombstone first, queue removal second;
- restore: tombstone removed;
- delete success: begin -> native delete -> mark file deleted -> queue cleanup -> finalize;
- user cancellation: begin -> native cancelled -> pending cleared, track untouched;
- native failure: pending cleared, error preserved;
- DB failure after native success: `file_deleted=1` remains for recovery;
- startup recovery finalizes every `file_deleted=1` record; for a `file_deleted=0` record it probes `TrackFilePort::exists`: a missing file is finalized, an existing file clears the stale pending operation, and an indeterminate provider error leaves the row for the next startup instead of guessing.

Inject `TrackRepository`, `TrackFilePort`, `PlayerService`, and `Clock`. Do not use Tauri types in this service.

**Step 4: Implement service and state wiring type**

Export `TrackRemovalService` from `application/mod.rs` and add `pub track_removal: Arc<TrackRemovalService>` to `AppState`.

**Step 5: Run application tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml application::track_removal_service -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml application::player_service -- --nocapture
```

Expected: PASS.

**Step 6: Commit**

```bash
git add src-tauri/src/application src-tauri/src/state.rs
git commit -m "feat: orchestrate track removal"
```

## Task 5: Add native MediaStore and SAF deletion

**Files:**

- Create: `tauri-plugin-player/android/src/main/java/com/xima/music/player/TrackFileDeleter.kt`
- Create: `tauri-plugin-player/android/src/test/java/com/xima/music/player/TrackFileDeleterTest.kt`
- Modify: `tauri-plugin-player/android/src/main/java/com/xima/music/player/PlayerPlugin.kt`
- Modify: `tauri-plugin-player/src/models.rs`
- Modify: `tauri-plugin-player/src/mobile.rs`
- Modify: `tauri-plugin-player/src/desktop.rs`
- Modify: `tauri-plugin-player/src/commands.rs`
- Modify: `tauri-plugin-player/src/lib.rs`
- Modify: `tauri-plugin-player/permissions/default.toml`
- Modify: `src-tauri/src/infrastructure/android/player_adapter.rs`

**Step 1: Add pure Kotlin classification tests**

Extract URI/provider decisions into testable functions and cover:

- MediaStore URI;
- SAF document URI with delete capability;
- SAF document without delete support;
- unsupported/non-content URI;
- `RESULT_CANCELED` maps to `cancelled`, never an exception.
- existence probing distinguishes present, missing, and provider failure so crash recovery never deletes a DB row on an ambiguous result.

Run the plugin's existing Gradle unit-test task from `src-tauri/gen/android` (confirm the module name with `./gradlew projects` first).

Expected: FAIL because `TrackFileDeleter` is missing.

**Step 2: Implement `deleteTrackFile` plugin command**

Contract:

```json
{ "uri": "content://..." } -> { "status": "deleted" | "cancelled" }
trackFileExists { "uri": "content://..." } -> { "exists": true | false }
```

- MediaStore on Android 30+: create `MediaStore.createDeleteRequest(contentResolver, listOf(uri))`, launch its `IntentSender`, resolve from an `@ActivityCallback`.
- MediaStore where direct deletion is permitted: call `contentResolver.delete(uri, null, null)` and require a positive row count.
- SAF: inspect `DocumentsContract.Document.COLUMN_FLAGS`; require `FLAG_SUPPORTS_DELETE`, then call `DocumentsContract.deleteDocument` on `Dispatchers.IO`.
- Reject unsupported providers with a stable native error code/message that Rust maps to `UNSUPPORTED_DELETE`; do not silently hide.
- A dismissed system confirmation resolves `cancelled`.
- `trackFileExists` uses a read-only provider query/open-descriptor check; not-found maps to `false`, while permission/provider failures reject and remain retryable.

Follow the existing `pickFolder`/`onFolderPicked` callback pattern. Do not retain an Activity or Invoke beyond the Tauri callback mechanism.

**Step 3: Wire the Rust plugin surface**

Add the command to `commands.rs`, `lib.rs`, `mobile.rs`, desktop stub, generated permission allowlist, and `permissions/default.toml`. Add serializable `DeleteFileResponse { status }` with a closed enum.

Implement `TrackFilePort` on the Android adapter; desktop returns an explicit unsupported error.

**Step 4: Run checks**

```bash
cargo test --manifest-path tauri-plugin-player/Cargo.toml
cargo clippy --manifest-path tauri-plugin-player/Cargo.toml --all-targets -- -D warnings
cd src-tauri/gen/android && ./gradlew test
```

Expected: PASS.

**Step 5: Commit**

```bash
git add tauri-plugin-player src-tauri/src/infrastructure/android/player_adapter.rs
git commit -m "feat: delete Android track files"
```

## Task 6: Expose thin Tauri commands and startup recovery

**Files:**

- Create: `src-tauri/src/commands/track_removal.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/state.rs`
- Test: `src-tauri/src/commands/track_removal.rs`

**Step 1: Add command validation tests**

Assert non-positive IDs produce `INVALID_INPUT`, cancellation serializes as `"cancelled"`, and service errors retain stable codes.

**Step 2: Implement thin commands**

Each command calls `validated_id`, delegates to `state.track_removal`, and emits `emit_library_changed` only after a state-changing success:

- reason `track-hidden`;
- reason `track-restored`;
- reason `track-deleted`.

Do not emit for cancellation.

**Step 3: Assemble service and reconcile in `lib.rs`**

Construct one `Arc<TrackRemovalService>` from the existing repository/player adapter and add commands to `generate_handler!`. Run recovery during setup after database migrations and service construction, before normal UI use. Log recoverable failures with track ID and stable code; do not panic app startup.

**Step 4: Run core checks**

```bash
npm run rust:fmt
npm run rust:lint
npm run rust:test
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src-tauri/src/commands src-tauri/src/lib.rs src-tauri/src/state.rs
git commit -m "feat: expose track removal commands"
```

## Task 7: Add typed frontend APIs and actionable Undo toast

**Files:**

- Modify: `src/shared/ipc/types.ts`
- Modify: `src/shared/ipc/commands.ts`
- Modify: `src/shared/ui/Toasts.tsx`
- Modify: `src/styles/index.css` only if the existing HeroUI toast/button classes cannot express the action placement

**Step 1: Add strict IPC types**

```ts
export type HiddenTrack = { track: Track; hiddenAt: number }
export type DeleteTrackResult = 'deleted' | 'cancelled'
```

Add `trackHide`, `trackRestore`, `trackHidden`, and `trackDeleteFile` wrappers through the existing typed invoke helper. No `any`, casts to `any`, or direct `invoke` calls from components.

**Step 2: Extend toast options**

```ts
export type ToastAction = {
  label: string
  onClick: () => void | Promise<void>
  ariaLabel?: string
}

export type ToastOptions = {
  // existing fields
  action?: ToastAction
  duration?: number
}
```

Render the action with the existing small ghost/accent button class. On click, await the action, dismiss only after success, and keep/re-report failure. Pass `duration={options.duration}` to the individual `<Toast>` root; Kobalte supports overriding the region duration per toast. Use 8 seconds for the Undo toast and retain the four-second default for ordinary notifications.

**Step 3: Run TypeScript validation**

```bash
npm run typecheck
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src/shared/ipc src/shared/ui/Toasts.tsx src/styles/index.css
git commit -m "feat: add track removal UI contracts"
```

## Task 8: Add track actions, confirmation, and Undo

**Files:**

- Modify: `src/features/player/ui/TrackMenu.tsx`
- Reuse: `src/shared/ui/ConfirmDialog.tsx`
- Modify: `src/features/library/model/library-store.ts` only if the existing `library:changed` reset does not already refresh all caches

**Step 1: Add two visually separate actions**

In the action sheet add:

- `Скрыть из music.xima` with a non-danger hide icon;
- `Удалить файл с устройства` with `Trash2`, danger text/button styling, after a separator.

Do not combine them into one ambiguous Delete action.

**Step 2: Implement hiding and Undo**

On hide success:

1. Close the sheet.
2. Show `Песня скрыта из music.xima` with action `Вернуть`.
3. Undo calls `trackRestore(track.id)` and shows `Песня возвращена`.
4. Errors show specific danger toasts and keep user data unchanged.

**Step 3: Implement destructive confirmation**

Use `ConfirmDialog` with:

- title: `Удалить файл с устройства?`;
- description: `«{title}» будет удалена с устройства без возможности восстановления.`;
- confirm: `Удалить файл`;
- `danger={true}`.

On `cancelled`, close without an error toast. On `deleted`, close fullscreen/menu as needed and show `Файл удалён с устройства`. Unsupported provider errors explain: `Этот файл нельзя удалить через Android. Его можно скрыть из music.xima.`

**Step 4: Validate**

```bash
npm run typecheck
npm run build
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/features/player/ui/TrackMenu.tsx src/features/library/model/library-store.ts
git commit -m "feat: add hide and delete track actions"
```

## Task 9: Add the hidden-songs settings screen

**Files:**

- Create: `src/features/settings/ui/HiddenTracksScreen.tsx`
- Modify: `src/features/settings/ui/SettingsScreen.tsx`
- Modify: `src/app/routes.tsx`

**Step 1: Add route and entry row**

Lazy route: `/settings/hidden-tracks`. In the Library section of settings, add a chevron row `Скрытые песни`; do not add a new bottom-nav destination.

**Step 2: Implement screen states**

Use `createResource(trackHidden)` and existing `Screen`, `TopBar`, `TrackRow`/empty-state primitives:

- loading spinner;
- empty: `Скрытых песен нет`;
- list newest-first with title/artist and `Вернуть` action;
- restore removes the row after success and refreshes the resource;
- failure leaves the row visible and shows a danger toast.

Do not provide permanent deletion from this screen; it is a restore surface.

**Step 3: Validate phone/tablet layout**

```bash
npm run typecheck
npm run build
```

Check at 360x800 and 800x1280: no horizontal overflow, 44px minimum touch targets, long titles truncate.

**Step 4: Commit**

```bash
git add src/features/settings/ui src/app/routes.tsx
git commit -m "feat: restore hidden songs from settings"
```

## Task 10: End-to-end verification on Android

**Files:** no production edits unless a failing check reveals a scoped defect.

**Step 1: Run automated gates**

```bash
npm run typecheck
npm run build
npm run rust:fmt
npm run rust:lint
npm run rust:test
npm run android:build
```

Expected: all PASS with no warnings promoted by clippy.

**Step 2: Physical-device matrix**

Test one MediaStore song and one SAF song:

- hide while stopped, queued, current, duplicated in queue;
- rescan and restart: hidden song stays hidden;
- Undo and Settings restore recover playlist/favorite/history relations;
- delete confirmation accept/cancel;
- delete current track advances once and notification/lock screen update;
- provider denial/unsupported URI leaves DB and file intact;
- force-stop after native file deletion but before DB finalization (debug hook or controlled breakpoint), restart, confirm recovery removes stale DB/queue state.

**Step 3: Inspect scope**

```bash
git status --short
git diff --stat HEAD~9..HEAD
git diff --check
```

Confirm no pre-existing icon/device-UI changes were reverted and no unrelated files entered commits.

**Step 4: Final commit only if verification required fixes**

```bash
git add <only-the-scoped-fix-files>
git commit -m "fix: harden track removal flow"
```
