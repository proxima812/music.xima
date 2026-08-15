import { Router, type RouteSectionProps } from '@solidjs/router'
import { ErrorBoundary, Show, Suspense } from 'solid-js'

import { usePlayer } from '@/features/player/model/player-store'
import { FullPlayer } from '@/features/player/ui/FullPlayer'
import { MiniPlayer } from '@/features/player/ui/MiniPlayer'
import { Button, Spinner } from '@/shared/ui'
import { BottomNav } from './BottomNav'
import { AppProviders } from './providers'
import { routes } from './routes'

/**
 * Каркас: скроллящийся `main` под экран, снизу — мини-плеер и навигация.
 * Оба бара `fixed`, поэтому экраны резервируют место через `pb-player-stack`.
 */
function AppShell(props: RouteSectionProps) {
  return (
    <AppProviders>
      <div class="safe-top flex h-dvh w-full flex-col overflow-hidden bg-background text-foreground">
        <main class="relative min-h-0 w-full flex-1 overflow-y-auto overscroll-contain">
          {/*
            Граница ошибок обязана быть внутри `Suspense`: без неё любое падение
            ресурса экрана оставляет приложение на заглушке-спиннере навсегда —
            ловить исключение просто некому.
          */}
          <Suspense
            fallback={
              <div class="flex h-full w-full items-center justify-center">
                <Spinner size="lg" color="accent" />
              </div>
            }
          >
            <ErrorBoundary fallback={(error, reset) => <ScreenError error={error} reset={reset} />}>
              {props.children}
            </ErrorBoundary>
          </Suspense>
        </main>

        <PlayerDock />
      </div>
    </AppProviders>
  )
}

/** Экран упал — показываем причину и даём пересобрать его, а не вечный спиннер. */
function ScreenError(props: { error: unknown; reset: () => void }) {
  const message = (): string =>
    props.error instanceof Error ? props.error.message : String(props.error)

  return (
    <div class="flex h-full w-full flex-col items-center justify-center gap-4 px-6 text-center">
      <p class="text-base font-medium text-foreground">Экран не открылся</p>
      <p class="text-sm text-muted">{message()}</p>
      <Button variant="secondary" onClick={props.reset}>
        Повторить
      </Button>
    </div>
  )
}

/** Мини-плеер показываем только когда в плеере есть текущий трек. */
function PlayerDock() {
  const player = usePlayer()

  const hasCurrentTrack = (): boolean => {
    // `current` у стора плеера может быть как значением, так и аксессором.
    const value: unknown = player.current
    const track: unknown = typeof value === 'function' ? (value as () => unknown)() : value
    return track !== null && track !== undefined
  }

  return (
    <>
      <div class="fixed inset-x-0 bottom-0 z-40 flex flex-col">
        <Show when={hasCurrentTrack()}>
          <MiniPlayer />
        </Show>
        <BottomNav />
      </div>

      {/* Живёт в портале Kobalte, поэтому место в разметке роли не играет. */}
      <FullPlayer />
    </>
  )
}

export function App() {
  return <Router root={AppShell}>{routes}</Router>
}
