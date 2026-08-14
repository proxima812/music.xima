# Fullscreen Swipe Deck Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the fullscreen artwork area with the approved `C · Swipe Deck`: current artwork centered, previous/next edges visible, horizontal interactive switching, vertical dismissal, responsive phone/tablet layout, and reduced-motion support.

**Architecture:** Keep one fullscreen `Dialog` and one axis-locking gesture owner in `FullPlayer`. A focused `ArtworkDeck` renders queue neighbors and receives only presentation state. Pure deck helper functions compute neighbors and transforms and are tested with Node's built-in test runner; all navigation delegates to the existing `PlayerService` commands.

**Tech Stack:** SolidJS strict TypeScript, Kobalte Dialog, HeroUI styles, Tailwind CSS v4, existing `CoverArt`/deterministic glow, native Media3 player commands.

---

## Global constraints

- Work on the current `main` checkout and preserve all dirty files.
- Build on the already optimized `gestures.ts` axis lock and current `FullPlayer.tsx`; do not revert those changes.
- Do not copy VK assets, branding, or exact visual styling. The behavior/layout direction is the reference.
- Do not add React, animation libraries, or a test dependency.
- A swipe must invoke the same `player.next()` / `player.previous()` path used by buttons and MediaSession.
- Controls, seek bar, menus, and sheets remain `data-no-swipe` zones.
- Keep existing HeroUI variables/classes and Tailwind v4; no custom off-theme colors.

## Task 1: Extract and test deck math

**Files:**

- Create: `src/features/player/model/deck.ts`
- Create: `src/features/player/model/deck.test.ts`
- Modify: `package.json`

**Step 1: Write failing Node tests**

Use `node:test` and `node:assert/strict`. Cover:

```ts
type DeckNeighbors<T> = {
  previous: T | null
  current: T | null
  next: T | null
}

export function deckNeighbors<T>(items: readonly T[], index: number | null): DeckNeighbors<T>
export function clampDeckDrag(dx: number, viewportWidth: number): number
export function shouldCommitDeckSwipe(dx: number, velocityX: number, width: number): boolean
```

Expected rules:

- invalid/null index returns all null;
- first/last item has only the available neighbor; no implicit wrap;
- drag is rubber-banded after 42% of deck width;
- commit at 25% width or a fast same-direction flick;
- NaN/zero width returns safe neutral values.

Add script:

```json
"test:ui-model": "node --experimental-strip-types --test src/features/player/model/*.test.ts"
```

Run:

```bash
npm run test:ui-model
```

Expected: FAIL because `deck.ts` is missing.

**Step 2: Implement pure helpers**

Keep the file dependency-free and strict. Export constants for the commit fraction and maximum drag fraction so UI and tests cannot drift.

**Step 3: Re-run tests and typecheck**

```bash
npm run test:ui-model
npm run typecheck
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src/features/player/model/deck.ts src/features/player/model/deck.test.ts package.json
git commit -m "test: define swipe deck behavior"
```

## Task 2: Extend the existing gesture primitive without regressions

**Files:**

- Modify: `src/features/player/model/gestures.ts`
- Create: `src/features/player/model/gestures.test.ts`

**Step 1: Add failing gesture tests**

Extract/export pure decision functions only where necessary. Test:

- movement below 8px remains pending;
- horizontal/vertical dominance ratio prevents diagonal double-actions;
- left/right/down resolve at their independent thresholds;
- a horizontal gesture can use deck width and velocity to commit;
- `data-no-swipe` start remains ignored;
- cancel calls `onEnd` once and never `onSwipe`.

Do not weaken the current mini-player gesture behavior.

**Step 2: Add optional velocity/commit customization**

Extend `SwipeOptions` narrowly:

```ts
type SwipeEnd = SwipeDelta & { velocityX: number; velocityY: number }

type SwipeOptions = {
  // existing fields
  shouldCommit?: (direction: SwipeDirection, end: SwipeEnd) => boolean
}
```

Track the last move timestamp/position with `event.timeStamp`; use a bounded sample and return zero velocity when timing is invalid. Existing callers that omit `shouldCommit` retain the current threshold behavior.

**Step 3: Run model tests**

```bash
npm run test:ui-model
npm run typecheck
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src/features/player/model/gestures.ts src/features/player/model/gestures.test.ts
git commit -m "feat: support interactive deck gestures"
```

## Task 3: Build the presentation-only artwork deck

**Files:**

- Create: `src/features/player/ui/ArtworkDeck.tsx`
- Modify: `src/styles/index.css` only for a reusable HeroUI-token-based background/depth class that Tailwind cannot express inline

**Step 1: Define the component contract**

```ts
export type ArtworkDeckProps = {
  previous: Track | null
  current: Track
  next: Track | null
  dragX: number
  settling: boolean
  reducedMotion: boolean
}
```

The component contains no player-store calls and no touch handlers.

**Step 2: Render three stable slots**

