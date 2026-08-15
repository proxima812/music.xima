import {
  createEffect,
  createSignal,
  on,
  onCleanup,
  Show,
  type Accessor,
  type JSX,
} from 'solid-js'

import { artworkUri } from '@/shared/ipc'
import { cn, glowGradient } from '@/shared/lib'
import { Skeleton } from './Skeleton'

/**
 * Резолв обложек кэшируется на весь сеанс: один `cover_key` встречается
 * в списках сотни раз, дёргать команду на каждую строку нельзя.
 */
const resolved = new Map<string, string | null>()
const inFlight = new Map<string, Promise<string | null>>()

export function resolveArtwork(coverKey: string): Promise<string | null> {
  const cached = resolved.get(coverKey)
  if (cached !== undefined) return Promise.resolve(cached)

  const pending = inFlight.get(coverKey)
  if (pending !== undefined) return pending

  const request = artworkUri(coverKey)
    .then((uri) => {
      resolved.set(coverKey, uri)
      return uri
    })
    .catch((error: unknown) => {
      console.error(`[cover-art] не удалось получить обложку ${coverKey}`, error)
      resolved.set(coverKey, null)
      return null
    })
    .finally(() => {
      inFlight.delete(coverKey)
    })

  inFlight.set(coverKey, request)
  return request
}

/** Сбрасывает кэш обложек — после пересканирования библиотеки. */
export function clearArtworkCache(): void {
  resolved.clear()
  inFlight.clear()
}

export type Artwork = {
  /** URI обложки либо `null`: её нет, ещё грузится или запрос упал. */
  uri: Accessor<string | null>
  loading: Accessor<boolean>
}

/**
 * Обложка по ключу — на сигналах и **намеренно без `createResource`**.
 *
 * Ресурс, прочитанный в разметке пока он грузится, поднимает ближайший
 * `<Suspense>`, а он в приложении один — общий, в `App.tsx`. Обложка живёт в
 * каждой строке списка, и стоило прокрутке втянуть новую строку, как общий
 * бандер уходил в fallback: Solid вынимал экран из `main` и вставлял обратно, а
 * прокрутка контейнера при этом обнулялась. Снаружи это выглядело как «список
 * не листается пальцем» — палец уводил список на 20–30 px, и тот прыгал в
 * начало (docs/BUGS.md, B8).
 *
 * Уже разрешённый ключ отдаётся синхронно — при прокрутке нет и мигания
 * скелетоном на строках, которые уже показывались.
 */
export function createArtwork(coverKey: Accessor<string | null | undefined>): Artwork {
  const [uri, setUri] = createSignal<string | null>(null)
  const [loading, setLoading] = createSignal(false)

  createEffect(() => {
    const key = coverKey()

    if (key === undefined || key === null) {
      setUri(null)
      setLoading(false)
      return
    }

    const cached = resolved.get(key)
    if (cached !== undefined) {
      setUri(cached)
      setLoading(false)
      return
    }

    // Ключ сменился, пока запрос летел, — ответ уже не наш.
    let stale = false
    onCleanup(() => {
      stale = true
    })

    setUri(null)
    setLoading(true)

    void resolveArtwork(key).then((value) => {
      if (stale) return
      setUri(value)
      setLoading(false)
    })
  })

  return { uri, loading }
}

export type CoverArtSize = 'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl' | 'full'

export type CoverArtRounded = 'none' | 'sm' | 'md' | 'lg' | 'full'

export type CoverArtProps = {
  coverKey: string | null
  /** Готовый размер или число пикселей. */
  size?: CoverArtSize | number
  rounded?: CoverArtRounded
  alt?: string
  /**
   * Семя glow-градиента-заглушки: название трека, альбома, исполнителя.
   * Без него заглушки у разных треков без обложек были бы одинаковыми.
   */
  seed?: string
  class?: string
}

const SIZE_CLASSES: Record<CoverArtSize, string> = {
  xs: 'size-10',
  sm: 'size-12',
  md: 'size-14',
  lg: 'size-16',
  xl: 'size-24',
  '2xl': 'size-40',
  full: 'aspect-square w-full',
}

const ROUNDED_CLASSES: Record<CoverArtRounded, string> = {
  none: 'rounded-none',
  sm: 'rounded-lg',
  md: 'rounded-xl',
  lg: 'rounded-2xl',
  full: 'rounded-full',
}

/** Обложка по `coverKey`: сама резолвит URI, кэширует и показывает заглушку. */
export function CoverArt(props: CoverArtProps) {
  const [broken, setBroken] = createSignal(false)

  const artwork = createArtwork(() => props.coverKey)

  createEffect(
    on(
      () => props.coverKey,
      () => {
        setBroken(false)
      },
      { defer: true },
    ),
  )

  const sizeClass = (): string | undefined =>
    typeof props.size === 'number' ? undefined : SIZE_CLASSES[props.size ?? 'md']

  const sizeStyle = (): JSX.CSSProperties | undefined =>
    typeof props.size === 'number'
      ? { width: `${props.size}px`, height: `${props.size}px` }
      : undefined

  const uri = (): string | undefined => {
    if (broken()) return undefined
    return artwork.uri() ?? undefined
  }

  /** Семя заглушки. Ключ обложки стабильнее названия, поэтому он первый. */
  const seed = (): string => props.seed ?? props.coverKey ?? props.alt ?? 'music.xima'

  return (
    <div
      class={cn(
        'relative flex shrink-0 items-center justify-center overflow-hidden bg-surface-secondary text-muted',
        sizeClass(),
        ROUNDED_CLASSES[props.rounded ?? 'md'],
        props.class,
      )}
      style={sizeStyle()}
    >
      <Show when={artwork.loading()}>
        <Skeleton class="absolute inset-0 h-full w-full rounded-none" />
      </Show>

      <Show
        when={uri()}
        fallback={
          <Show when={!artwork.loading()}>
            <div
              class="absolute inset-0 h-full w-full"
              style={{ 'background-image': glowGradient(seed()) }}
              aria-hidden="true"
            />
          </Show>
        }
      >
        {(src) => (
          <img
            src={src()}
            alt={props.alt ?? ''}
            class="animate-in fade-in-0 h-full w-full object-cover duration-300 ease-out"
            loading="lazy"
            decoding="async"
            draggable={false}
            onError={() => {
              setBroken(true)
            }}
          />
        )}
      </Show>
    </div>
  )
}
