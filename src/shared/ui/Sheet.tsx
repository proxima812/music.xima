import { Dialog } from '@kobalte/core/dialog'
import { Show, type JSX, type ParentProps } from 'solid-js'

import { cn } from '@/shared/lib'

export type SheetProps = ParentProps<{
  open: boolean
  onOpenChange: (open: boolean) => void
  title?: JSX.Element
  description?: JSX.Element
  footer?: JSX.Element
  /** Ручка сверху; выключается для «плотных» шитов. */
  handle?: boolean
  class?: string
}>

/** Боттом-шит на Kobalte Dialog со стилями HeroUI (`.drawer__*`). */
export function Sheet(props: SheetProps) {
  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange} modal preventScroll>
      <Dialog.Portal>
        <Dialog.Overlay class="drawer__backdrop drawer__backdrop--blur h-dvh animate-in fade-in-0 duration-150" />
        <div class="drawer__content drawer__content--bottom h-dvh">
          <Dialog.Content
            data-placement="bottom"
            class={cn(
              'drawer__dialog animate-in slide-in-from-bottom duration-250 ease-out-fluid safe-bottom',
              props.class,
            )}
          >
            <Show when={props.handle !== false}>
              <div class="drawer__handle">
                <div data-slot="drawer-handle-bar" />
              </div>
            </Show>

            <Show when={props.title !== undefined || props.description !== undefined}>
              <div class="drawer__header">
                <Show when={props.title}>
                  {(title) => <Dialog.Title class="drawer__heading">{title()}</Dialog.Title>}
                </Show>
                <Show when={props.description}>
                  {(description) => (
                    <Dialog.Description class="text-sm text-muted">
                      {description()}
                    </Dialog.Description>
                  )}
                </Show>
              </div>
            </Show>

            <div class="drawer__body">{props.children}</div>

            <Show when={props.footer}>{(footer) => <div class="drawer__footer">{footer()}</div>}</Show>
          </Dialog.Content>
        </div>
      </Dialog.Portal>
    </Dialog>
  )
}
