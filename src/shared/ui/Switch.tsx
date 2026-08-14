import { Switch as KobalteSwitch } from '@kobalte/core/switch'
import { Show, type JSX } from 'solid-js'

import { cn } from '@/shared/lib'

export type SwitchSize = 'sm' | 'md' | 'lg'

export type SwitchProps = {
  checked: boolean
  onChange: (checked: boolean) => void
  label?: JSX.Element
  description?: JSX.Element
  size?: SwitchSize
  disabled?: boolean
  ariaLabel?: string
  class?: string
}

/**
 * Переключатель на Kobalte со стилями HeroUI (`.switch__*`).
 * `data-selected`/`data-disabled` проставляем сами: HeroUI ждёт значение
 * `"true"`, а Kobalte пишет пустую строку.
 */
export function Switch(props: SwitchProps) {
  return (
    <KobalteSwitch
      class={cn('switch', `switch--${props.size ?? 'md'}`, props.class)}
      checked={props.checked}
      onChange={props.onChange}
      disabled={props.disabled ?? false}
      data-selected={props.checked ? 'true' : undefined}
      data-disabled={props.disabled === true ? 'true' : undefined}
    >
      <div class="switch__content" data-slot="switch-content">
        <KobalteSwitch.Input aria-label={props.ariaLabel} />
        <KobalteSwitch.Control class="switch__control">
          <KobalteSwitch.Thumb class="switch__thumb" />
        </KobalteSwitch.Control>
        <Show when={props.label}>
          {(label) => (
            <KobalteSwitch.Label data-slot="label" class="switch__label">
              {label()}
            </KobalteSwitch.Label>
          )}
        </Show>
      </div>

      <Show when={props.description}>
        {(description) => (
          <span data-slot="description" class="text-xs text-muted">
            {description()}
          </span>
        )}
      </Show>
    </KobalteSwitch>
  )
}
