import { useNavigate } from '@solidjs/router'
import { CircleAlert, Music, Play, Shuffle } from 'lucide-solid'
import { For, Show, type JSX } from 'solid-js'

import { cn } from '@/shared/lib'
import { Button, EmptyState, Skeleton } from '@/shared/ui'

/** Общие состояния списков библиотеки: скелетоны, ошибка, пустая библиотека. */

/** Одна сетка на всё приложение: три колонки. Литерал — его видит сканер Tailwind. */
export const GRID_COLUMNS = 'grid-cols-3'

function range(count: number): number[] {
  return Array.from({ length: count }, (_, index) => index)
}

export type ListSkeletonProps = {
  rows?: number
  class?: string
}

/** Заглушка списка треков: те же 64px в строке, что и у `TrackRow`. */
export function ListSkeleton(props: ListSkeletonProps) {
  return (
    <div class={cn('flex flex-col', props.class)}>
      <For each={range(props.rows ?? 8)}>
        {() => (
          <div class="flex h-16 items-center gap-3 px-4">
            <Skeleton class="size-12 shrink-0 rounded-xl" />
            <div class="flex min-w-0 flex-1 flex-col gap-2">
              <Skeleton class="h-3.5 w-2/3 rounded-md" />
              <Skeleton class="h-3 w-2/5 rounded-md" />
            </div>
          </div>
        )}
      </For>
    </div>
  )
}

export type GridSkeletonProps = {
  columns: string
  cells?: number
  round?: boolean
  class?: string
}

/** Заглушка сетки обложек под текущий размер сетки. */
export function GridSkeleton(props: GridSkeletonProps) {
  return (
    <div class={cn('grid gap-3 px-4 pt-2', props.columns, props.class)}>
      <For each={range(props.cells ?? 9)}>
        {() => (
          <div class="flex flex-col gap-2">
            <Skeleton
              class={cn('aspect-square w-full', props.round === true ? 'rounded-full' : 'rounded-xl')}
            />
            <Skeleton class="h-3 w-3/4 rounded-md" />
          </div>
        )}
      </For>
    </div>
  )
}

export type PlaybackButtonsProps = {
  onPlay: () => void
  onShuffle: () => void
  disabled?: boolean
  class?: string
}

/** Пара кнопок «Слушать» / «Перемешать» для детальных экранов. */
export function PlaybackButtons(props: PlaybackButtonsProps) {
  return (
    <div class={cn('flex w-full items-center gap-2', props.class)}>
      <Button
        variant="primary"
        fullWidth
        disabled={props.disabled === true}
        onClick={() => props.onPlay()}
      >
        <Play size={18} aria-hidden="true" />
        Слушать
      </Button>
      <Button
        variant="secondary"
        fullWidth
        disabled={props.disabled === true}
        onClick={() => props.onShuffle()}
      >
        <Shuffle size={18} aria-hidden="true" />
        Перемешать
      </Button>
    </div>
  )
}

export type ErrorStateProps = {
  message: string
  onRetry: () => void
  title?: string
}

/** Ошибка загрузки: текст из ядра плюс кнопка повтора. */
export function ErrorState(props: ErrorStateProps) {
  return (
    <EmptyState
      icon={<CircleAlert aria-hidden="true" />}
      title={props.title ?? 'Не удалось загрузить'}
      description={props.message}
      action={
        <Button variant="secondary" onClick={() => props.onRetry()}>
          Повторить
        </Button>
      }
    />
  )
}

export type EmptyLibraryStateProps = {
  title?: string
  description?: string
  icon?: JSX.Element
  /** Показывать кнопку перехода в настройки (по умолчанию — да). */
  withSettings?: boolean
}

/** Пусто: библиотека не отсканирована — уводим в настройки за папкой с музыкой. */
export function EmptyLibraryState(props: EmptyLibraryStateProps) {
  const navigate = useNavigate()

  return (
    <EmptyState
      icon={props.icon ?? <Music aria-hidden="true" />}
      title={props.title ?? 'Здесь пока пусто'}
      description={props.description ?? 'Добавьте папку с музыкой в настройках'}
      action={
        <Show when={props.withSettings !== false}>
          <Button
            variant="primary"
            onClick={() => {
              navigate('/settings')
            }}
          >
            Открыть настройки
          </Button>
        </Show>
      }
    />
  )
}
