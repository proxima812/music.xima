/**
 * Модель правил умного плейлиста: ярлыки, набор полей, значения по умолчанию,
 * человекочитаемое описание и валидация. Типы правил берутся из `@/shared/ipc`
 * (зеркало CONTRACTS §1.4) и здесь не дублируются.
 *
 * Валидация повторяет `SmartRule::validate()` из ядра, но на русском и до IPC:
 * пользователь видит ошибку сразу, а не после отказа команды.
 */

import type {
  NumOp,
  SmartPlaylist,
  SmartPlaylistDraft,
  SmartRule,
  SmartRuleKind,
  SmartSort,
} from '@/shared/ipc'
import { formatCount, formatDuration, formatPlural } from '@/shared/lib'

/** Ограничение имени из `sanitize_playlist_name` в ядре. */
export const MAX_SMART_PLAYLIST_NAME_LEN = 120

/** Пресеты качества для `BITRATE_AT_LEAST`, кбит/с. */
export const BITRATE_PRESETS_KBPS: readonly number[] = [128, 192, 256, 320]

const DAY_FORMS: readonly [string, string, string] = ['день', 'дня', 'дней']

const TRACK_FORMS: readonly [string, string, string] = ['трек', 'трека', 'треков']

export const NUM_OP_LABELS: Record<NumOp, string> = {
  EQ: 'равно',
  NE: 'не равно',
  LT: 'меньше',
  LTE: 'не больше',
  GT: 'больше',
  GTE: 'не меньше',
}

const NUM_OP_SIGNS: Record<NumOp, string> = {
  EQ: '=',
  NE: '≠',
  LT: '<',
  LTE: '≤',
  GT: '>',
  GTE: '≥',
}

export const SMART_SORT_LABELS: Record<SmartSort, string> = {
  TITLE_ASC: 'По названию',
  DATE_ADDED_DESC: 'Сначала новые',
  PLAY_COUNT_DESC: 'Самые слушаемые',
  LAST_PLAYED_DESC: 'Недавно слушали',
  YEAR_DESC: 'Сначала свежие годы',
  RANDOM: 'В случайном порядке',
}

/** Тип поля правила — определяет, какой контрол рисует `RuleEditor`. */
export type RuleFieldType =
  | 'NUM_OP'
  | 'COUNT'
  | 'DAYS'
  | 'YEAR'
  | 'SECONDS'
  | 'BITRATE'
  | 'TEXT'
  | 'GENRE'
  | 'ARTIST'
  | 'BOOLEAN'

/** Имя поля внутри правила — совпадает с ключом в JSON правила. */
export type RuleFieldName = 'op' | 'value' | 'days' | 'from' | 'to' | 'artistId'

export type RuleField = {
  name: RuleFieldName
  type: RuleFieldType
  label: string
}

export type RuleDescriptor = {
  kind: SmartRuleKind
  /** Название правила в списке выбора. */
  label: string
  /** Пояснение под названием. */
  hint: string
  fields: readonly RuleField[]
  /** Новое правило этого типа со значениями по умолчанию. */
  create: () => SmartRule
}

