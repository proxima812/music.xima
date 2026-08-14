type Timer = ReturnType<typeof setTimeout>

export type Debounced<A extends unknown[]> = {
  (...args: A): void
  /** Отменить отложенный вызов. */
  cancel(): void
  /** Выполнить отложенный вызов немедленно, если он есть. */
  flush(): void
  /** Есть ли невыполненный вызов. */
  pending(): boolean
}

export type Throttled<A extends unknown[]> = {
  (...args: A): void
  cancel(): void
}

/** Вызов срабатывает через `waitMs` после последнего обращения. */
export function debounce<A extends unknown[]>(
  fn: (...args: A) => void,
  waitMs: number,
): Debounced<A> {
  let timer: Timer | null = null
  let lastArgs: A | null = null

  const run = (): void => {
    timer = null
    const args = lastArgs
    lastArgs = null
    if (args !== null) fn(...args)
  }

  const debounced = (...args: A): void => {
    lastArgs = args
    if (timer !== null) clearTimeout(timer)
    timer = setTimeout(run, waitMs)
  }

  debounced.cancel = (): void => {
    if (timer !== null) clearTimeout(timer)
    timer = null
    lastArgs = null
  }

  debounced.flush = (): void => {
    if (timer === null) return
    clearTimeout(timer)
    run()
  }

  debounced.pending = (): boolean => timer !== null

  return debounced
}

/** Не чаще одного вызова в `intervalMs`: первый — сразу, последний — в конце окна. */
export function throttle<A extends unknown[]>(
  fn: (...args: A) => void,
  intervalMs: number,
): Throttled<A> {
  let timer: Timer | null = null
  let lastCallAt = 0
  let trailingArgs: A | null = null

  const run = (args: A): void => {
    lastCallAt = Date.now()
    fn(...args)
  }

  const throttled = (...args: A): void => {
    const remaining = intervalMs - (Date.now() - lastCallAt)

    if (remaining <= 0) {
      if (timer !== null) {
        clearTimeout(timer)
        timer = null
      }
      trailingArgs = null
      run(args)
      return
    }

    trailingArgs = args
    if (timer !== null) return

    timer = setTimeout(() => {
      timer = null
      const args_ = trailingArgs
      trailingArgs = null
      if (args_ !== null) run(args_)
    }, remaining)
  }

  throttled.cancel = (): void => {
    if (timer !== null) clearTimeout(timer)
    timer = null
    trailingArgs = null
  }

  return throttled
}

export const sleep = (ms: number): Promise<void> =>
  new Promise((resolve) => {
    setTimeout(resolve, ms)
  })
