import { Toast, toaster } from '@kobalte/core/toast'
import { X } from 'lucide-solid'
import { createSignal, Show } from 'solid-js'
import { Portal } from 'solid-js/web'

import { cn } from '@/shared/lib'

export type ToastVariant = 'default' | 'accent' | 'success' | 'warning' | 'danger'

export type ToastAction = {
  label: string
  onClick: () => void | Promise<void>
  ariaLabel?: string
}

export type ToastOptions = {
  title: string
  description?: string
  variant?: ToastVariant
  action?: ToastAction
  duration?: number
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
  const [actionPending, setActionPending] = createSignal(false)
  let toastId = 0

  const runAction = async (): Promise<void> => {
    if (options.action === undefined || actionPending()) return

    setActionPending(true)
    try {
      await options.action.onClick()
      dismissToast(toastId)
    } catch (error: unknown) {
      console.error('[toast] действие не выполнено', error)
      const description = error instanceof Error ? error.message : null
      toast(
        description === null
          ? { title: 'Не удалось выполнить действие', variant: 'danger' }
          : { title: 'Не удалось выполнить действие', description, variant: 'danger' },
      )
    } finally {
      setActionPending(false)
    }
  }

  toastId = toaster.show((props) => (
    <Toast
      toastId={props.toastId}
      data-frontmost="true"
      duration={options.duration ?? 4000}
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

      <Show when={options.action}>
        {(action) => (
          <button
            type="button"
            class="button button--ghost button--sm shrink-0"
            aria-label={action().ariaLabel ?? action().label}
            disabled={actionPending()}
            onClick={() => {
              void runAction()
            }}
          >
            {action().label}
          </button>
        )}
      </Show>

      <Toast.CloseButton
        class="button button--ghost button--icon-only button--sm -me-2 shrink-0"
        aria-label="Закрыть уведомление"
      >
        <X aria-hidden="true" />
      </Toast.CloseButton>
    </Toast>
  ))

  return toastId
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
