import { createResource, For, Index, Show, Suspense, type JSX } from 'solid-js'

import { cn, glowGradient } from '@/shared/lib'
import { CoverArt } from '@/shared/ui'
import { MOSAIC_COVERS, playlistCovers } from '../model/playlists-store'

export type PlaylistCoverProps = {
  playlistId: number
  /** Обложка самого плейлиста: показывается, пока треки не прочитаны. */
  coverKey: string | null
  trackCount: number
  /** Сторона квадрата в пикселях; игнорируется при `fill`. */
  size?: number
  /** Растянуть на всю ширину родителя (шапка экрана плейлиста). */
  fill?: boolean
  name?: string
  class?: string
}

const DEFAULT_SIZE = 56

/** Заглушка вместо обложки: у каждого плейлиста свой градиент, семя — имя. */
function Glow(props: { seed: string }) {
  return (
    <div
      class="h-full w-full"
      style={{ 'background-image': glowGradient(props.seed) }}
      aria-hidden="true"
    />
  )
}

/** Обложка плейлиста: мозаика 2x2 из обложек первых треков либо одна картинка. */
export function PlaylistCover(props: PlaylistCoverProps) {
  const [keys] = createResource(
    () => (props.trackCount > 0 ? props.playlistId : undefined),
    (id: number) => playlistCovers(id),
  )

  const found = (): readonly string[] => keys() ?? []

  const mosaic = (): readonly string[] | null =>
    found().length >= 2 ? found().slice(0, MOSAIC_COVERS) : null

  const single = (): string | null => found()[0] ?? props.coverKey

  const style = (): JSX.CSSProperties | undefined =>
    props.fill === true
      ? undefined
      : {
          width: `${String(props.size ?? DEFAULT_SIZE)}px`,
          height: `${String(props.size ?? DEFAULT_SIZE)}px`,
        }

  return (
    <div
      class={cn(
        'shrink-0 overflow-hidden rounded-xl bg-surface-secondary',
        props.fill === true && 'aspect-square w-full',
        props.class,
      )}
      style={style()}
    >
      {/*
        Своя граница ожидания, а не общая. Ключи обложек читаются ресурсом, и
        без локального `<Suspense>` он поднимал бы единственный общий бандер из
        `App.tsx` — а тот на время загрузки вынимает из DOM весь экран вместе с
        прокруткой списка (docs/BUGS.md, B8).
      */}
      <Suspense fallback={<Glow seed={props.name ?? 'playlist'} />}>
        <Show
          when={mosaic()}
          fallback={
            <Show when={single()} fallback={<Glow seed={props.name ?? 'playlist'} />}>
              {(coverKey) => (
                <CoverArt
                  coverKey={coverKey()}
                  size="full"
                  rounded="none"
                  seed={props.name ?? ''}
                  alt={props.name ?? ''}
                  class="h-full w-full"
                />
              )}
            </Show>
          }
        >
          {(tiles) => (
            <div class="grid h-full w-full grid-cols-2 grid-rows-2">
              <For each={tiles()}>
                {(coverKey) => (
                  <CoverArt
                    coverKey={coverKey}
                    size="full"
                    rounded="none"
                    alt=""
                    class="h-full w-full"
                  />
                )}
              </For>
              <Index each={new Array<null>(Math.max(0, MOSAIC_COVERS - tiles().length)).fill(null)}>
                {(_empty, index) => (
                  <Glow seed={`${props.name ?? 'playlist'}·${String(index)}`} />
                )}
              </Index>
            </div>
          )}
        </Show>
      </Suspense>
    </div>
  )
}
