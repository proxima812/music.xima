export type DeckNeighbors<T> = {
  previous: T | null
  current: T | null
  next: T | null
}

export const DECK_COMMIT_FRACTION = 0.25
export const DECK_MAX_DRAG_FRACTION = 0.42

/** Насколько далеко тянется дека у края очереди: дальше ехать некуда. */
export const DECK_EDGE_DRAG_FRACTION = 0.06

const DECK_RUBBER_BAND_RESISTANCE = 0.35
const DECK_EDGE_RESISTANCE = 0.12
const FAST_FLICK_VELOCITY = 0.5

/**
 * Соседние карточки деки.
 *
 * `wrap` включается при повторе всей очереди: кнопка «дальше» на последнем
 * треке уводит на первый, значит и свайп обязан вести туда же — иначе кнопка и
 * жест расходятся ровно на краях. Очередь из одного трека не заворачивается:
 * соседом стал бы он сам.
 */
export function deckNeighbors<T>(
  items: readonly T[],
  index: number | null,
  wrap = false,
): DeckNeighbors<T> {
  if (index === null || !Number.isInteger(index) || index < 0 || index >= items.length) {
    return { previous: null, current: null, next: null }
  }

  const canWrap = wrap && items.length > 1

  const previous = index > 0 ? items[index - 1] : canWrap ? items[items.length - 1] : undefined
  const next = index < items.length - 1 ? items[index + 1] : canWrap ? items[0] : undefined

  return {
    previous: previous ?? null,
    current: items[index] ?? null,
    next: next ?? null,
  }
}

/**
 * Смещение деки под палец.
 *
 * `hasAdjacent` — есть ли карточка в ту сторону, куда тянут. У края очереди
 * ехать некуда: тянем на несколько процентов и упираемся, иначе из-под обложки
 * выезжает пустой фон и это читается как поломка, а не как «дальше ничего нет».
 */
export function clampDeckDrag(dx: number, viewportWidth: number, hasAdjacent = true): number {
  if (!Number.isFinite(dx) || !isValidWidth(viewportWidth)) return 0

  const maximumDrag = viewportWidth * (hasAdjacent ? DECK_MAX_DRAG_FRACTION : DECK_EDGE_DRAG_FRACTION)
  const resistance = hasAdjacent ? DECK_RUBBER_BAND_RESISTANCE : DECK_EDGE_RESISTANCE

  const absoluteDrag = Math.abs(dx)
  if (absoluteDrag <= maximumDrag) return dx

  const resistedDrag = maximumDrag + (absoluteDrag - maximumDrag) * resistance
  return Math.sign(dx) * resistedDrag
}

/**
 * `hasAdjacent === false` — у края очереди свайп не засчитывается никогда:
 * ни по расстоянию, ни по скорости флика.
 */
export function shouldCommitDeckSwipe(
  dx: number,
  velocityX: number,
  width: number,
  hasAdjacent = true,
): boolean {
  if (!hasAdjacent) return false
  if (!Number.isFinite(dx) || !Number.isFinite(velocityX) || !isValidWidth(width)) return false
  if (Math.abs(dx) >= width * DECK_COMMIT_FRACTION) return true

  return Math.abs(velocityX) >= FAST_FLICK_VELOCITY && Math.sign(dx) === Math.sign(velocityX)
}

function isValidWidth(width: number): boolean {
  return Number.isFinite(width) && width > 0
}
