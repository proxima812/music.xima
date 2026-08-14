import { AlertDialog } from '@kobalte/core/alert-dialog'
import { createSignal, Show } from 'solid-js'

import { cn } from '@/shared/lib'
import { Button } from './Button'

export type ConfirmDialogProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description?: string
  confirmLabel?: string
  cancelLabel?: string
  /** Красная кнопка подтверждения — для удаления. */
  danger?: boolean
  /** Может вернуть промис: диалог сам покажет спиннер и закроется после успеха. */
  onConfirm: () => unknown
  class?: string
}

/** Диалог подтверждения на Kobalte AlertDialog со стилями HeroUI (`.modal__*`). */
export function ConfirmDialog(props: ConfirmDialogProps) {
  const [pending, setPending] = createSignal(false)

  const close = (): void => {
    props.onOpenChange(false)
  }

  const confirm = (): void => {
    const result: unknown = props.onConfirm()

    if (!(result instanceof Promise)) {
      close()
      return
    }

    const task: Promise<unknown> = result
    setPending(true)
    void task.then(
      () => {
        setPending(false)
        close()
      },
      (error: unknown) => {
        setPending(false)
        console.error('[confirm-dialog] действие не выполнено', error)
      },
    )
  }

  return (
    <AlertDialog open={props.open} onOpenChange={props.onOpenChange} modal preventScroll>
      <AlertDialog.Portal>
        <AlertDialog.Overlay class="modal__backdrop modal__backdrop--blur h-dvh animate-in fade-in-0 duration-150" />
        <div class="fixed inset-0 z-50 modal__container h-dvh" data-placement="center">
          <AlertDialog.Content
            data-placement="center"
            class={cn(
              'modal__dialog modal__dialog--sm animate-in fade-in-0 zoom-in-95 duration-150',
              props.class,
            )}
          >
            <div class="modal__header">
              <AlertDialog.Title class="modal__heading text-base font-semibold">
                {props.title}
              </AlertDialog.Title>
            </div>

            <Show when={props.description}>
              {(description) => (
                <AlertDialog.Description class="modal__body text-sm text-muted">
                  {description()}
                </AlertDialog.Description>
              )}
            </Show>

            <div class="modal__footer">
              <Button variant="ghost" onClick={close} disabled={pending()}>
                {props.cancelLabel ?? 'Отмена'}
              </Button>
              <Button
                variant={props.danger === true ? 'danger' : 'primary'}
                pending={pending()}
                onClick={confirm}
              >
                {props.confirmLabel ?? 'Подтвердить'}
              </Button>
            </div>
          </AlertDialog.Content>
        </div>
      </AlertDialog.Portal>
    </AlertDialog>
  )
}
