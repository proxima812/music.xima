# Track Removal, Crossfade, and Swipe Deck

## Goal

Add reversible library hiding, permanent file deletion, a VK-inspired fullscreen Swipe Deck, and a configurable true crossfade while preserving the app's offline architecture and MediaSession behavior.

## Delivery phases

### Phase 1

- Hide and restore tracks without deleting their files.
- Permanently delete MediaStore and SAF-backed files with the required Android confirmation flow.
- Replace the current fullscreen player layout with Swipe Deck.

### Phase 2

- Replace the single native ExoPlayer implementation with a dual-player crossfade engine behind one Media3 Player facade.
- Add the crossfade slider and persist/apply its value as part of the same working release.
- Verify automatic and manual crossfades on a physical Android device, including background playback, Bluetooth, notification controls, and the lock screen.

The phases remain separate so file lifecycle changes and the audio-engine replacement can be tested and reviewed independently.

## Track hiding

The track action sheet gains `Скрыть из music.xima`.

- Hiding never modifies the source file.
- A hidden URI is stored as a persistent tombstone so a later MediaStore or SAF scan cannot re-import it.
- Hiding removes the track from visible library queries, search, favorites, playlists, history-derived lists, and the native queue. Existing playlist, favorite, and history relationships remain stored so restoration can bring them back.
- An Undo toast restores the track immediately after hiding.
- Settings gains a `Скрытые песни` screen that lists tombstones and can restore tracks after the Undo window has expired.
- Restoring removes the tombstone and triggers a targeted scan or full library refresh so the file returns if it still exists.

The Rust application layer owns this operation. The UI only invokes typed commands and reacts to `library:changed`.

## Permanent file deletion

The track action sheet gains a visually dangerous `Удалить файл с устройства` action, separate from hiding.

1. The app displays a destructive confirmation that names the track and states that the file will be removed from the device.
2. A dedicated application service records the operation in `pending_deletions`.
3. The native player plugin receives the track's `content://` URI.
4. MediaStore items use Android's delete-request confirmation flow when required. SAF documents use `DocumentsContract.deleteDocument` only when the persisted URI permission and provider flags allow deletion.
5. Only after native deletion succeeds does a SQLite transaction remove the track and its dependent library data.
6. The application removes the track from the native queue and emits `library:changed`.

If the user cancels or Android rejects the deletion, the pending record is cleared and the library remains unchanged. A file-system deletion and a SQLite transaction cannot be truly atomic, so `pending_deletions` provides crash recovery: startup reconciliation completes database and queue cleanup if the file was deleted before the process stopped.

References:

