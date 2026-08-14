import { createSignal } from 'solid-js'

import { favoriteToggle, type Track } from '@/shared/ipc'
import { toast } from '@/shared/ui/Toasts'

/**
 * Локальные переопределения «избранного»: экраны держат свои копии `Track`,
 * а флаг меняется из плеера и из меню трека. Карта живёт на сеанс и
 * перекрывает значение, пришедшее из библиотеки.
 */

const [overrides, setOverrides] = createSignal<ReadonlyMap<number, boolean>>(new Map())

function put(trackId: number, value: boolean): void {
  setOverrides((previous) => {
    const next = new Map(previous)
    next.set(trackId, value)
    return next
  })
}

export function isFavorite(track: Track): boolean {
  return overrides().get(track.id) ?? track.isFavorite
}

/** Оптимистично переключает флаг; при ошибке возвращает прежнее значение. */
export function toggleFavorite(track: Track): void {
  const previous = isFavorite(track)
  put(track.id, !previous)

  favoriteToggle(track.id)
    .then((value) => {
      put(track.id, value)
    })
    .catch((error: unknown) => {
      put(track.id, previous)
      console.error(`[player] не удалось изменить избранное для ${track.id}`, error)
      toast({ title: 'Не удалось изменить избранное', variant: 'danger' })
    })
}
