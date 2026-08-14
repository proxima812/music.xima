export type ClassValue = string | false | null | undefined

/** Склейка классов: `cn('btn', isActive && 'btn--active')`. */
export function cn(...classes: ClassValue[]): string {
  let result = ''
  for (const value of classes) {
    if (value === '' || value === false || value === null || value === undefined) continue
    result = result === '' ? value : `${result} ${value}`
  }
  return result
}
