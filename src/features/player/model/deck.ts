export type DeckNeighbors<T> = {
  previous: T | null
  current: T | null
  next: T | null
}

export const DECK_COMMIT_FRACTION = 0.25
export const DECK_MAX_DRAG_FRACTION = 0.42

const DECK_RUBBER_BAND_RESISTANCE = 0.35
const FAST_FLICK_VELOCITY = 0.5

export function deckNeighbors<T>(items: readonly T[], index: number | null): DeckNeighbors<T> {
  if (index === null || !Number.isInteger(index) || index < 0 || index >= items.length) {
    return { previous: null, current: null, next: null }
  }

  return {
    previous: items[index - 1] ?? null,
    current: items[index] ?? null,
    next: items[index + 1] ?? null,
  }
}

export function clampDeckDrag(dx: number, viewportWidth: number): number {
  if (!Number.isFinite(dx) || !isValidWidth(viewportWidth)) return 0

  const maximumDrag = viewportWidth * DECK_MAX_DRAG_FRACTION
  const absoluteDrag = Math.abs(dx)
  if (absoluteDrag <= maximumDrag) return dx

  const resistedDrag = maximumDrag + (absoluteDrag - maximumDrag) * DECK_RUBBER_BAND_RESISTANCE
  return Math.sign(dx) * resistedDrag
}

export function shouldCommitDeckSwipe(dx: number, velocityX: number, width: number): boolean {
  if (!Number.isFinite(dx) || !Number.isFinite(velocityX) || !isValidWidth(width)) return false
  if (Math.abs(dx) >= width * DECK_COMMIT_FRACTION) return true

  return Math.abs(velocityX) >= FAST_FLICK_VELOCITY && Math.sign(dx) === Math.sign(velocityX)
}

function isValidWidth(width: number): boolean {
  return Number.isFinite(width) && width > 0
}
