# Track removal frontend report (tasks 7–9)

## Files

- `src/shared/ipc/types.ts`, `src/shared/ipc/commands.ts`: strict removal contracts and typed wrappers.
- `src/shared/ui/Toasts.tsx`: per-toast duration plus awaited action support.
- `src/features/player/ui/TrackMenu.tsx`: separate hide and file-delete flows, Undo and confirmation.
- `src/features/settings/ui/HiddenTracksScreen.tsx`, `SettingsScreen.tsx`, `src/app/routes.tsx`: hidden-song restore screen and lazy route.

## Commits

- `21c0795 feat: add track removal UI contracts`
- `62344f5 feat: add hide and delete track actions`
- `efa6e1f feat: restore hidden songs from settings`

## Validation

- `npm run typecheck` — passed.
- `npm run build` — passed.
- `git diff --check` — passed before commits.

## Concern

The local Playwright wrapper could not start (`playwright-cli: command not found`), so the requested 360×800 and 800×1280 browser pass was not run. The new screen uses existing responsive `Screen`/HeroUI primitives, truncation and 44px minimum restore controls.
