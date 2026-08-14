import { Dialog } from '@kobalte/core/dialog'
import { createEffect, createSignal, on, Show } from 'solid-js'

import { Button } from '@/shared/ui'
import {
  MAX_PLAYLIST_NAME_LEN,
  renamePlaylist,
  validatePlaylistName,
} from '../model/playlists-store'

export type RenamePlaylistDialogProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  playlistId: number
  initialName: string
  onRenamed?: (name: string) => void
}

/** Модалка переименования: то же поле и та же валидация, что при создании. */
export function RenamePlaylistDialog(props: RenamePlaylistDialogProps) {
  const [name, setName] = createSignal('')
  const [error, setError] = createSignal<string | null>(null)
  const [pending, setPending] = createSignal(false)

  createEffect(
    on(
      () => props.open,
      (open) => {
        if (!open) return
        setName(props.initialName)
        setError(null)
        setPending(false)
      },
    ),
  )

  const submit = (): void => {
    if (pending()) return

    const failure = validatePlaylistName(name())
    if (failure !== null) {
      setError(failure)
      return
    }

    const value = name().trim()
    if (value === props.initialName) {
      props.onOpenChange(false)
      return
    }

    setPending(true)
    void renamePlaylist(props.playlistId, value).then((ok) => {
      setPending(false)
      if (!ok) return
      props.onRenamed?.(value)
      props.onOpenChange(false)
    })
  }

  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange} modal preventScroll>
      <Dialog.Portal>
        <Dialog.Overlay class="modal__backdrop modal__backdrop--blur h-dvh animate-in fade-in-0 duration-150" />
        <div class="fixed inset-0 z-50 modal__container h-dvh" data-placement="center">
          <Dialog.Content
            data-placement="center"
            class="modal__dialog modal__dialog--sm animate-in fade-in-0 zoom-in-95 duration-150"
          >
            <form
              onSubmit={(event) => {
                event.preventDefault()
                submit()
              }}
            >
              <div class="modal__header">
                <Dialog.Title class="modal__heading text-base font-semibold">
                  Переименовать плейлист
                </Dialog.Title>
              </div>

              <div class="modal__body flex flex-col gap-1.5">
                <input
                  class="input input--secondary h-11 w-full"
                  type="text"
                  autofocus
                  autocapitalize="sentences"
                  autocomplete="off"
                  spellcheck={false}
                  enterkeyhint="done"
                  maxLength={MAX_PLAYLIST_NAME_LEN}
                  aria-label="Название плейлиста"
                  aria-invalid={error() !== null ? 'true' : undefined}
                  value={name()}
                  disabled={pending()}
                  onInput={(event) => {
                    setName(event.currentTarget.value)
                    setError(null)
                  }}
                />

                <Show when={error()}>
                  {(message) => (
                    <p class="error-message text-xs text-danger" role="alert">
                      {message()}
                    </p>
                  )}
                </Show>
              </div>

              <div class="modal__footer">
                <Button
                  variant="ghost"
                  disabled={pending()}
                  onClick={() => props.onOpenChange(false)}
                >
                  Отмена
                </Button>
                <Button type="submit" variant="primary" pending={pending()}>
                  Сохранить
                </Button>
              </div>
            </form>
          </Dialog.Content>
        </div>
      </Dialog.Portal>
    </Dialog>
  )
}
