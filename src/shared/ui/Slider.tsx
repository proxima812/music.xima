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
  /**
   * `media` — тонкая полоса перемотки: дорожка 6 px, круглая ручка.
   * Пресет HeroUI рассчитан на «толстый» слайдер настроек: дорожка 20 px,
   * прямоугольная заливка внутри скруглённой дорожки и ручка-пилюля того же
   * цвета — под треком это читается как переключатель, а не как прогресс.
   */
  variant?: 'default' | 'media'
  /**
   * Плавно догонять значение вместо скачков. Позиция трека приезжает тиком раз
   * в ~500 мс, и без этого полоса дёргается ступеньками. Во время перетаскивания
   * выключается — иначе ручка отстаёт от пальца.
   */
  smooth?: boolean
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

      <Show
        when={props.variant === 'media'}
        fallback={
          <KobalteSlider.Track class="slider__track">
            <KobalteSlider.Fill class="slider__fill" />
            <KobalteSlider.Thumb class="slider__thumb top-0" aria-label={props.ariaLabel}>
              <KobalteSlider.Input />
            </KobalteSlider.Thumb>
          </KobalteSlider.Track>
        }
      >
        <KobalteSlider.Track
          class={cn(
            'relative h-1.5 w-full touch-none rounded-full bg-default',
            // Зона попадания пальцем. Дорожка — 6 px, попасть в неё на ходу
            // невозможно, поэтому растягиваем область указателя псевдоэлементом:
            // он не влияет на вёрстку и не меняет `getBoundingClientRect()`
            // дорожки, по которому Kobalte считает позицию.
            "before:absolute before:inset-x-0 before:-inset-y-5 before:block before:content-['']",
          )}
        >
          <KobalteSlider.Fill
            class={cn(
              'absolute h-full rounded-full bg-accent',
              props.smooth === true && 'transition-[width] duration-500 ease-linear',
            )}
          />
          <KobalteSlider.Thumb
            class={cn(
              'top-1/2 size-3.5 -translate-y-1/2 rounded-full bg-foreground shadow-[0_1px_4px_rgba(0,0,0,0.45)] outline-none',
              'transition-transform duration-150 ease-out-fluid focus-visible:ring-2 focus-visible:ring-accent active:scale-125',
              props.smooth === true && 'transition-[left,transform] duration-500 ease-linear',
            )}
            aria-label={props.ariaLabel}
          >
            <KobalteSlider.Input />
          </KobalteSlider.Thumb>
        </KobalteSlider.Track>
      </Show>
    </KobalteSlider>
  )
}
