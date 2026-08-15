import { createEffect, createSignal, untrack, type Accessor, type Resource } from 'solid-js'

/**
 * Значение ресурса, которое **не поднимает `<Suspense>`**.
 *
 * Ресурс, прочитанный в разметке, пока летит запрос, регистрируется в
 * ближайшей границе ожидания — а она в приложении одна, общая (`App.tsx`).
 * На время загрузки Solid вынимает из DOM весь экран и вставляет обратно:
 * вместе с ним теряются прокрутка, фокус и позиция каретки. Из-за этого
 * список треков не листался пальцем (docs/BUGS.md, B8).
 *
 * Зеркало обновляется в пользовательском эффекте, а в таких Solid ресурс в
 * границе не регистрирует — значение остаётся реактивным, но экран не
 * трогается. Пока идёт перезагрузка, отдаётся прошлое значение; ошибку зеркало
 * не бросает — для неё есть `resource.error`.
 */
export function settled<T>(resource: Resource<T>): Accessor<T | undefined> {
  const initial = untrack(() => (resource.error === undefined ? resource() : undefined))
  const [value, setValue] = createSignal<T | undefined>(initial)

  createEffect(() => {
    if (resource.error !== undefined) return
    setValue(() => resource())
  })

  return value
}
