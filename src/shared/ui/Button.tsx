import { Show, splitProps, type JSX, type ParentProps } from 'solid-js'

import { cn } from '@/shared/lib'
import { Spinner } from './Spinner'

export type ButtonVariant =
  | 'primary'
  | 'secondary'
  | 'tertiary'
  | 'ghost'
  | 'outline'
  | 'danger'
  | 'danger-soft'

export type ButtonSize = 'sm' | 'md' | 'lg'

export type ButtonProps = ParentProps<
  JSX.ButtonHTMLAttributes<HTMLButtonElement> & {
    variant?: ButtonVariant
    size?: ButtonSize
    fullWidth?: boolean
    iconOnly?: boolean
    /** Показывает спиннер и блокирует нажатие. */
    pending?: boolean
  }
>

/** Кнопка на классах HeroUI (`.button`). Остальные пропсы уходят на `<button>`. */
export function Button(props: ButtonProps) {
  const [local, rest] = splitProps(props, [
    'variant',
    'size',
    'fullWidth',
    'iconOnly',
    'pending',
    'class',
    'children',
    'disabled',
    'type',
  ])

  const isPending = (): boolean => local.pending === true

  return (
    <button
      {...rest}
      type={local.type ?? 'button'}
      disabled={isPending() || local.disabled === true}
      aria-busy={isPending() ? 'true' : undefined}
      data-pending={isPending() ? 'true' : undefined}
      class={cn(
        'button',
        `button--${local.variant ?? 'secondary'}`,
        `button--${local.size ?? 'md'}`,
        local.fullWidth === true && 'button--full-width',
        local.iconOnly === true && 'button--icon-only',
        local.class,
      )}
    >
      <Show when={isPending()}>
        <Spinner size="sm" color="current" />
      </Show>
      {local.children}
    </button>
  )
}
