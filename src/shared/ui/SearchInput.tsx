import { TextField } from '@kobalte/core/text-field'
import { Search, X } from 'lucide-solid'
import { Show } from 'solid-js'

import { cn } from '@/shared/lib'

export type SearchInputProps = {
  value: string
  onChange: (value: string) => void
  placeholder?: string
  /** Вызывается по Enter / кнопке «искать» на клавиатуре. */
  onSubmit?: (value: string) => void
  autofocus?: boolean
  disabled?: boolean
  label?: string
  class?: string
}

/** Поле поиска на Kobalte TextField со стилями HeroUI (`.search-field__*`). */
export function SearchInput(props: SearchInputProps) {
  return (
    <TextField
      class={cn('search-field search-field--full-width', props.class)}
      value={props.value}
      onChange={props.onChange}
      disabled={props.disabled ?? false}
    >
      <div class="search-field__group search-field__group--full-width h-11">
        <Search class="search-field__search-icon" aria-hidden="true" />

        <TextField.Input
          class="search-field__input"
          data-slot="search-field-input"
          type="search"
          inputmode="search"
          enterkeyhint="search"
          autocapitalize="off"
          autocorrect="off"
          spellcheck={false}
          autofocus={props.autofocus ?? false}
          placeholder={props.placeholder ?? 'Поиск'}
          aria-label={props.label ?? 'Поиск'}
          onKeyDown={(event: KeyboardEvent) => {
            if (event.key === 'Enter') props.onSubmit?.(props.value)
          }}
        />

        <Show when={props.value !== ''}>
          <button
            type="button"
            class="button button--ghost button--icon-only button--sm me-1"
            aria-label="Очистить поиск"
            onClick={() => props.onChange('')}
          >
            <X aria-hidden="true" />
          </button>
        </Show>
      </div>
    </TextField>
  )
}