export const RULE_DESCRIPTORS: Record<SmartRuleKind, RuleDescriptor> = {
  PLAY_COUNT: {
    kind: 'PLAY_COUNT',
    label: 'Количество прослушиваний',
    hint: 'Сравнивает счётчик прослушиваний трека с числом',
    fields: [
      { name: 'op', type: 'NUM_OP', label: 'Условие' },
      { name: 'value', type: 'COUNT', label: 'Прослушиваний' },
    ],
    create: () => ({ kind: 'PLAY_COUNT', op: 'GTE', value: 5 }),
  },
  NEVER_PLAYED: {
    kind: 'NEVER_PLAYED',
    label: 'Ни разу не играли',
    hint: 'Треки без единого прослушивания',
    fields: [],
    create: () => ({ kind: 'NEVER_PLAYED' }),
  },
  LAST_PLAYED_BEFORE_DAYS: {
    kind: 'LAST_PLAYED_BEFORE_DAYS',
    label: 'Давно не слушали',
    hint: 'Последнее прослушивание было раньше, чем указано',
    fields: [{ name: 'days', type: 'DAYS', label: 'Дней без прослушивания' }],
    create: () => ({ kind: 'LAST_PLAYED_BEFORE_DAYS', days: 60 }),
  },
  ADDED_WITHIN_DAYS: {
    kind: 'ADDED_WITHIN_DAYS',
    label: 'Недавно добавлены',
    hint: 'Треки, попавшие в библиотеку за последние дни',
    fields: [{ name: 'days', type: 'DAYS', label: 'Дней с добавления' }],
    create: () => ({ kind: 'ADDED_WITHIN_DAYS', days: 30 }),
  },
  FAVORITE: {
    kind: 'FAVORITE',
    label: 'Избранное',
    hint: 'Треки в избранном либо, наоборот, вне его',
    fields: [{ name: 'value', type: 'BOOLEAN', label: 'В избранном' }],
    create: () => ({ kind: 'FAVORITE', value: true }),
  },
  YEAR_BETWEEN: {
    kind: 'YEAR_BETWEEN',
    label: 'Год выпуска',
    hint: 'Год трека попадает в диапазон',
    fields: [
      { name: 'from', type: 'YEAR', label: 'С года' },
      { name: 'to', type: 'YEAR', label: 'По год' },
    ],
    create: () => ({ kind: 'YEAR_BETWEEN', from: 2020, to: 2029 }),
  },
  BITRATE_AT_LEAST: {
    kind: 'BITRATE_AT_LEAST',
    label: 'Качество',
    hint: 'Битрейт не ниже выбранного',
    fields: [{ name: 'value', type: 'BITRATE', label: 'Битрейт' }],
    create: () => ({ kind: 'BITRATE_AT_LEAST', value: kbpsToBitrate(320) }),
  },
  DURATION_BETWEEN_MS: {
    kind: 'DURATION_BETWEEN_MS',
    label: 'Длительность',
    hint: 'Длина трека попадает в диапазон',
    fields: [
      { name: 'from', type: 'SECONDS', label: 'От, секунд' },
      { name: 'to', type: 'SECONDS', label: 'До, секунд' },
    ],
    create: () => ({ kind: 'DURATION_BETWEEN_MS', from: 60_000, to: 600_000 }),
  },
  GENRE_IS: {
    kind: 'GENRE_IS',
    label: 'Жанр',
    hint: 'Жанр трека совпадает с выбранным',
    fields: [{ name: 'value', type: 'GENRE', label: 'Жанр' }],
    create: () => ({ kind: 'GENRE_IS', value: '' }),
  },
  ARTIST_IS: {
    kind: 'ARTIST_IS',
    label: 'Исполнитель',
    hint: 'Треки одного исполнителя',
    fields: [{ name: 'artistId', type: 'ARTIST', label: 'Исполнитель' }],
    create: () => ({ kind: 'ARTIST_IS', artistId: 0 }),
  },
  TITLE_CONTAINS: {
    kind: 'TITLE_CONTAINS',
    label: 'Название содержит',
    hint: 'Поиск по названию трека',
    fields: [{ name: 'value', type: 'TEXT', label: 'Название содержит' }],
    create: () => ({ kind: 'TITLE_CONTAINS', value: '' }),
  },
}

/** Порядок правил в списке выбора: сверху то, что нужно чаще. */
export const RULE_KIND_ORDER: readonly SmartRuleKind[] = [
  'ADDED_WITHIN_DAYS',
  'PLAY_COUNT',
  'NEVER_PLAYED',
  'LAST_PLAYED_BEFORE_DAYS',
  'FAVORITE',
  'GENRE_IS',
  'ARTIST_IS',
  'YEAR_BETWEEN',
  'DURATION_BETWEEN_MS',
  'BITRATE_AT_LEAST',
  'TITLE_CONTAINS',
]

export function ruleDescriptor(kind: SmartRuleKind): RuleDescriptor {
  return RULE_DESCRIPTORS[kind]
}

/** Новое правило выбранного типа со значениями по умолчанию. */
export function createRule(kind: SmartRuleKind): SmartRule {
  return RULE_DESCRIPTORS[kind].create()
}

/** Подпись поля правила; для неизвестной пары возвращает имя типа правила. */
export function ruleFieldLabel(kind: SmartRuleKind, name: RuleFieldName): string {
  const field = RULE_DESCRIPTORS[kind].fields.find((item) => item.name === name)
  return field === undefined ? RULE_DESCRIPTORS[kind].label : field.label
}

/**
 * Битрейт хранится в битах в секунду (так его отдаёт Android), а показываем
 * кбит/с. Значения из старых правил в кбит/с тоже читаются корректно.
 */
export function bitrateToKbps(value: number): number {
  if (!Number.isFinite(value) || value <= 0) return 0
  return value >= 1000 ? Math.round(value / 1000) : Math.round(value)
}

export function kbpsToBitrate(kbps: number): number {
  if (!Number.isFinite(kbps) || kbps <= 0) return 0
  return Math.round(kbps) * 1000
}

export type RuleNames = {
  /** Имена исполнителей для `ARTIST_IS`; без них правило описывается номером. */
  artistNames?: ReadonlyMap<number, string>
}

