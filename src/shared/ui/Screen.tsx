import type { ParentProps } from 'solid-js'

import { cn } from '@/shared/lib'

export type ScreenProps = ParentProps<{
  /**
   * `true` (по умолчанию) — экран сам скроллится и резервирует место под
   * мини-плеер с навигацией. `false` — экран занимает высоту вьюпорта и
   * скроллом управляет что-то внутри (например `VirtualList`).
   */
  scrollable?: boolean
  class?: string
}>

/** Каркас экрана: высота вьюпорта, скролл и отступ под плеер-стек. */
export function Screen(props: ScreenProps) {
  const scrollable = (): boolean => props.scrollable !== false

  return (
    <div
      class={cn(
        'flex h-full min-h-0 w-full flex-col',
        scrollable()
          ? 'overflow-y-auto overscroll-contain scrollbar-none pb-player-stack'
          : 'overflow-hidden',
        props.class,
      )}
    >
      {props.children}
    </div>
  )
}
