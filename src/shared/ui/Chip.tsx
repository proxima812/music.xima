import type { ParentProps } from 'solid-js'

import { cn } from '@/shared/lib'

export type ChipColor = 'default' | 'accent' | 'success' | 'warning' | 'danger'

export type ChipVariant = 'primary' | 'secondary' | 'tertiary'

export type ChipSize = 'sm' | 'md' | 'lg'

export type ChipProps = ParentProps<{
  color?: ChipColor
  variant?: ChipVariant
  size?: ChipSize
  class?: string
}>

/** Маленькая метка (`.chip`): счётчики, битрейт, год. */
export function Chip(props: ChipProps) {
  return (
    <span
      class={cn(
        'chip',
        `chip--${props.variant ?? 'secondary'}`,
        `chip--${props.color ?? 'default'}`,
        `chip--${props.size ?? 'md'}`,
        props.class,
      )}
    >
      <span class="chip__label">{props.children}</span>
    </span>
  )
}
