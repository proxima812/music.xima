import { FolderOpen, FolderPlus, RefreshCw, Trash2 } from 'lucide-solid'
import { createEffect, createResource, createSignal, For, onCleanup, onMount, Show } from 'solid-js'

import { useSettings } from '@/app/providers'
import {
  libraryPickFolder,
  libraryRemoveRoot,
  libraryRoots,
  libraryScan,
  libraryScanStatus,
  onScanProgress,
  toIpcError,
  type ScanMode,
  type ScanStatus,
} from '@/shared/ipc'
import { formatCount, settled } from '@/shared/lib'
import {
  Button,
  clearArtworkCache,
  ConfirmDialog,
  EmptyState,
  IconButton,
  SectionHeader,
  toast,
} from '@/shared/ui'

const PHASE_LABELS: Record<string, string> = {
  idle: 'Ожидание',
  enumerating: 'Поиск файлов',
  reading: 'Чтение тегов',
  writing: 'Запись в базу',
  cleanup: 'Очистка',
  done: 'Готово',
}

function phaseLabel(phase: string): string {
  return PHASE_LABELS[phase.toLowerCase()] ?? phase
}

/** Из SAF tree URI делаем что-то читаемое: `primary:Music/Rock` → `Music/Rock`. */
function rootLabel(uri: string): string {
  let decoded = uri
  try {
    decoded = decodeURIComponent(uri)
  } catch {
    decoded = uri
  }

  const separator = decoded.lastIndexOf(':')
  const tail = separator === -1 ? decoded : decoded.slice(separator + 1)
  return tail === '' ? decoded : tail
}

