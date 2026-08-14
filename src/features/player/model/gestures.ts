/**
 * Тач-жесты плеера: свайпы по мини-плееру и закрытие полноэкранного вниз.
 * Никакой реактивности внутри — только вычисление дельты и порога.
 */

export type SwipeDirection = 'left' | 'right' | 'up' | 'down'

export type SwipeDelta = {
  dx: number
  dy: number
}

export type SwipeOptions = {
  /** Направления, на которые реагируем. */
  directions: readonly SwipeDirection[]
  onSwipe: (direction: SwipeDirection) => void
  /** Дистанция срабатывания, px. */
  threshold?: number
  /** Живая обратная связь во время жеста. */
  onMove?: (delta: SwipeDelta) => void
  /** Вызывается по завершении жеста, в том числе несостоявшегося. */
  onEnd?: () => void
}

export type SwipeHandlers = {
  onTouchStart: (event: TouchEvent) => void
  onTouchMove: (event: TouchEvent) => void
  onTouchEnd: (event: TouchEvent) => void
  onTouchCancel: (event: TouchEvent) => void
}

const DEFAULT_THRESHOLD = 60

/** Элементы с `data-no-swipe` жест не начинают: слайдеры, списки, кнопки-переключатели. */
const IGNORE_SELECTOR = '[data-no-swipe]'

export function createSwipe(options: SwipeOptions): SwipeHandlers {
  let startX = 0
  let startY = 0
  let active = false

  const threshold = (): number => options.threshold ?? DEFAULT_THRESHOLD

  const allows = (direction: SwipeDirection): boolean => options.directions.includes(direction)

  const finish = (): void => {
    if (!active) return
    active = false
    options.onEnd?.()
  }

  return {
    onTouchStart: (event) => {
      const touch = event.touches[0]
      if (touch === undefined || event.touches.length > 1) {
        finish()
        return
      }
      if (isIgnored(event.target)) return

      startX = touch.clientX
      startY = touch.clientY
      active = true
    },

    onTouchMove: (event) => {
      if (!active) return
      const touch = event.touches[0]
      if (touch === undefined) return

      options.onMove?.({ dx: touch.clientX - startX, dy: touch.clientY - startY })
    },

    onTouchEnd: (event) => {
      if (!active) return

      const touch = event.changedTouches[0]
      if (touch === undefined) {
        finish()
        return
      }

      const dx = touch.clientX - startX
      const dy = touch.clientY - startY
      const direction = resolve(dx, dy, threshold())

      finish()
      if (direction !== null && allows(direction)) options.onSwipe(direction)
    },

    onTouchCancel: () => {
      finish()
    },
  }
}

function resolve(dx: number, dy: number, threshold: number): SwipeDirection | null {
  const horizontal = Math.abs(dx) > Math.abs(dy)

  if (horizontal) {
    if (Math.abs(dx) < threshold) return null
    return dx < 0 ? 'left' : 'right'
  }

  if (Math.abs(dy) < threshold) return null
  return dy < 0 ? 'up' : 'down'
}

function isIgnored(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(IGNORE_SELECTOR) !== null
}
