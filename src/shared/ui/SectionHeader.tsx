import { Show, type JSX } from 'solid-js'

import { cn } from '@/shared/lib'

export type SectionHeaderProps = {
  title: JSX.Element
  description?: JSX.Element
  action?: JSX.Element
  class?: string
}

/** Заголовок секции внутри экрана с опциональным действием справа. */
export function SectionHeader(props: SectionHeaderProps) {
  return (
    <div class={cn('flex items-end justify-between gap-3 px-4 pt-5 pb-2', props.class)}>
      <div class="flex min-w-0 flex-col gap-0.5">
        <h2 class="truncate text-base font-semibold text-foreground">{props.title}</h2>
        <Show when={props.description}>
          {(description) => <p class="truncate text-xs text-muted">{description()}</p>}
        </Show>
      </div>
      <Show when={props.action}>
        {(action) => <div class="flex shrink-0 items-center">{action()}</div>}
      </Show>
    </div>
  )
}
