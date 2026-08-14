import { Tabs as KobalteTabs } from '@kobalte/core/tabs'
import { For, type JSX, type ParentProps } from 'solid-js'

import { cn } from '@/shared/lib'

export type TabItem = {
  value: string
  label: JSX.Element
  disabled?: boolean
}

export type TabsProps = ParentProps<{
  items: readonly TabItem[]
  value: string
  onChange: (value: string) => void
  variant?: 'primary' | 'secondary'
  class?: string
  listClass?: string
}>

/**
 * Табы/сегментированный переключатель на Kobalte со стилями HeroUI (`.tabs__*`).
 * Панели не обязательны: без `children` компонент работает как переключатель.
 * `data-selected` дублируем вручную — HeroUI ждёт `"true"`, Kobalte пишет `""`.
 */
export function Tabs(props: TabsProps) {
  return (
    <KobalteTabs
      class={cn('tabs', props.variant === 'secondary' && 'tabs--secondary', props.class)}
      value={props.value}
      onChange={props.onChange}
    >
      <div class={cn('tabs__list-container', props.listClass)}>
        <KobalteTabs.List class="tabs__list">
          <For each={props.items}>
            {(item) => (
              <KobalteTabs.Trigger
                class="tabs__tab"
                value={item.value}
                disabled={item.disabled ?? false}
                data-selected={item.value === props.value ? 'true' : undefined}
              >
                {item.label}
              </KobalteTabs.Trigger>
            )}
          </For>
          <KobalteTabs.Indicator class="tabs__indicator" />
        </KobalteTabs.List>
      </div>

      {props.children}
    </KobalteTabs>
  )
}

export type TabPanelProps = ParentProps<{
  value: string
  class?: string
}>

/** Панель таба; рендерится внутри `<Tabs>`. */
export function TabPanel(props: TabPanelProps) {
  return (
    <KobalteTabs.Content class={cn('tabs__panel', props.class)} value={props.value}>
      {props.children}
    </KobalteTabs.Content>
  )
}
