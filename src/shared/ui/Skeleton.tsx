import { cn } from '@/shared/lib'

export type SkeletonAnimation = 'pulse' | 'shimmer' | 'none'

export type SkeletonProps = {
  animation?: SkeletonAnimation
  class?: string
}

/** Плейсхолдер загрузки (`.skeleton`). Размер задаётся классами снаружи. */
export function Skeleton(props: SkeletonProps) {
  return (
    <div
      class={cn('skeleton', `skeleton--${props.animation ?? 'pulse'}`, props.class)}
      aria-hidden="true"
    />
  )
}
