import { DropdownMenu } from '@kobalte/core/dropdown-menu'
import { EllipsisVertical } from 'lucide-solid'
import { For, Show, type JSX } from 'solid-js'

import { cn } from '@/shared/lib'

export type MenuPlacement = 'bottom-start' | 'bottom-end' | 'top-start' | 'top-end'

export type MenuAction = {
  label: string
  onSelect: () => void
  icon?: JSX.Element
  /** Красный пункт: удаление и прочее необратимое. */
  danger?: boolean
  disabled?: boolean
}

export type MenuProps = {
  items: readonly MenuAction[]
  /** Содержимое кнопки-триггера; по умолчанию — три точки. */
  trigger?: JSX.Element
  label?: string
  placement?: MenuPlacement
  triggerClass?: string
  class?: string
}

/** Контекстное меню на Kobalte DropdownMenu со стилями HeroUI (`.menu`). */
export function Menu(props: MenuProps) {
  return (
    <DropdownMenu placement={props.placement ?? 'bottom-end'} gutter={6} modal={false}>
      <DropdownMenu.Trigger
        aria-label={props.label ?? 'Меню'}
        class={cn(
          'button button--ghost button--icon-only min-h-11 min-w-11',
          props.triggerClass,
        )}
      >
        <Show when={props.trigger} fallback={<EllipsisVertical aria-hidden="true" />}>
          {(trigger) => trigger()}
        </Show>
      </DropdownMenu.Trigger>

      <DropdownMenu.Portal>
        <DropdownMenu.Content class={cn('dropdown__popover menu z-50 p-1.5', props.class)}>
          <For each={props.items}>
            {(item) => (
              <DropdownMenu.Item
                class={cn('menu-item', item.danger === true && 'menu-item--danger')}
                disabled={item.disabled ?? false}
                onSelect={item.onSelect}
              >
                <Show when={item.icon}>
                  {(icon) => (
                    <span class="flex size-5 shrink-0 items-center justify-center text-muted">
                      {icon()}
                    </span>
                  )}
                </Show>
                <span data-slot="label" class="flex-1 truncate text-start text-sm">
                  {item.label}
                </span>
              </DropdownMenu.Item>
            )}
          </For>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu>
  )
}
