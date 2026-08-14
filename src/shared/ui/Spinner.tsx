import { cn } from '@/shared/lib'

export type SpinnerSize = 'sm' | 'md' | 'lg' | 'xl'

export type SpinnerColor = 'current' | 'accent' | 'success' | 'warning' | 'danger'

export type SpinnerProps = {
  size?: SpinnerSize
  color?: SpinnerColor
  class?: string
  label?: string
}

/** Индикатор загрузки на классах HeroUI (`.spinner`). */
export function Spinner(props: SpinnerProps) {
  return (
    <span
      class={cn(
        'spinner',
        `spinner--${props.size ?? 'md'}`,
        `spinner--${props.color ?? 'current'}`,
        props.class,
      )}
      data-slot="spinner"
      role="status"
      aria-label={props.label ?? 'Загрузка'}
    >
      <svg
        class="h-full w-full"
        data-slot="spinner-icon"
        viewBox="0 0 24 24"
        fill="none"
        aria-hidden="true"
      >
        <circle
          cx="12"
          cy="12"
          r="9"
          stroke="currentColor"
          stroke-opacity="0.25"
          stroke-width="3"
        />
        <path
          d="M21 12a9 9 0 0 0-9-9"
          stroke="currentColor"
          stroke-width="3"
          stroke-linecap="round"
        />
      </svg>
    </span>
  )
}
