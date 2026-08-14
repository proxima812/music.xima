/**
 * Форматирование для UI. Чистые функции, без Solid и без IPC.
 * Локаль — русская, она в приложении единственная.
 */

const MS_IN_DAY = 86_400_000

const BYTE_UNITS = ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ'] as const

const MONTHS_GENITIVE = [
  'января',
  'февраля',
  'марта',
  'апреля',
  'мая',
  'июня',
  'июля',
  'августа',
  'сентября',
  'октября',
  'ноября',
  'декабря',
] as const

const DAY_FORMS = ['день', 'дня', 'дней'] as const

const numberFormats = new Map<number, Intl.NumberFormat>()

function formatNumber(value: number, digits: number): string {
  let formatter = numberFormats.get(digits)
  if (formatter === undefined) {
    formatter = new Intl.NumberFormat('ru-RU', {
      minimumFractionDigits: digits,
      maximumFractionDigits: digits,
    })
    numberFormats.set(digits, formatter)
  }
  return formatter.format(value)
}

const pad2 = (value: number): string => String(value).padStart(2, '0')

/** Длительность трека: `3:51`, `1:02:11`. Отрицательные и NaN → `0:00`. */
export function formatDuration(ms: number): string {
  const totalSeconds = Number.isFinite(ms) && ms > 0 ? Math.floor(ms / 1000) : 0
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60

  return hours > 0
    ? `${hours}:${pad2(minutes)}:${pad2(seconds)}`
    : `${minutes}:${pad2(seconds)}`
}

/** Длительность словами: `45 с`, `12 мин`, `3 ч 14 мин`. Для статистики и итогов. */
export function formatDurationHuman(ms: number): string {
  const totalSeconds = Number.isFinite(ms) && ms > 0 ? Math.floor(ms / 1000) : 0
  if (totalSeconds < 60) return `${totalSeconds} с`

  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)

  if (hours === 0) return `${formatCount(minutes)} мин`
  if (minutes === 0) return `${formatCount(hours)} ч`
  return `${formatCount(hours)} ч ${minutes} мин`
}

/** Количество с разбивкой разрядов: `312`, `1 234`. */
export function formatCount(value: number): string {
  if (!Number.isFinite(value)) return '0'
  return formatNumber(Math.trunc(value), 0)
}

/** Русская форма слова по числу: `pluralize(3, ['трек', 'трека', 'треков']) === 'трека'`. */
export function pluralize(value: number, forms: readonly [string, string, string]): string {
  const abs = Math.abs(Math.trunc(value))
  const tail100 = abs % 100
  if (tail100 > 10 && tail100 < 20) return forms[2]

  const tail10 = abs % 10
  if (tail10 === 1) return forms[0]
  if (tail10 > 1 && tail10 < 5) return forms[1]
  return forms[2]
}

/** `formatPlural(3, ['трек', 'трека', 'треков']) === '3 трека'`. */
export function formatPlural(value: number, forms: readonly [string, string, string]): string {
  return `${formatCount(value)} ${pluralize(value, forms)}`
}

/** Размер файла: `512 Б`, `4,2 МБ`, `1,5 ГБ`. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 Б'

  let value = bytes
  let unit = 0
  while (value >= 1024 && unit < BYTE_UNITS.length - 1) {
    value /= 1024
    unit += 1
  }

  const digits = unit === 0 ? 0 : value < 10 ? 1 : 0
  return `${formatNumber(value, digits)} ${BYTE_UNITS[unit] ?? 'Б'}`
}

/** Абсолютная дата: `12 марта` для текущего года, иначе `12 марта 2023`. */
export function formatDate(ms: number, now: number = Date.now()): string {
  if (!Number.isFinite(ms)) return ''

  const date = new Date(ms)
  const month = MONTHS_GENITIVE[date.getMonth()] ?? ''
  const head = `${date.getDate()} ${month}`.trim()

  return date.getFullYear() === new Date(now).getFullYear()
    ? head
    : `${head} ${date.getFullYear()}`
}

/** `сегодня`, `вчера`, `3 дня назад`, дальше недели — абсолютная дата. */
export function formatRelativeDate(ms: number, now: number = Date.now()): string {
  if (!Number.isFinite(ms)) return ''

  const days = Math.round((startOfDay(now) - startOfDay(ms)) / MS_IN_DAY)
  if (days <= 0) return 'сегодня'
  if (days === 1) return 'вчера'
  if (days < 7) return `${days} ${pluralize(days, DAY_FORMS)} назад`
  return formatDate(ms, now)
}

function startOfDay(ms: number): number {
  const date = new Date(ms)
  date.setHours(0, 0, 0, 0)
  return date.getTime()
}
