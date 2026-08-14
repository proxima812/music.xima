import { Check, Trash2 } from 'lucide-solid'
import {
  createEffect,
  createResource,
  createSignal,
  For,
  Match,
  onCleanup,
  Show,
  Switch as SwitchFlow,
} from 'solid-js'

import {
  libraryArtist,
  libraryArtists,
  libraryGenres,
  NUM_OPS,
  oneOf,
  searchAll,
  SMART_RULE_KINDS,
  type Artist,
  type Genre,
  type NumOp,
  type SmartRule,
} from '@/shared/ipc'
import { debounce } from '@/shared/lib'
import { Button, IconButton, SearchInput, Sheet, Spinner, Switch } from '@/shared/ui'
import {
  BITRATE_PRESETS_KBPS,
  bitrateToKbps,
  createRule,
  kbpsToBitrate,
  NUM_OP_LABELS,
  RULE_DESCRIPTORS,
  RULE_KIND_ORDER,
  ruleFieldLabel,
  validateRule,
} from '../model/rules'

const ARTIST_PAGE_LIMIT = 50

const ARTIST_SEARCH_LIMIT = 20

const ARTIST_SEARCH_DEBOUNCE_MS = 300

const KIND_OPTIONS: readonly SelectOption[] = RULE_KIND_ORDER.map((kind) => ({
  value: kind,
  label: RULE_DESCRIPTORS[kind].label,
}))

const OP_OPTIONS: readonly SelectOption[] = NUM_OPS.map((op) => ({
  value: op,
  label: NUM_OP_LABELS[op],
}))

/** Жанры одни на все правила экрана — тянем их один раз за сеанс. */
let genresRequest: Promise<Genre[]> | null = null

function loadGenres(): Promise<Genre[]> {
  genresRequest ??= libraryGenres().catch((error: unknown) => {
    // Сбрасываем кэш, иначе одна ошибка запомнится до конца сеанса.
    console.error('[smart] не удалось прочитать жанры', error)
    genresRequest = null
    return []
  })
  return genresRequest
}

export type RuleEditorProps = {
  rule: SmartRule
  onChange: (rule: SmartRule) => void
  onRemove: () => void
}

