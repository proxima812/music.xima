import { useNavigate, useParams } from '@solidjs/router'
import { ChevronLeft, ListPlus, Play, Plus, Sparkles } from 'lucide-solid'
import {
  createEffect,
  createResource,
  createSignal,
  For,
  Index,
  onCleanup,
  Show,
} from 'solid-js'

import { usePlayer } from '@/features/player/model/player-store'
import {
  oneOf,
  SMART_RULE_KINDS,
  SMART_SORTS,
  smartPlaylistCreate,
  smartPlaylistGet,
  smartPlaylistPreview,
  smartPlaylistUpdate,
  toIpcError,
  type SmartPlaylistDraft,
  type SmartRule,
  type Track,
} from '@/shared/ipc'
import { debounce, formatPlural, settled } from '@/shared/lib'
import {
  Button,
  EmptyState,
  IconButton,
  Screen,
  SectionHeader,
  Sheet,
  Spinner,
  Tabs,
  toast,
  TopBar,
  TrackRow,
  type TabItem,
} from '@/shared/ui'
import {
  createRule,
  draftFrom,
  emptyDraft,
  formatTrackCount,
  MAX_SMART_PLAYLIST_NAME_LEN,
  RULE_DESCRIPTORS,
  RULE_KIND_ORDER,
  SMART_SORT_LABELS,
  validateDraft,
  validateRule,
} from '../model/rules'
import { RuleEditor, SelectField, type SelectOption } from './RuleEditor'

/** Живой предпросмотр не должен бить по ядру на каждый символ. */
const PREVIEW_DEBOUNCE_MS = 400

/** Сколько треков предпросмотра показываем списком. */
const PREVIEW_ROWS = 30

const RULE_FORMS: readonly [string, string, string] = ['условие', 'условия', 'условий']

const MATCH_TABS: readonly TabItem[] = [
  { value: 'all', label: 'Все условия' },
  { value: 'any', label: 'Любое условие' },
]

const SORT_OPTIONS: readonly SelectOption[] = SMART_SORTS.map((sort) => ({
  value: sort,
  label: SMART_SORT_LABELS[sort],
}))

