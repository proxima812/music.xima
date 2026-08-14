import { createVirtualizer } from '@tanstack/solid-virtual'
import { createSignal, For, onMount, Show, type JSX } from 'solid-js'

import { cn } from '@/shared/lib'

const DEFAULT_OVERSCAN = 6

export type VirtualListProps<T> = {
  items: readonly T[]
  /** Ожидаемая высота строки в пикселях либо функция от индекса. */
  estimateSize: number | ((index: number) => number)
  overscan?: number
  class?: string
  /** Классы внутреннего контейнера — например отступ под плеер-стек. */
  contentClass?: string
  children: (item: T, index: number) => JSX.Element
}

/**
 * Виртуализированный список поверх `@tanstack/solid-virtual`.
 * Скроллится сам, поэтому родитель обязан задать ему высоту.
 */
export function VirtualList<T>(props: VirtualListProps<T>) {
  const [scroller, setScroller] = createSignal<HTMLDivElement | null>(null)
  let element!: HTMLDivElement

  const estimateSize = (index: number): number => {
    const estimate = props.estimateSize
    return typeof estimate === 'function' ? estimate(index) : estimate
  }

  const virtualizer = createVirtualizer<HTMLDivElement, HTMLDivElement>({
    get count() {
      return props.items.length
    },
    get overscan() {
      return props.overscan ?? DEFAULT_OVERSCAN
    },
    getScrollElement: () => scroller(),
    estimateSize,
  })

  /**
   * Ссылку отдаём виртуализатору **после** монтирования, а не в `ref`.
   * Если сделать это в `ref`, он навесит `ResizeObserver` на элемент, которого
   * ещё нет в документе: начальный размер приезжает нулём, а второго замера уже
   * не будет — на устройстве это выглядело как «6 треков» над пустым списком.
   */
  onMount(() => {
    setScroller(element)
  })

  // `flex-1` работает внутри flex-колонки (экран с TopBar), `h-full` — в обычном блоке.
  return (
    <div
      ref={element}
      class={cn(
        'h-full min-h-0 w-full flex-1 overflow-y-auto overscroll-contain scrollbar-none',
        props.class,
      )}
    >
      <div
        class={cn('relative w-full', props.contentClass)}
        style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        <For each={virtualizer.getVirtualItems()}>
          {(row) => (
            <div
              class="absolute inset-x-0 top-0"
              style={{ height: `${row.size}px`, transform: `translateY(${row.start}px)` }}
            >
              <Show when={props.items[row.index]}>
                {(item) => props.children(item(), row.index)}
              </Show>
            </div>
          )}
        </For>
      </div>
    </div>
  )
}