/** Человекочитаемое описание одного правила. */
export function describeRule(rule: SmartRule, names?: RuleNames): string {
  switch (rule.kind) {
    case 'PLAY_COUNT':
      return `Прослушиваний ${NUM_OP_SIGNS[rule.op]} ${formatCount(rule.value)}`
    case 'NEVER_PLAYED':
      return 'Ни разу не играли'
    case 'LAST_PLAYED_BEFORE_DAYS':
      return `Не слушали больше ${formatPlural(rule.days, DAY_FORMS)}`
    case 'ADDED_WITHIN_DAYS':
      return `Добавлены за ${formatPlural(rule.days, DAY_FORMS)}`
    case 'FAVORITE':
      return rule.value ? 'В избранном' : 'Не в избранном'
    case 'YEAR_BETWEEN':
      return rule.from === rule.to
        ? `Год ${String(rule.from)}`
        : `Годы ${String(rule.from)}–${String(rule.to)}`
    case 'BITRATE_AT_LEAST':
      return `Битрейт от ${formatCount(bitrateToKbps(rule.value))} кбит/с`
    case 'DURATION_BETWEEN_MS':
      return `Длительность ${formatDuration(rule.from)}–${formatDuration(rule.to)}`
    case 'GENRE_IS':
      return rule.value.trim() === '' ? 'Жанр не выбран' : `Жанр «${rule.value}»`
    case 'ARTIST_IS':
      return describeArtist(rule.artistId, names)
    case 'TITLE_CONTAINS':
      return rule.value.trim() === ''
        ? 'Название не задано'
        : `Название содержит «${rule.value}»`
  }
}

/** Описание всего набора правил одной строкой — для списка плейлистов. */
export function describeRules(
  rules: readonly SmartRule[],
  matchAll: boolean,
  names?: RuleNames,
): string {
  if (rules.length === 0) return 'Все треки библиотеки'
  return rules.map((rule) => describeRule(rule, names)).join(matchAll ? ' и ' : ' или ')
}

/** Ошибка правила на русском или `null`, если правило корректно. */
export function validateRule(rule: SmartRule): string | null {
  switch (rule.kind) {
    case 'PLAY_COUNT':
      return isCount(rule.value) ? null : 'Количество прослушиваний должно быть от нуля'
    case 'NEVER_PLAYED':
    case 'FAVORITE':
      return null
    case 'LAST_PLAYED_BEFORE_DAYS':
    case 'ADDED_WITHIN_DAYS':
      return isPositive(rule.days) ? null : 'Число дней должно быть больше нуля'
    case 'YEAR_BETWEEN': {
      if (!isYear(rule.from) || !isYear(rule.to)) return 'Год указан неверно'
      return rule.from <= rule.to ? null : 'Начальный год больше конечного'
    }
    case 'BITRATE_AT_LEAST':
      return isCount(rule.value) ? null : 'Битрейт не может быть отрицательным'
    case 'DURATION_BETWEEN_MS': {
      if (!isCount(rule.from) || !isCount(rule.to)) return 'Длительность не может быть отрицательной'
      return rule.from <= rule.to ? null : 'Начальная длительность больше конечной'
    }
    case 'GENRE_IS':
      return rule.value.trim() === '' ? 'Выберите жанр' : null
    case 'ARTIST_IS':
      return isPositive(rule.artistId) ? null : 'Выберите исполнителя'
    case 'TITLE_CONTAINS':
      return rule.value.trim() === '' ? 'Введите текст для поиска по названию' : null
  }
}

/** Ошибка черновика целиком: имя, лимит, каждое правило. */
export function validateDraft(draft: SmartPlaylistDraft): string | null {
  const name = draft.name.trim()
  if (name === '') return 'Введите название плейлиста'
  if (name.length > MAX_SMART_PLAYLIST_NAME_LEN) {
    return `Название длиннее ${String(MAX_SMART_PLAYLIST_NAME_LEN)} символов`
  }

  if (draft.limit !== null && !isPositive(draft.limit)) {
    return 'Лимит должен быть больше нуля'
  }

  for (const rule of draft.rules) {
    const error = validateRule(rule)
    if (error !== null) return error
  }
  return null
}

/** Пустой черновик для экрана создания. */
export function emptyDraft(): SmartPlaylistDraft {
  return { name: '', rules: [], matchAll: true, sort: 'TITLE_ASC', limit: null }
}

/** Черновик из сохранённого плейлиста — для экрана редактирования. */
export function draftFrom(playlist: SmartPlaylist): SmartPlaylistDraft {
  return {
    name: playlist.name,
    rules: playlist.rules.map((rule) => ({ ...rule })),
    matchAll: playlist.matchAll,
    sort: playlist.sort,
    limit: playlist.limit,
  }
}

/** `12 треков` — подпись под плейлистом и в предпросмотре. */
export function formatTrackCount(count: number): string {
  return formatPlural(count, TRACK_FORMS)
}

function describeArtist(artistId: number, names?: RuleNames): string {
  if (!isPositive(artistId)) return 'Исполнитель не выбран'
  const name = names?.artistNames?.get(artistId)
  return name === undefined ? `Исполнитель № ${String(artistId)}` : `Исполнитель «${name}»`
}

function isCount(value: number): boolean {
  return Number.isInteger(value) && value >= 0
}

function isPositive(value: number): boolean {
  return Number.isInteger(value) && value > 0
}

function isYear(value: number): boolean {
  return Number.isInteger(value) && value >= 1 && value <= 9999
}
