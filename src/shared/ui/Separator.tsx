import { cn } from '@/shared/lib'

export type SeparatorOrientation = 'horizontal' | 'vertical'

export type SeparatorColor = 'default' | 'secondary' | 'tertiary'

export type SeparatorProps = {
  orientation?: SeparatorOrientation
  color?: SeparatorColor
  class?: string
}

/** Разделитель (`.separator`). */
export function Separator(props: SeparatorProps) {
  const orientation = (): SeparatorOrientation => props.orientation ?? 'horizontal'

  return (
    <div
      role="separator"
      aria-orientation={orientation()}
      data-slot="separator"
      class={cn(
        'separator',
        `separator--${orientation()}`,
        `separator--${props.color ?? 'default'}`,
        props.class,
      )}
    />
  )
}
