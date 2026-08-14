import { Show, type JSX } from 'solid-js'

import { cn } from '@/shared/lib'

export type EmptyStateProps = {
  icon?: JSX.Element
  title: string
  description?: string
  action?: JSX.Element
  class?: string
}

/** Пустое состояние списка/экрана (`.empty-state`). */
export function EmptyState(props: EmptyStateProps) {
  return (
    <div
      class={cn(
        'empty-state flex flex-col items-center justify-center gap-4 px-6 py-12 text-center',
        props.class,
      )}
    >
      <Show when={props.icon}>
        {(icon) => (
          <div class="flex size-14 items-center justify-center rounded-full bg-surface-secondary text-muted">
            {icon()}
          </div>
        )}
      </Show>

      <div class="flex flex-col gap-1">
        <p class="text-base font-medium text-foreground">{props.title}</p>
        <Show when={props.description}>
          {(description) => <p class="text-sm text-muted">{description()}</p>}
        </Show>
      </div>

      <Show when={props.action}>{(action) => <div>{action()}</div>}</Show>
    </div>
  )
}