/** Редактор одного правила: тип сверху, поля — под выбранный тип. */
export function RuleEditor(props: RuleEditorProps) {
  // Явные аксессоры вместо приведения типов: сужение делает сам TypeScript.
  const playCount = () => (props.rule.kind === 'PLAY_COUNT' ? props.rule : undefined)
  const lastPlayed = () =>
    props.rule.kind === 'LAST_PLAYED_BEFORE_DAYS' ? props.rule : undefined
  const addedWithin = () => (props.rule.kind === 'ADDED_WITHIN_DAYS' ? props.rule : undefined)
  const favorite = () => (props.rule.kind === 'FAVORITE' ? props.rule : undefined)
  const yearBetween = () => (props.rule.kind === 'YEAR_BETWEEN' ? props.rule : undefined)
  const bitrate = () => (props.rule.kind === 'BITRATE_AT_LEAST' ? props.rule : undefined)
  const duration = () => (props.rule.kind === 'DURATION_BETWEEN_MS' ? props.rule : undefined)
  const genre = () => (props.rule.kind === 'GENRE_IS' ? props.rule : undefined)
  const artist = () => (props.rule.kind === 'ARTIST_IS' ? props.rule : undefined)
  const title = () => (props.rule.kind === 'TITLE_CONTAINS' ? props.rule : undefined)

  const error = (): string | null => validateRule(props.rule)

  return (
    <div class="depth-raised flex flex-col gap-3 rounded-2xl p-3">
      <div class="flex items-start gap-2">
        <div class="min-w-0 flex-1">
          <SelectField
            label="Условие"
            value={props.rule.kind}
            options={KIND_OPTIONS}
            onChange={(value) => {
              const kind = oneOf(value, SMART_RULE_KINDS)
              if (kind !== undefined && kind !== props.rule.kind) props.onChange(createRule(kind))
            }}
          />
        </div>
        <IconButton
          label="Удалить условие"
          class="mt-6"
          onClick={() => {
            props.onRemove()
          }}
        >
          <Trash2 aria-hidden="true" />
        </IconButton>
      </div>

      <p class="text-xs text-muted">{RULE_DESCRIPTORS[props.rule.kind].hint}</p>

      <SwitchFlow>
        <Match when={playCount()}>
          {(rule) => (
            <div class="flex items-end gap-2">
              <div class="min-w-0 flex-1">
                <SelectField
                  label={ruleFieldLabel('PLAY_COUNT', 'op')}
                  value={rule().op}
                  options={OP_OPTIONS}
                  onChange={(value) => {
                    const op: NumOp | undefined = oneOf(value, NUM_OPS)
                    if (op !== undefined) {
                      props.onChange({ kind: 'PLAY_COUNT', op, value: rule().value })
                    }
                  }}
                />
              </div>
              <div class="w-28 shrink-0">
                <NumberField
                  label={ruleFieldLabel('PLAY_COUNT', 'value')}
                  value={rule().value}
                  min={0}
                  max={100_000}
                  suffix=""
                  onChange={(value) => {
                    props.onChange({ kind: 'PLAY_COUNT', op: rule().op, value })
                  }}
                />
              </div>
            </div>
          )}
        </Match>

        <Match when={lastPlayed()}>
          {(rule) => (
            <NumberField
              label={ruleFieldLabel('LAST_PLAYED_BEFORE_DAYS', 'days')}
              value={rule().days}
              min={1}
              max={3650}
              suffix="дн."
              onChange={(days) => {
                props.onChange({ kind: 'LAST_PLAYED_BEFORE_DAYS', days })
              }}
            />
          )}
        </Match>

        <Match when={addedWithin()}>
          {(rule) => (
            <NumberField
              label={ruleFieldLabel('ADDED_WITHIN_DAYS', 'days')}
              value={rule().days}
              min={1}
              max={3650}
              suffix="дн."
              onChange={(days) => {
                props.onChange({ kind: 'ADDED_WITHIN_DAYS', days })
              }}
            />
          )}
        </Match>

        <Match when={favorite()}>
          {(rule) => (
            <Switch
              checked={rule().value}
              label={rule().value ? 'Только избранные' : 'Только не избранные'}
              onChange={(value) => {
                props.onChange({ kind: 'FAVORITE', value })
              }}
            />
          )}
        </Match>

        <Match when={yearBetween()}>
          {(rule) => (
            <div class="flex items-end gap-2">
              <div class="min-w-0 flex-1">
                <NumberField
                  label={ruleFieldLabel('YEAR_BETWEEN', 'from')}
                  value={rule().from}
                  min={1}
                  max={9999}
                  suffix=""
                  onChange={(from) => {
                    props.onChange({ kind: 'YEAR_BETWEEN', from, to: rule().to })
                  }}
                />
              </div>
              <div class="min-w-0 flex-1">
                <NumberField
                  label={ruleFieldLabel('YEAR_BETWEEN', 'to')}
                  value={rule().to}
                  min={1}
                  max={9999}
                  suffix=""
                  onChange={(to) => {
                    props.onChange({ kind: 'YEAR_BETWEEN', from: rule().from, to })
                  }}
                />
              </div>
            </div>
          )}
        </Match>

        <Match when={bitrate()}>
          {(rule) => (
            <div class="flex flex-col gap-2">
              <span class="label">{ruleFieldLabel('BITRATE_AT_LEAST', 'value')}</span>
              <div class="flex flex-wrap gap-2">
                <For each={BITRATE_PRESETS_KBPS}>
                  {(kbps) => (
                    <Button
                      variant={bitrateToKbps(rule().value) === kbps ? 'primary' : 'secondary'}
                      size="md"
                      class="min-h-11"
                      onClick={() => {
                        props.onChange({ kind: 'BITRATE_AT_LEAST', value: kbpsToBitrate(kbps) })
                      }}
                    >
                      {String(kbps)} кбит/с
                    </Button>
                  )}
                </For>
              </div>
            </div>
          )}
        </Match>

        <Match when={duration()}>
          {(rule) => (
            <div class="flex items-end gap-2">
              <div class="min-w-0 flex-1">
                <NumberField
                  label={ruleFieldLabel('DURATION_BETWEEN_MS', 'from')}
                  value={Math.round(rule().from / 1000)}
                  min={0}
                  max={86_400}
                  suffix=""
                  onChange={(seconds) => {
                    props.onChange({
                      kind: 'DURATION_BETWEEN_MS',
                      from: seconds * 1000,
                      to: rule().to,
                    })
                  }}
                />
              </div>
              <div class="min-w-0 flex-1">
                <NumberField
                  label={ruleFieldLabel('DURATION_BETWEEN_MS', 'to')}
                  value={Math.round(rule().to / 1000)}
                  min={0}
                  max={86_400}
                  suffix=""
                  onChange={(seconds) => {
                    props.onChange({
                      kind: 'DURATION_BETWEEN_MS',
                      from: rule().from,
                      to: seconds * 1000,
                    })
                  }}
                />
              </div>
            </div>
          )}
        </Match>

        <Match when={genre()}>
          {(rule) => (
            <GenreField
              value={rule().value}
              onChange={(value) => {
                props.onChange({ kind: 'GENRE_IS', value })
              }}
            />
          )}
        </Match>

        <Match when={artist()}>
          {(rule) => (
            <ArtistField
              artistId={rule().artistId}
              onChange={(artistId) => {
                props.onChange({ kind: 'ARTIST_IS', artistId })
              }}
            />
          )}
        </Match>

        <Match when={title()}>
          {(rule) => (
            <label class="flex flex-col gap-1">
              <span class="label">{ruleFieldLabel('TITLE_CONTAINS', 'value')}</span>
              <input
                class="input input--secondary h-11 w-full"
                type="text"
                value={rule().value}
                placeholder="Например: live"
                onInput={(event) => {
                  props.onChange({ kind: 'TITLE_CONTAINS', value: event.currentTarget.value })
                }}
              />
            </label>
          )}
        </Match>
      </SwitchFlow>

      <Show when={error()}>
        {(message) => <p class="error-message text-xs text-danger">{message()}</p>}
      </Show>
    </div>
  )
}

