import { ArrowUpDown, Check } from 'lucide-solid'

import { TRACK_SORTS, type TrackSort } from '@/shared/ipc'
import { Menu, type MenuAction } from '@/shared/ui'

/** Подписи вариантов `TrackSort` (§1.6). Порядок пунктов — как в контракте. */
const SORT_LABELS: Record<TrackSort, string> = {
  TITLE_ASC: 'Название: А→Я',
  TITLE_DESC: 'Название: Я→А',
  ARTIST_ASC: 'Исполнитель: А→Я',
  ALBUM_ASC: 'Альбом',
  DATE_ADDED_DESC: 'Сначала новые',
  DATE_ADDED_ASC: 'Сначала старые',
  DURATION_ASC: 'Сначала короткие',
  DURATION_DESC: 'Сначала длинные',
  PLAY_COUNT_DESC: 'Чаще всего слушаю',
  LAST_PLAYED_DESC: 'Недавно прослушанные',
  YEAR_DESC: 'Год: новые первыми',
}

export function sortLabel(sort: TrackSort): string {
  return SORT_LABELS[sort]
}

export type SortMenuProps = {
  value: TrackSort
  onChange: (sort: TrackSort) => void
}

/** Кнопка сортировки списка песен: все варианты `TrackSort`, текущий с галочкой. */
export function SortMenu(props: SortMenuProps) {
  const items = (): MenuAction[] =>
    TRACK_SORTS.map((sort) => {
      const action: MenuAction = {
        label: SORT_LABELS[sort],
        onSelect: () => {
          props.onChange(sort)
        },
      }

      return sort === props.value
        ? { ...action, icon: <Check size={16} aria-hidden="true" /> }
        : action
    })

  return (
    <Menu
      items={items()}
      label={`Сортировка: ${SORT_LABELS[props.value]}`}
      placement="bottom-end"
      trigger={<ArrowUpDown size={18} aria-hidden="true" />}
    />
  )
}
