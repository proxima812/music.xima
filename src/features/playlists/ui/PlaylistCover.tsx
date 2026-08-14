import { createResource, For, Index, Show, type JSX } from 'solid-js'

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
      <Show
        when={mosaic()}
        fallback={
          <Show
            when={single()}
            fallback={
              <div
                class="h-full w-full"
                style={{ 'background-image': glowGradient(props.name ?? 'playlist') }}
                aria-hidden="true"
              />
            }
          >
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
                <div
                  class="h-full w-full"
                  style={{
                    'background-image': glowGradient(`${props.name ?? 'playlist'}·${String(index)}`),
                  }}
                  aria-hidden="true"
                />
              )}
            </Index>
          </div>
        )}
      </Show>
    </div>
  )
}
