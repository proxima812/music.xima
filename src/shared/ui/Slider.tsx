import { Slider as KobalteSlider } from '@kobalte/core/slider'
import { Show, type JSX } from 'solid-js'

import { cn } from '@/shared/lib'

export type SliderProps = {
  value: number
  onChange: (value: number) => void
  /** Вызывается один раз в конце перетаскивания — под запись в настройки/seek. */
  onChangeEnd?: (value: number) => void
  min?: number
  max?: number
  step?: number
  label?: JSX.Element
  /** Текст справа от лейбла: громкость, время, количество. */
  valueLabel?: JSX.Element
  disabled?: boolean
  ariaLabel?: string
  class?: string
}

/** Слайдер на Kobalte со стилями HeroUI (`.slider__*`). Значение — одно число. */
export function Slider(props: SliderProps) {
  const pick = (values: number[]): number | undefined => values[0]

  return (
    <KobalteSlider
      class={cn('slider', props.class)}
      value={[props.value]}
      minValue={props.min ?? 0}
      maxValue={props.max ?? 100}
      step={props.step ?? 1}
      disabled={props.disabled ?? false}
      onChange={(values) => {
        const next = pick(values)
        if (next !== undefined) props.onChange(next)
      }}
      onChangeEnd={(values) => {
        const next = pick(values)
        if (next !== undefined) props.onChangeEnd?.(next)
      }}
    >
      <Show when={props.label}>
        {(label) => (
          <KobalteSlider.Label data-slot="label" class="text-sm font-medium text-foreground">
            {label()}
          </KobalteSlider.Label>
        )}
      </Show>

      <Show when={props.valueLabel}>
        {(valueLabel) => <div class="slider__output">{valueLabel()}</div>}
      </Show>

      <KobalteSlider.Track class="slider__track">
        <KobalteSlider.Fill class="slider__fill" />
        <KobalteSlider.Thumb class="slider__thumb top-0" aria-label={props.ariaLabel}>
          <KobalteSlider.Input />
        </KobalteSlider.Thumb>
      </KobalteSlider.Track>
    </KobalteSlider>
  )
}