/** Папки библиотеки и сканирование. */
export function LibrarySection() {
  const settings = useSettings()

  const [roots, { refetch }] = createResource(libraryRoots)
  const [status, setStatus] = createSignal<ScanStatus | null>(null)
  const [scanning, setScanning] = createSignal(false)
  const [picking, setPicking] = createSignal(false)
  const [rootToRemove, setRootToRemove] = createSignal<string | null>(null)

  onMount(() => {
    void libraryScanStatus()
      .then((initial) => {
        setStatus(initial)
        setScanning(initial.running)
      })
      .catch((error: unknown) => {
        console.error('[settings] не удалось прочитать статус сканирования', error)
      })

    const subscription = onScanProgress((next) => {
      setStatus(next)
      if (!next.running) setScanning(false)
    })

    onCleanup(() => {
      void subscription.then((unlisten) => {
        unlisten()
      })
    })
  })

  // Список корней держим в настройках, чтобы плеер и экраны видели то же самое.
  createEffect(() => {
    const list = roots()
    if (list === undefined) return

    const stored = settings.value().scanRoots
    const same = stored.length === list.length && stored.every((item, i) => item === list[i])
    if (!same) settings.set('scanRoots', [...list])
  })

  // Зеркало, а не сам ресурс: чтение в загрузке поднимает общий `<Suspense>`,
  // и добавление папки вынимало бы экран настроек из DOM (docs/BUGS.md, B8).
  const settledRoots = settled(roots)

  const rootList = (): readonly string[] => settledRoots() ?? []

  const progress = (): number | null => {
    const current = status()
    if (current === null || current.total <= 0) return null
    return Math.min(100, Math.round((current.scanned / current.total) * 100))
  }

  const addFolder = (): void => {
    setPicking(true)
    void libraryPickFolder()
      .then((uri) => {
        if (uri === null) return
        void refetch()
        toast({
          title: 'Папка добавлена',
          description: 'Запустите сканирование, чтобы её содержимое попало в библиотеку.',
        })
      })
      .catch((error: unknown) => {
        toast({
          title: 'Не удалось выбрать папку',
          description: toIpcError(error).message,
          variant: 'danger',
        })
      })
      .finally(() => {
        setPicking(false)
      })
  }

  const removeRoot = (uri: string): Promise<void> =>
    libraryRemoveRoot(uri)
      .then(() => {
        void refetch()
        setRootToRemove(null)
        toast({ title: 'Папка отключена', description: rootLabel(uri) })
      })
      .catch((error: unknown) => {
        toast({
          title: 'Не удалось отключить папку',
          description: toIpcError(error).message,
          variant: 'danger',
        })
      })

  const runScan = (mode: ScanMode): void => {
    if (scanning()) return

    setScanning(true)
    void libraryScan([...rootList()], mode)
      .then((result) => {
        clearArtworkCache()
        toast({
          title: 'Сканирование завершено',
          description: `Добавлено ${formatCount(result.added)}, обновлено ${formatCount(
            result.updated,
          )}, удалено ${formatCount(result.removed)}`,
          variant: 'success',
        })
      })
      .catch((error: unknown) => {
        toast({
          title: 'Сканирование не удалось',
          description: toIpcError(error).message,
          variant: 'danger',
        })
      })
      .finally(() => {
        setScanning(false)
      })
  }

  return (
    <section>
      <SectionHeader
        title="Папки библиотеки"
        description="Музыка читается из выбранных папок и общей медиатеки телефона"
        action={
          <Button size="sm" variant="tertiary" pending={picking()} onClick={addFolder}>
            <FolderPlus aria-hidden="true" />
            Добавить
          </Button>
        }
      />

      <div class="px-4">
        <Show
          when={rootList().length > 0}
          fallback={
            <EmptyState
              icon={<FolderOpen aria-hidden="true" />}
              title="Папки не выбраны"
              description="Сканирование пройдёт по общей медиатеке телефона."
              class="py-8"
            />
          }
        >
          <ul class="depth-raised flex flex-col divide-y divide-separator overflow-hidden rounded-2xl">
            <For each={rootList()}>
              {(uri) => (
                <li class="flex min-h-14 items-center gap-2 py-2 pr-1 pl-3">
                  <FolderOpen class="size-5 shrink-0 text-muted" aria-hidden="true" />
                  <span class="min-w-0 flex-1 truncate text-sm text-foreground">
                    {rootLabel(uri)}
                  </span>
                  <IconButton
                    label={`Отключить папку ${rootLabel(uri)}`}
                    variant="ghost"
                    onClick={() => setRootToRemove(uri)}
                  >
                    <Trash2 aria-hidden="true" />
                  </IconButton>
                </li>
              )}
            </For>
          </ul>
        </Show>
      </div>

      <SectionHeader title="Сканирование" />

      <div class="flex flex-col gap-3 px-4">
        <div class="flex gap-2">
          <Button variant="primary" pending={scanning()} onClick={() => runScan('INCREMENTAL')}>
            <RefreshCw aria-hidden="true" />
            Пересканировать
          </Button>
          <Button variant="ghost" disabled={scanning()} onClick={() => runScan('FULL')}>
            Полное
          </Button>
        </div>

        <Show when={status()}>
          {(current) => (
            <div class="depth-raised flex flex-col gap-2 rounded-2xl p-3">
              <div class="flex items-center justify-between gap-2 text-xs text-muted">
                <span class="truncate">{phaseLabel(current().phase)}</span>
                <span class="shrink-0 tabular-nums">
                  {formatCount(current().scanned)}
                  <Show when={current().total > 0}> / {formatCount(current().total)}</Show>
                </span>
              </div>

              <div class="h-1.5 w-full overflow-hidden rounded-full bg-default">
                <div
                  class="h-full rounded-full bg-accent transition-[width] duration-200"
                  style={{ width: `${progress() ?? (current().running ? 15 : 0)}%` }}
                />
              </div>
            </div>
          )}
        </Show>
      </div>

      <ConfirmDialog
        open={rootToRemove() !== null}
        onOpenChange={(open) => {
          if (!open) setRootToRemove(null)
        }}
        title="Отключить папку?"
        description="Треки из неё исчезнут из библиотеки. Файлы на устройстве останутся."
        confirmLabel="Отключить"
        danger
        onConfirm={() => {
          const uri = rootToRemove()
          return uri === null ? undefined : removeRoot(uri)
        }}
      />
    </section>
  )
}
