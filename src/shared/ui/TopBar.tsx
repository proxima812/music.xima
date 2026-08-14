import { Show, type JSX } from 'solid-js'

import { cn } from '@/shared/lib'

export type TopBarProps = {
  title?: JSX.Element
  subtitle?: JSX.Element
  left?: JSX.Element
  right?: JSX.Element
  class?: string
}

/** Шапка экрана: липнет к верху скролла, слоты слева и справа. */
export function TopBar(props: TopBarProps) {
  return (
    <header
      class={cn(
        'depth-bar sticky top-0 z-20 flex min-h-14 shrink-0 items-center gap-2 px-2',
        props.class,
      )}
    >
      <Show when={props.left}>{(left) => <div class="flex shrink-0 items-center">{left()}</div>}</Show>

      <div class="flex min-w-0 flex-1 flex-col justify-center px-1">
        <Show when={props.title}>
          {(title) => (
            <h1 class="truncate text-lg leading-tight font-semibold text-foreground">{title()}</h1>
          )}
        </Show>
        <Show when={props.subtitle}>
          {(subtitle) => <p class="truncate text-xs text-muted">{subtitle()}</p>}
        </Show>
      </div>

      <Show when={props.right}>
        {(right) => <div class="flex shrink-0 items-center gap-1">{right()}</div>}
      </Show>
    </header>
  )
}