export type SelectOption = {
  value: string
  label: string
}

/**
 * Нативный `<select>` со стилями поля HeroUI: на телефоне системный пикер
 * удобнее и доступнее любого кастомного поповера.
 */
export function SelectField(props: {
  label: string
  value: string
  options: readonly SelectOption[]
  onChange: (value: string) => void
}) {
  return (
    <label class="flex min-w-0 flex-col gap-1">
      <span class="label">{props.label}</span>
      <select
        class="input input--secondary h-11 w-full"
        value={props.value}
        onChange={(event) => {
          props.onChange(event.currentTarget.value)
        }}
      >
        <For each={props.options}>
          {(option) => (
            <option value={option.value} selected={option.value === props.value}>
              {option.label}
            </option>
          )}
        </For>
      </select>
    </label>
  )
}

/**
 * Числовое поле с локальным текстом: пока пользователь стирает и набирает
 * заново, значение правила не дёргается на каждый промежуточный символ.
 */
function NumberField(props: {
  label: string
  value: number
  min: number
  max: number
  suffix: string
  onChange: (value: number) => void
}) {
  const [text, setText] = createSignal(String(props.value))

  createEffect(() => {
    const next = props.value
    if (Number.parseInt(text(), 10) !== next) setText(String(next))
  })

  const commit = (): void => {
    const parsed = Number.parseInt(text(), 10)
    const value = Number.isNaN(parsed)
      ? props.min
      : Math.min(props.max, Math.max(props.min, parsed))
    setText(String(value))
    if (value !== props.value) props.onChange(value)
  }

  return (
    <label class="flex min-w-0 flex-col gap-1">
      <span class="label">{props.label}</span>
      <span class="relative flex items-center">
        <input
          class="input input--secondary h-11 w-full tabular-nums"
          type="number"
          inputmode="numeric"
          min={props.min}
          max={props.max}
          value={text()}
          onInput={(event) => {
            setText(event.currentTarget.value)
            const parsed = Number.parseInt(event.currentTarget.value, 10)
            if (!Number.isNaN(parsed)) props.onChange(parsed)
          }}
          onBlur={commit}
        />
        <Show when={props.suffix !== ''}>
          <span class="pointer-events-none absolute end-3 text-xs text-muted">{props.suffix}</span>
        </Show>
      </span>
    </label>
  )
}

