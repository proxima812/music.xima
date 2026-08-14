import { Toast, toaster } from '@kobalte/core/toast'
import { X } from 'lucide-solid'
import { Show } from 'solid-js'
import { Portal } from 'solid-js/web'

import { cn } from '@/shared/lib'

export type ToastVariant = 'default' | 'accent' | 'success' | 'warning' | 'danger'

export type ToastOptions = {
  title: string
  description?: string
  variant?: ToastVariant
}

const VARIANT_CLASS: Record<ToastVariant, string | undefined> = {
  default: undefined,
  accent: 'toast--accent',
  success: 'toast--success',
  warning: 'toast--warning',
  danger: 'toast--danger',
}

/** Показывает тост. Возвращает id — им же можно закрыть через `dismissToast`. */
export function toast(options: ToastOptions): number {
  return toaster.show((props) => (
    <Toast
      toastId={props.toastId}
      data-frontmost="true"
      class={cn('toast relative w-full', VARIANT_CLASS[options.variant ?? 'default'])}
    >
      <div class="toast__content">
        <Toast.Title class="toast__title">{options.title}</Toast.Title>
        <Show when={options.description}>
          {(description) => (
            <Toast.Description class="toast__description">{description()}</Toast.Description>
          )}
        </Show>
      </div>

      <Toast.CloseButton
        class="button button--ghost button--icon-only button--sm -me-2 shrink-0"
        aria-label="Закрыть уведомление"
      >
        <X aria-hidden="true" />
      </Toast.CloseButton>
    </Toast>
  ))
}

export function dismissToast(id: number): void {
  toaster.dismiss(id)
}

export function clearToasts(): void {
  toaster.clear()
}

/**
 * Регион тостов (`.toast-region`). Монтируется один раз в каркасе приложения,
 * поднят над мини-плеером и нижней навигацией.
 */
export function Toasts() {
  return (
    <Portal>
      <Toast.Region
        duration={4000}
        limit={3}
        swipeDirection="down"
        class="toast-region toast-region--bottom bottom-[calc(var(--spacing-player-stack)_+_var(--safe-bottom))]"
      >
        <Toast.List class="flex flex-col gap-2" />
      </Toast.Region>
    </Portal>
  )
}