- Current art: square, centered, highest depth, full accessible alt.
- Previous/next: absolute side slots with only 8–12% visible at rest, scaled slightly down, `aria-hidden="true"`.
- Use `CoverArt` for every slot, including its existing deterministic mesh fallback.
- Translate all slots from `dragX`; do not remount the current art during drag.
- Clamp width with `min()`/container queries so the deck remains square and centered on short phones and tablets.

Suggested structure:

```tsx
<div class="relative w-full overflow-hidden [container-type:inline-size]">
  <div class="relative mx-auto aspect-square w-[min(78cqw,58dvh,34rem)]">
    <DeckCard position="previous" ... />
    <DeckCard position="current" ... />
    <DeckCard position="next" ... />
  </div>
</div>
```

Use existing `ease-out-fluid` and approximately 220ms settling. Under `prefers-reduced-motion`, use zero decorative travel duration while retaining the final item change.

**Step 3: Validate component compilation**

```bash
npm run typecheck
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src/features/player/ui/ArtworkDeck.tsx src/styles/index.css
git commit -m "feat: add fullscreen artwork deck"
```

## Task 4: Integrate the deck and responsive fullscreen layout

**Files:**

- Modify: `src/features/player/ui/FullPlayer.tsx`
- Modify: `src/features/player/ui/SeekBar.tsx` only if elapsed/remaining labels are not already exposed
- Modify: `src/features/player/model/player-store.tsx` only if the current queue index is not exposed reactively

**Step 1: Compute queue neighbors from authoritative state**

Use `deckNeighbors(player.queue, player.state.queueIndex)`. Do not infer by track ID because queues may contain duplicate tracks. Do not wrap at edges unless the native queue itself reports wrapped neighbors.

**Step 2: Give one gesture owner both axes**

Replace the down-only setup with directions `['left', 'right', 'down']`:

- horizontal `onMove`: set `dragX` only;
- positive vertical `onMove`: set `dragY` only;
- left commit: animate deck out, call `player.next()`, reset after the current index changes;
- right commit: same with `player.previous()`;
- down commit: `player.closeFull()`;
- disallowed edge swipe: spring to zero, do not invoke a player command;
- while settling, ignore another commit to prevent double Next/Previous.

Use `shouldCommitDeckSwipe` for horizontal completion and the existing 90px threshold for dismissal. Reset both signals on cancel, dialog close, and track-change error.

**Step 3: Apply the approved content hierarchy**

Order:

1. top row: collapse, `Сейчас играет`, actual `TrackMenu` trigger;
2. `ArtworkDeck`;
3. title, artist, album;
4. seek slider plus elapsed and remaining/total labels;
5. primary controls;
6. secondary favorite and queue controls (do not duplicate the menu here after moving it to the top row).

Keep the content within `md:max-w-2xl` and use dynamic viewport/safe-area classes already present. Do not expand artwork to the full tablet width.

**Step 4: Add restrained artwork-derived background**

Use `glowGradient(album/title + artist)` as the fallback/background seed. Render it through a low-opacity pseudo/absolute layer behind content, using existing background/depth tokens for the base. Do not sample remote artwork in the WebView or introduce canvas; reduced motion disables interpolation.

**Step 5: Run automated checks**

```bash
npm run test:ui-model
npm run typecheck
npm run build
```

Expected: PASS.

**Step 6: Commit**

```bash
git add src/features/player/ui/FullPlayer.tsx src/features/player/ui/SeekBar.tsx src/features/player/model/player-store.tsx
git commit -m "feat: integrate fullscreen swipe deck"
```

## Task 5: Browser interaction and accessibility verification

**Files:** no production edits unless checks reveal a scoped issue.

**Step 1: Launch the UI**

```bash
npm run dev -- --host 127.0.0.1
```

Use the existing browser verification route/mock player state used by the project. If no mock queue exists, add temporary local state only outside tracked source files and remove it after verification.

**Step 2: Verify viewports**

At minimum:

- 360x800 compact Android phone;
- 412x915 large phone;
- 800x1280 tablet;
- landscape 915x412.

Assert: square artwork, visible neighbor edges, no horizontal page overflow, metadata/control truncation, safe-area clearance, and centered bounded tablet layout.

**Step 3: Verify interaction**

- slow short horizontal drag springs back;
- committed left/right drag changes exactly one item;
- rapid flick commits;
- diagonal movement locks to one axis;
- down drag closes without changing track;
- seek/button/menu/queue touches never move the deck;
- first/last queue edge springs back;
- repeated fast touches do not double-skip.

**Step 4: Verify accessibility/reduced motion**

- keyboard focus remains trapped by Dialog and returns on close;
- all icon buttons have labels;
- side artwork is hidden from accessibility tree;
- with `prefers-reduced-motion: reduce`, navigation works with no decorative travel/background interpolation;
- contrast remains readable over every deterministic glow seed sampled.

**Step 5: Run final checks**

```bash
npm run test:ui-model
npm run typecheck
npm run build
git diff --check
git status --short
```

Expected: PASS, with only intended tracked changes plus the user's pre-existing dirty files.

**Step 6: Commit scoped fixes, if any**

```bash
git add <only-the-scoped-fix-files>
git commit -m "fix: polish fullscreen swipe deck"
```