/** Выбор жанра из списка библиотеки. */
function GenreField(props: { value: string; onChange: (value: string) => void }) {
  const [genres] = createResource(() => loadGenres())

  const options = (): SelectOption[] => {
    const list = (genres() ?? []).map((item) => ({ value: item.name, label: item.name }))
    // Значение из правила может не встречаться в текущей библиотеке — не теряем его.
    if (props.value !== '' && !list.some((option) => option.value === props.value)) {
      list.unshift({ value: props.value, label: props.value })
    }
    return [{ value: '', label: 'Выберите жанр' }, ...list]
  }

  return (
    <Show
      when={!genres.loading}
      fallback={
        <div class="flex items-center gap-2 py-2 text-xs text-muted">
          <Spinner size="sm" />
          Загружаем жанры
        </div>
      }
    >
      <SelectField
        label={ruleFieldLabel('GENRE_IS', 'value')}
        value={props.value}
        options={options()}
        onChange={props.onChange}
      />
    </Show>
  )
}

/** Выбор исполнителя: список библиотеки плюс поиск по имени. */
function ArtistField(props: { artistId: number; onChange: (artistId: number) => void }) {
  const [open, setOpen] = createSignal(false)
  const [query, setQuery] = createSignal('')
  const [debouncedQuery, setDebouncedQuery] = createSignal('')

  const pushQuery = debounce((value: string) => {
    setDebouncedQuery(value)
  }, ARTIST_SEARCH_DEBOUNCE_MS)

  onCleanup(() => {
    pushQuery.cancel()
  })

  const [selected] = createResource(
    () => (props.artistId > 0 ? props.artistId : undefined),
    (id: number) => libraryArtist(id),
  )

  const [found] = createResource(
    () => (open() ? debouncedQuery().trim() : undefined),
    async (q: string): Promise<Artist[]> => {
      if (q === '') {
        const page = await libraryArtists(0, ARTIST_PAGE_LIMIT)
        return page.items
      }
      const results = await searchAll(q, ARTIST_SEARCH_LIMIT)
      return results.artists
    },
  )

  return (
    <div class="flex flex-col gap-1">
      <span class="label">{ruleFieldLabel('ARTIST_IS', 'artistId')}</span>
      <Button
        variant="secondary"
        fullWidth
        class="min-h-11 justify-between"
        onClick={() => {
          setOpen(true)
        }}
      >
        <span class="truncate">{selected()?.name ?? 'Выбрать исполнителя'}</span>
      </Button>

      <Sheet
        open={open()}
        onOpenChange={(value) => {
          setOpen(value)
          if (!value) {
            pushQuery.cancel()
            setQuery('')
            setDebouncedQuery('')
          }
        }}
        title="Исполнитель"
      >
        <div class="flex flex-col gap-3">
          <SearchInput
            value={query()}
            placeholder="Имя исполнителя"
            onChange={(value) => {
              setQuery(value)
              pushQuery(value)
            }}
          />

          <Show
            when={!found.loading}
            fallback={
              <div class="flex justify-center py-6">
                <Spinner />
              </div>
            }
          >
            <Show
              when={(found() ?? []).length > 0}
              fallback={<p class="py-6 text-center text-sm text-muted">Никого не нашли</p>}
            >
              <div class="-mx-2 flex max-h-[50vh] flex-col overflow-y-auto scrollbar-none">
                <For each={found() ?? []}>
                  {(item) => (
                    <button
                      type="button"
                      class="menu-item min-h-11"
                      onClick={() => {
                        props.onChange(item.id)
                        setOpen(false)
                      }}
                    >
                      <span data-slot="label" class="flex-1 truncate text-start text-sm">
                        {item.name}
                      </span>
                      <Show when={item.id === props.artistId}>
                        <Check size={16} class="shrink-0 text-accent" aria-hidden="true" />
                      </Show>
                    </button>
                  )}
                </For>
              </div>
            </Show>
          </Show>
        </div>
      </Sheet>
    </div>
  )
}
