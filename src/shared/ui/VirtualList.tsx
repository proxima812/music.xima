import { createVirtualizer } from '@tanstack/solid-virtual'
import { createMemo, createSignal, For, onMount, Show, type JSX } from 'solid-js'

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

  const rows = createMemo(() => virtualizer.getVirtualItems())

  /**
   * Ключи для `<For>`. `getVirtualItems()` отдаёт новые объекты на каждом кадре
   * прокрутки, а `<For>` сравнивает элементы по ссылке — по таким ключам он
   * уничтожал и заново создавал **все** строки списка на каждом кадре. Числовые
   * индексы сравниваются по значению, поэтому DOM переиспользуется, а
   * создаются и удаляются только строки на краях окна.
   */
  const visibleIndexes = createMemo(() => rows().map((row) => row.index))

  /**
   * Высота распорки. До `onMount` виртуализатор не знает контейнера и отдаёт
   * `0` — то есть на первой раскладке контейнер выглядит пустым.
   *
   * WebView решает, прокручиваемый ли блок, именно в этот момент, и **больше к
   * этому решению не возвращается**: когда контент вырастает, скролл-узел не
   * пересоздаётся. На устройстве это выглядело как «список вообще не листается
   * пальцем», хотя `scrollTop` из кода работал. Ни смена `overflow`, ни
   * пересборка стилей, ни изъятие элемента из DOM решение не отменяли —
   * прокручивался только контейнер, созданный уже с содержимым.
   *
   * Поэтому на первый кадр считаем высоту сами, из тех же оценок размера.
   */
  const totalSize = createMemo(() => {
    const measured = virtualizer.getTotalSize()
    if (measured > 0) return measured

    let sum = 0
    for (let index = 0; index < props.items.length; index += 1) sum += estimateSize(index)
    return sum
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
        style={{ height: `${totalSize()}px` }}
      >
        <For each={visibleIndexes()}>
          {(index) => {
            // Строку ищем по индексу, а не берём объект из `<For>`: он новый на
            // каждом кадре, и вместе с ним пересоздавался бы весь DOM.
            const row = createMemo(() => rows().find((candidate) => candidate.index === index))

            return (
              <Show when={row()}>
                {(current) => (
                  <div
                    class="absolute inset-x-0 top-0"
                    style={{
                      height: `${current().size}px`,
                      transform: `translateY(${current().start}px)`,
                    }}
                  >
                    <Show when={props.items[index]}>{(item) => props.children(item(), index)}</Show>
                  </div>
                )}
              </Show>
            )
          }}
        </For>
      </div>
    </div>
  )
}
