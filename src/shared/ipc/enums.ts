/**
 * Рантайм-списки значений enum-ов из CONTRACTS.md.
 * `types.ts` содержит только типы, поэтому всё, что нужно в рантайме
 * (списки для селектов, валидация настроек), живёт здесь.
 */

import type {
  NumOp,
  PlaybackStatus,
  RepeatMode,
  ScanMode,
  SmartRuleKind,
  SmartSort,
  TrackSort,
} from './types'

export const REPEAT_MODES = ['OFF', 'ALL', 'ONE'] as const satisfies readonly RepeatMode[]

export const PLAYBACK_STATUSES = [
  'IDLE',
  'BUFFERING',
  'PLAYING',
  'PAUSED',
  'ENDED',
] as const satisfies readonly PlaybackStatus[]

export const TRACK_SORTS = [
  'TITLE_ASC',
  'TITLE_DESC',
  'ARTIST_ASC',
  'ALBUM_ASC',
  'DATE_ADDED_DESC',
  'DATE_ADDED_ASC',
  'DURATION_ASC',
  'DURATION_DESC',
  'PLAY_COUNT_DESC',
  'LAST_PLAYED_DESC',
  'YEAR_DESC',
] as const satisfies readonly TrackSort[]

export const SMART_SORTS = [
  'TITLE_ASC',
  'DATE_ADDED_DESC',
  'PLAY_COUNT_DESC',
  'LAST_PLAYED_DESC',
  'YEAR_DESC',
  'RANDOM',
] as const satisfies readonly SmartSort[]

export const SMART_RULE_KINDS = [
  'PLAY_COUNT',
  'NEVER_PLAYED',
  'LAST_PLAYED_BEFORE_DAYS',
  'ADDED_WITHIN_DAYS',
  'FAVORITE',
  'YEAR_BETWEEN',
  'BITRATE_AT_LEAST',
  'DURATION_BETWEEN_MS',
  'GENRE_IS',
  'ARTIST_IS',
  'TITLE_CONTAINS',
] as const satisfies readonly SmartRuleKind[]

export const NUM_OPS = ['EQ', 'NE', 'LT', 'LTE', 'GT', 'GTE'] as const satisfies readonly NumOp[]

export const SCAN_MODES = ['FULL', 'INCREMENTAL'] as const satisfies readonly ScanMode[]

/** Проверка «значение принадлежит списку» без `any`. */
export function oneOf<T extends string>(value: unknown, allowed: readonly T[]): T | undefined {
  if (typeof value !== 'string') return undefined
  const list: readonly string[] = allowed
  return list.includes(value) ? (value as T) : undefined
}