/** Создание и правка умного плейлиста: правила, сортировка, предпросмотр. */
export function SmartPlaylistEditorScreen() {
  const navigate = useNavigate()
  const params = useParams()
  const player = usePlayer()

  const editingId = (): number | undefined => {
    const raw = params['id']
    if (raw === undefined) return undefined
    const parsed = Number.parseInt(raw, 10)
    return Number.isNaN(parsed) ? undefined : parsed
  }

  const [existing] = createResource(editingId, (id: number) => smartPlaylistGet(id))

  // Зеркало для разметки: прямое чтение поднимает общий `<Suspense>` (BUGS.md, B8).
  const loaded = settled(existing)

  const [draft, setDraft] = createSignal<SmartPlaylistDraft>(emptyDraft())
  const [saving, setSaving] = createSignal(false)
  const [addOpen, setAddOpen] = createSignal(false)

  createEffect(() => {
    // Читать упавший ресурс нельзя: обращение к нему бросает исключение.
    if (existing.error !== undefined && existing.error !== null) return
    const playlist = existing()
    if (playlist !== undefined) setDraft(draftFrom(playlist))
  })

  // ─── предпросмотр ──────────────────────────────────────────────────────────

  const [previewDraft, setPreviewDraft] = createSignal<SmartPlaylistDraft | undefined>(undefined)

  const schedulePreview = debounce((value: SmartPlaylistDraft) => {
    setPreviewDraft(value)
  }, PREVIEW_DEBOUNCE_MS)

  onCleanup(() => {
    schedulePreview.cancel()
  })

  /**
   * Ядро отвергает черновик без правил (`INVALID_INPUT`), поэтому пустой набор
   * считаем неготовым и предпросмотр не запрашиваем вовсе.
   */
  const rulesValid = (): boolean =>
    draft().rules.length > 0 && draft().rules.every((rule) => validateRule(rule) === null)

  createEffect(() => {
    const value = draft()
    if (!rulesValid()) {
      schedulePreview.cancel()
      setPreviewDraft(undefined)
      return
    }
    schedulePreview(value)
  })

  const [preview] = createResource(previewDraft, (value: SmartPlaylistDraft) =>
    smartPlaylistPreview(value),
  )

  /**
   * Чтение `preview()` у упавшего ресурса бросает исключение, а ловить его
   * некому: ближайший `Suspense` — общий для всего приложения, и экран
   * навсегда остаётся спиннером. Поэтому сначала всегда смотрим `.error`.
   */
  const previewError = (): string | null => {
    const cause: unknown = preview.error
    return cause === undefined || cause === null ? null : toIpcError(cause).message
  }

  /**
   * Пока запрос летит, к `preview()` не прикасаемся вовсе. Ресурс, прочитанный
   * в загрузке, поднимает тот же общий `Suspense`, и правка любого условия
   * вынимала бы экран редактора из DOM прямо под руками (docs/BUGS.md, B8).
   */
  const previewTracks = (): readonly Track[] =>
    previewError() !== null || preview.loading ? [] : (preview() ?? [])

  // ─── правила ───────────────────────────────────────────────────────────────

  const updateRule = (index: number, rule: SmartRule): void => {
    setDraft((previous) => ({
      ...previous,
      rules: previous.rules.map((item, position) => (position === index ? rule : item)),
    }))
  }

  const removeRule = (index: number): void => {
    setDraft((previous) => ({
      ...previous,
      rules: previous.rules.filter((_, position) => position !== index),
    }))
  }

  const addRule = (value: string): void => {
    const kind = oneOf(value, SMART_RULE_KINDS)
    if (kind === undefined) return
    setDraft((previous) => ({ ...previous, rules: [...previous.rules, createRule(kind)] }))
    setAddOpen(false)
  }

  // ─── сохранение ────────────────────────────────────────────────────────────

  const save = (): void => {
    if (saving()) return

    const value: SmartPlaylistDraft = { ...draft(), name: draft().name.trim() }
    const error = validateDraft(value)
    if (error !== null) {
      toast({ title: error, variant: 'danger' })
      return
    }

    const id = editingId()
    setSaving(true)

    const request =
      id === undefined ? smartPlaylistCreate(value) : smartPlaylistUpdate(id, value)

    request
      .then((saved) => {
        toast({ title: `Плейлист «${saved.name}» сохранён` })
        navigate('/smart')
      })
      .catch((cause: unknown) => {
        console.error('[smart] не удалось сохранить плейлист', cause)
        toast({ title: 'Не удалось сохранить плейлист', variant: 'danger' })
      })
      .finally(() => {
        setSaving(false)
      })
  }

  const previewSummary = (): string => {
    if (draft().rules.length === 0) return 'Добавьте условие'
    if (!rulesValid()) return 'Проверьте условия'
    if (previewError() !== null) return 'Предпросмотр недоступен'
    if (preview.loading || preview() === undefined) return 'Подбираем треки'
    return formatTrackCount(previewTracks().length)
  }

  const playPreview = (startIndex: number): void => {
    const ids = previewTracks().map((track) => track.id)
    if (ids.length === 0) return
    player.playTracks(ids, startIndex)
  }

  return (
    <Screen>
      <TopBar
        title={editingId() === undefined ? 'Новый умный плейлист' : 'Правка плейлиста'}
        left={
          <IconButton
            label="Назад"
            onClick={() => {
              navigate(-1)
            }}
          >
            <ChevronLeft aria-hidden="true" />
          </IconButton>
        }
        right={
          <Button variant="primary" pending={saving()} onClick={save}>
            Сохранить
          </Button>
        }
      />

      <Show
        when={editingId() === undefined || loaded() !== undefined}
        fallback={
          <div class="flex justify-center py-10">
            <Spinner />
          </div>
        }
      >
        <div class="flex flex-col gap-5 px-4 pt-2">
          <label class="flex flex-col gap-1">
            <span class="label">Название</span>
            <input
              class="input input--secondary h-11 w-full"
              type="text"
              value={draft().name}
              maxLength={MAX_SMART_PLAYLIST_NAME_LEN}
              placeholder="Например: Забытые хиты"
              onInput={(event) => {
                const name = event.currentTarget.value
                setDraft((previous) => ({ ...previous, name }))
              }}
            />
          </label>

          <div class="flex flex-col gap-2">
            <span class="label">Совпадение</span>
            <Tabs
              items={MATCH_TABS}
              value={draft().matchAll ? 'all' : 'any'}
              onChange={(value) => {
                const matchAll = value === 'all'
                setDraft((previous) => ({ ...previous, matchAll }))
              }}
            />
          </div>
        </div>

        <section>
          <SectionHeader
            title="Условия"
            description={
              draft().rules.length === 0
                ? 'Нужно хотя бы одно условие'
                : formatPlural(draft().rules.length, RULE_FORMS)
            }
            action={
              <IconButton
                label="Добавить условие"
                variant="primary"
                onClick={() => {
                  setAddOpen(true)
                }}
              >
                <Plus aria-hidden="true" />
              </IconButton>
            }
          />

          <div class="flex flex-col gap-3 px-4">
            {/* Index, а не For: правило правится на месте, строку пересоздавать нельзя —
                иначе поле теряет фокус на каждом символе. */}
            <Index each={draft().rules}>
              {(rule, index) => (
                <RuleEditor
                  rule={rule()}
                  onChange={(next) => {
                    updateRule(index, next)
                  }}
                  onRemove={() => {
                    removeRule(index)
                  }}
                />
              )}
            </Index>

            <Show when={draft().rules.length === 0}>
              <Button
                variant="secondary"
                fullWidth
                class="min-h-12"
                onClick={() => {
                  setAddOpen(true)
                }}
              >
                <ListPlus size={18} aria-hidden="true" />
                Добавить первое условие
              </Button>
            </Show>
          </div>
        </section>

        <div class="flex flex-col gap-5 px-4 pt-5">
          <SelectField
            label="Сортировка"
            value={draft().sort}
            options={SORT_OPTIONS}
            onChange={(value) => {
              const sort = oneOf(value, SMART_SORTS)
              if (sort !== undefined) setDraft((previous) => ({ ...previous, sort }))
            }}
          />

          <label class="flex flex-col gap-1">
            <span class="label">Ограничение</span>
            <input
              class="input input--secondary h-11 w-full tabular-nums"
              type="number"
              inputmode="numeric"
              min={1}
              max={10_000}
              placeholder="Без ограничения"
              value={draft().limit === null ? '' : String(draft().limit)}
              onInput={(event) => {
                const raw = event.currentTarget.value.trim()
                const parsed = Number.parseInt(raw, 10)
                const limit = raw === '' || Number.isNaN(parsed) ? null : parsed
                setDraft((previous) => ({ ...previous, limit }))
              }}
            />
            <span class="description text-xs text-muted">
              Сколько треков оставить после сортировки
            </span>
          </label>
        </div>

        <section>
          <SectionHeader
            title="Предпросмотр"
            description={previewSummary()}
            action={
              <Show when={previewTracks().length > 0}>
                <IconButton
                  label="Слушать предпросмотр"
                  onClick={() => {
                    playPreview(0)
                  }}
                >
                  <Play aria-hidden="true" />
                </IconButton>
              </Show>
            }
          />

          <Show
            when={rulesValid()}
            fallback={
              <p class="px-4 py-3 text-sm text-muted">
                {draft().rules.length === 0
                  ? 'Добавьте хотя бы одно условие — предпросмотр появится сам.'
                  : 'Исправьте условия — предпросмотр обновится сам.'}
              </p>
            }
          >
            <Show
              when={previewError() === null}
              fallback={
                <p class="px-4 py-3 text-sm text-danger">{previewError()}</p>
              }
            >
            <Show
              when={!preview.loading && preview() !== undefined}
              fallback={
                <div class="flex justify-center py-8">
                  <Spinner />
                </div>
              }
            >
              <Show
                when={previewTracks().length > 0}
                fallback={
                  <EmptyState
                    icon={<Sparkles aria-hidden="true" />}
                    title="Под условия ничего не подошло"
                    description="Ослабьте правила или уберите лишнее условие."
                  />
                }
              >
                <For each={previewTracks().slice(0, PREVIEW_ROWS)}>
                  {(track, index) => (
                    <TrackRow
                      track={track}
                      onPlay={() => {
                        playPreview(index())
                      }}
                    />
                  )}
                </For>

                <Show when={previewTracks().length > PREVIEW_ROWS}>
                  <p class="px-4 py-3 text-xs text-muted">
                    Показаны первые {String(PREVIEW_ROWS)} из{' '}
                    {formatTrackCount(previewTracks().length)}
                  </p>
                </Show>
              </Show>
            </Show>
            </Show>
          </Show>
        </section>
      </Show>

      <Sheet
        open={addOpen()}
        onOpenChange={setAddOpen}
        title="Добавить условие"
        description="Условия комбинируются по правилу «все» или «любое»"
      >
        <div class="-mx-2 flex max-h-[60vh] flex-col overflow-y-auto scrollbar-none">
          <For each={RULE_KIND_ORDER}>
            {(kind) => (
              <button
                type="button"
                class="menu-item min-h-14 items-start"
                onClick={() => {
                  addRule(kind)
                }}
              >
                <span class="flex min-w-0 flex-1 flex-col gap-0.5 text-start">
                  <span class="truncate text-sm text-foreground">
                    {RULE_DESCRIPTORS[kind].label}
                  </span>
                  <span class="truncate text-xs text-muted">{RULE_DESCRIPTORS[kind].hint}</span>
                </span>
              </button>
            )}
          </For>
        </div>
      </Sheet>
    </Screen>
  )
}