- [Android shared-media deletion](https://developer.android.com/training/data-storage/shared/media)
- [DocumentsContract deletion](https://developer.android.com/reference/android/provider/DocumentsContract)

## Application boundaries

Introduce a focused track-removal application service rather than putting orchestration in a Tauri command.

- `TrackRepository` owns tombstones, pending deletion state, and database cleanup.
- A native `TrackFilePort` deletes the physical file and reports success, cancellation, unsupported providers, or failure.
- `PlayerPort` removes the deleted/hidden track from the current queue and advances safely if it was active.
- Tauri commands parse input, call the service, and serialize the result.
- SolidJS owns dialogs, sheets, Undo state, and navigation only.

No frontend code accesses SQL or file APIs directly.

## Crossfade setting

The existing `crossfadeMs` setting remains the source of truth.

- Range: `0..12000` milliseconds.
- Default: `3000` milliseconds.
- UI: a settings slider labelled `Плавный переход`, displaying `Выкл.` at zero and seconds otherwise.
- Changes persist through `tauri-plugin-store` and are sent to the native player when the setting is committed.
- The slider is released in Phase 2 together with the native engine so the UI never exposes a setting that has no effect.

## Native crossfade engine

Phase 2 introduces `CrossfadeEngine` inside `MediaSessionService`.

- Two ExoPlayer instances alternate active and standby roles.
- The standby player prepares the next logical queue item before the transition window.
- During a transition, both players output simultaneously using an equal-power gain curve.
- User volume is a master gain; crossfade gains multiply it without mutating the saved volume.
- After completion, standby becomes active and the old player is reset for reuse.
- A single Media3 `SimpleBasePlayer` facade exposes the logical queue, current item, position, commands, and events to `MediaSession`.
- Audio focus is owned once by the service; the standby player does not request competing focus.
- Only the logical active track publishes state, history completion, notification metadata, and track-change events.

Crossfade applies to natural completion, Next, Previous, and horizontal artwork swipes. It falls back to a normal transition when the setting is zero, Repeat One is active, the track is too short, the next item cannot be prepared, or the engine is recovering from a playback error.

References:

- [Media3 custom Player guidance](https://developer.android.com/media/media3/session/player)
- [SimpleBasePlayer API](https://developer.android.com/reference/androidx/media3/common/SimpleBasePlayer)

## Fullscreen Swipe Deck

The fullscreen player follows the approved `C · Swipe Deck` direction while retaining the existing HeroUI theme and clean dark visual language.

### Layout

- Top row: collapse button, `Сейчас играет`, track menu.
- Main deck: current square artwork with visible edges of previous and next artwork.
- Metadata: title, artist, and album.
- Seek area: slider, elapsed time, remaining/total time.
- Primary controls: shuffle, previous, play/pause, next, repeat.
- Secondary controls: favorite, queue, track menu.
- Background: restrained colors derived from artwork; the existing deterministic mesh is used when artwork is missing.

The deck and controls remain centered and bounded on tablets instead of expanding across the full viewport.

### Gestures and motion

- Horizontal drag moves the artwork deck with the pointer.
- The gesture locks to its dominant axis after a small slop so horizontal track switching and vertical dismissal cannot trigger together.
- A short or slow drag springs back to the current item.
- A committed drag settles in about 220 milliseconds and invokes the same logical Next or Previous command used by buttons and MediaSession clients.
- A vertical downward drag dismisses fullscreen playback.
- Seek, buttons, sheets, and menus use `data-no-swipe` and never begin deck gestures.
- Reduced-motion mode removes decorative travel and background interpolation while keeping navigation functional.

## Error handling

- Cancellation is not displayed as an error.
- Unsupported deletion providers explain that the song can still be hidden from music.xima.
- Native and database failures produce specific destructive-action toasts and preserve recoverable state.
- Crossfade preparation failure logs the native reason, abandons the standby player, and continues with an ordinary transition.
- A failed manual transition returns both players to one authoritative active state before reporting the error.

## Testing and verification

### Rust

- Hide and restore tombstones.
- Hidden URIs stay excluded after scans.
- Pending deletion recovery.
- Dependent-row cleanup and orphan cleanup.
- Current and non-current queue removal.
- Command validation and serialized error codes.

### Kotlin

- MediaStore success, cancellation, and permission failures.
- SAF support flags and delete results.
- Crossfade state transitions with fake players and a controllable clock.
- Equal-power gain endpoints and midpoint.
- Master-volume composition.
- Automatic, Next, Previous, Repeat One, short-track, and preparation-failure paths.
- Exactly one logical track-change/history completion event per transition.

### SolidJS and UI

- Separate hide and permanent-delete actions.
- Confirmation copy, cancellation, Undo, and hidden-track restoration.
- Crossfade slider range, default, `Выкл.` state, and persistence.
- Swipe Deck thresholds, axis locking, cancellation, and reduced motion.
- Phone and tablet viewport checks.

### Completion gates

- `npm run typecheck`
- `npm run build`
- `npm run rust:lint`
- `npm run rust:test`
- Android Gradle build for the plugin/application.
- Physical-device checks for file deletion, background playback, notification, lock screen, Bluetooth, and audible crossfade quality.
