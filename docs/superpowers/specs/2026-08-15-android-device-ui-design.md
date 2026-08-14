# Android device UI and branding

## Scope

- Use `src/assets/music.xima.png` as the source for generated Tauri and Android launcher icons.
- Keep the visible application name `music.xima` in Tauri, Android resources, and the document title.
- Optimize the existing UI for Android phones and tablets without redesigning its navigation or visual language.

## Touch and swipe behavior

- Preserve the current gestures: horizontal mini-player swipes change tracks, upward swipe opens the full player, and downward swipe closes it.
- Lock a gesture to its dominant axis after a small movement so vertical scrolling cannot accidentally trigger track changes.
- Cancel recognition for multitouch and ignored interactive controls.
- Base completion on deliberate distance and direction, then reset visual drag feedback on completion or cancellation.
- Keep native vertical scrolling available through explicit `touch-action` rules.

## Device viewport behavior

- Continue using Android safe-area insets for the status and gesture-navigation areas.
- Use dynamic viewport sizing for the application shell and fullscreen player so browser chrome and keyboard changes do not leave clipped or empty UI.
- Preserve existing minimum touch targets and disable ornamental motion when the device requests reduced motion.
- On wider tablet viewports, constrain reading and control widths while keeping backgrounds and bottom navigation full-width.
- Scale the fullscreen artwork and controls to the available height and width without stretching artwork or creating oversized control gaps.

## Validation

- Generate icon resources through the installed Tauri CLI and verify the Android mipmap outputs and application strings.
- Run TypeScript type checking and the focused production frontend build.
- Review the final diff to ensure no unrelated UI changes were introduced.
