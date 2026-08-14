import { useNavigate } from '@solidjs/router'
import { ChevronRight, Folder as FolderIcon, ListMusic } from 'lucide-solid'
import { createResource, createSignal, For, Match, Show, Switch } from 'solid-js'

import type { Folder } from '@/shared/ipc'
import { formatPlural } from '@/shared/lib'
import { IconButton, Separator, toast } from '@/shared/ui'
import {
  encodeRouteParam,
  libraryVersion,
  loadFolders,
  TRACK_FORMS,
} from '../model/library-store'
import { EmptyLibraryState, ErrorState, ListSkeleton } from './LibraryStates'

/** Папки: обход дерева вглубь с хлебными крошками; лист открывает список треков. */
export function FoldersTab() {
  const navigate = useNavigate()

  // Стек пройденных папок: путь может быть многосегментным, разбирать его нельзя.
  const [stack, setStack] = createSignal<readonly Folder[]>([])
  const [entering, setEntering] = createSignal<string | null>(null)

  const parent = (): string | null => {
    const items = stack()
    const last = items[items.length - 1]
    return last === undefined ? null : last.path
  }

  const [folders, { refetch }] = createResource(
    () => ({ parent: parent(), version: libraryVersion() }),
    (key) => loadFolders(key.parent),
  )

  const openTracks = (folder: Folder): void => {
    navigate(`/library/folder/${encodeRouteParam(folder.path)}`)
  }

  const enter = (folder: Folder): void => {
    setEntering(folder.path)

    loadFolders(folder.path)
      .then((children) => {
        setEntering(null)
        if (children.length === 0) {
          openTracks(folder)
          return
        }
        setStack((previous) => [...previous, folder])
      })
      .catch((error: unknown) => {
        setEntering(null)
        console.error(`[library] не удалось открыть папку ${folder.path}`, error)
        toast({ title: 'Не удалось открыть папку', variant: 'danger' })
      })
  }

  const goUpTo = (depth: number): void => {
    setStack((previous) => previous.slice(0, depth))
  }

  return (
    <div class="flex h-full min-h-0 flex-col">
      <div class="flex shrink-0 items-center gap-1 overflow-x-auto scrollbar-none px-4 py-1">
        <button
          type="button"
          class="shrink-0 py-2 text-xs text-muted no-highlight"
          onClick={() => {
            goUpTo(0)
          }}
        >
          Все папки
        </button>
        <For each={stack()}>
          {(folder, index) => (
            <>
              <ChevronRight size={14} class="shrink-0 text-muted" aria-hidden="true" />
              <button
                type="button"
                class="max-w-40 shrink-0 truncate py-2 text-xs text-foreground no-highlight"
                onClick={() => {
                  goUpTo(index() + 1)
                }}
              >
                {folder.name}
              </button>
            </>
          )}
        </For>
      </div>

      <Switch>
        <Match when={folders.error !== undefined}>
          <ErrorState
            message={errorMessage(folders.error)}
            onRetry={() => {
              void refetch()
            }}
          />
        </Match>

        <Match when={folders.loading}>
          <ListSkeleton rows={6} />
        </Match>

        <Match when={folders()?.length === 0}>
          <EmptyLibraryState
            icon={<FolderIcon aria-hidden="true" />}
            title="Папок пока нет"
            description="Добавьте папку с музыкой в настройках"
            withSettings={stack().length === 0}
          />
        </Match>

        <Match when={folders()}>
          {(items) => (
            <div class="min-h-0 flex-1 overflow-y-auto overscroll-contain scrollbar-none pb-player-stack">
              <For each={items()}>
                {(folder, index) => (
                  <>
                    <Show when={index() > 0}>
                      <Separator class="ms-4" />
                    </Show>
                    <div class="flex min-h-14 w-full items-center gap-2 pr-1 pl-4">
                      <button
                        type="button"
                        class="flex min-w-0 flex-1 items-center gap-3 py-2 text-start no-highlight"
                        disabled={entering() === folder.path}
                        onClick={() => {
                          enter(folder)
                        }}
                      >
                        <FolderIcon size={20} class="shrink-0 text-muted" aria-hidden="true" />
                        <span class="flex min-w-0 flex-1 flex-col gap-0.5">
                          <span class="truncate text-sm font-medium text-foreground">
                            {folder.name}
                          </span>
                          <span class="truncate text-xs text-muted">
                            {formatPlural(folder.trackCount, TRACK_FORMS)}
                          </span>
                        </span>
                      </button>

                      <IconButton
                        label={`Треки папки ${folder.name}`}
                        onClick={() => {
                          openTracks(folder)
                        }}
                      >
                        <ListMusic size={18} aria-hidden="true" />
                      </IconButton>
                    </div>
                  </>
                )}
              </For>
            </div>
          )}
        </Match>
      </Switch>
    </div>
  )
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  return typeof error === 'string' ? error : 'Неизвестная ошибка'
}
