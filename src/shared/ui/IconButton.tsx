import { splitProps } from 'solid-js'

import { cn } from '@/shared/lib'
import { Button, type ButtonProps } from './Button'

export type IconButtonProps = Omit<ButtonProps, 'iconOnly' | 'fullWidth'> & {
  /** Обязателен: у кнопки-иконки нет текста, читалке нужен `aria-label`. */
  label: string
}

/** Круглая кнопка под одну иконку. Тач-таргет держим не меньше 44px. */
export function IconButton(props: IconButtonProps) {
  const [local, rest] = splitProps(props, ['label', 'class', 'variant', 'size'])

  return (
    <Button
      {...rest}
      iconOnly
      variant={local.variant ?? 'ghost'}
      size={local.size ?? 'md'}
      aria-label={local.label}
      class={cn(local.size === 'sm' ? undefined : 'min-h-11 min-w-11', local.class)}
    />
  )
}
