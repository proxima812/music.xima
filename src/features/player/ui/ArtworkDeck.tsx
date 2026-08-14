import type { Track } from '@/shared/ipc'
import { CoverArt } from '@/shared/ui/CoverArt'

export type ArtworkDeckProps = {
  previous: Track | null
  current: Track
  next: Track | null
  dragX: number
  settling: boolean
  reducedMotion: boolean
}

type DeckPosition = 'previous' | 'current' | 'next'

type DeckCardProps = {
  position: DeckPosition
  track: Track
  dragX: number
  settling: boolean
  reducedMotion: boolean
}

const RESTING_OFFSET: Record<DeckPosition, string> = {
  previous: '-90%',
  current: '0px',
  next: '90%',
}

function DeckCard(props: DeckCardProps) {
  const isCurrent = (): boolean => props.position === 'current'
  const ariaHidden = (): true | undefined => (isCurrent() ? undefined : true)
  const transitionDuration = (): string =>
    props.settling && !props.reducedMotion ? 'duration-[220ms]' : 'duration-0'
  const transform = (): string =>
    `translateX(calc(${RESTING_OFFSET[props.position]} + ${String(props.dragX)}px))`

  return (
    <div
      class={`absolute inset-0 transition-[transform] ease-out-fluid ${transitionDuration()} ${
        isCurrent() ? 'z-20' : 'z-10'
      }`}
      style={{ transform: transform() }}
      aria-hidden={ariaHidden()}
    >
      <div class={`size-full ${isCurrent() ? '' : 'scale-95'}`}>
        <CoverArt
          coverKey={props.track.coverKey}
          seed={`${props.track.albumTitle ?? props.track.title}·${props.track.artistName ?? ''}`}
          size="full"
          rounded="lg"
          alt={isCurrent() ? `Обложка: ${props.track.title}` : ''}
        />
      </div>
    </div>
  )
}

/**
 * Презентационный стек обложек. Жест и смена очереди принадлежат родителю;
 * этот компонент получает уже вычисленные соседние треки и смещение.
 */
export function ArtworkDeck(props: ArtworkDeckProps) {
  return (
    <div class="relative w-full overflow-hidden [container-type:inline-size]">
      <div class="relative mx-auto aspect-square w-[min(78cqw,58dvh,34rem)]">
        {props.previous !== null && (
          <DeckCard
            position="previous"
            track={props.previous}
            dragX={props.dragX}
            settling={props.settling}
            reducedMotion={props.reducedMotion}
          />
        )}
        <DeckCard
          position="current"
          track={props.current}
          dragX={props.dragX}
          settling={props.settling}
          reducedMotion={props.reducedMotion}
        />
        {props.next !== null && (
          <DeckCard
            position="next"
            track={props.next}
            dragX={props.dragX}
            settling={props.settling}
            reducedMotion={props.reducedMotion}
          />
        )}
      </div>
    </div>
  )
}
