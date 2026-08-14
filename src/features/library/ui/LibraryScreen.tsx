import { useNavigate } from '@solidjs/router'
import { Settings } from 'lucide-solid'
import { createEffect, For, Match, Show, Switch } from 'solid-js'

import { oneOf } from '@/shared/ipc'
import { formatCount } from '@/shared/lib'
import {
  IconButton,
  Screen,
  Skeleton,
  Tabs,
  TopBar,
  type TabItem,
} from '@/shared/ui'
import {
  activeTab,
  ensureLibrarySubscription,
  libraryVersion,
  LIBRARY_TABS,
  setActiveTab,
  statsValue,
} from '../model/library-store'
import { AlbumsTab } from './AlbumsTab'
import { ArtistsTab } from './ArtistsTab'
import { FoldersTab } from './FoldersTab'
import { GenresTab } from './GenresTab'
import { SongsTab } from './SongsTab'

const TAB_ITEMS: readonly TabItem[] = [
  { value: 'songs', label: 'Песни' },
  { value: 'albums', label: 'Альбомы' },
  { value: 'artists', label: 'Исполнители' },
  { value: 'genres', label: 'Жанры' },
  { value: 'folders', label: 'Папки' },
]

/** Библиотека: сводка, недавно добавленные и пять режимов просмотра. */
export function LibraryScreen() {
  const navigate = useNavigate()

  createEffect(() => {
    libraryVersion()
    ensureLibrarySubscription()
    statsValue.ensure()
  })

  const changeTab = (value: string): void => {
    const tab = oneOf(value, LIBRARY_TABS)
    if (tab !== undefined) setActiveTab(tab)
  }

  return (
    <Screen scrollable={false}>
      <TopBar
        title="Библиотека"
        right={
          <IconButton
            label="Настройки"
            onClick={() => {
              navigate('/settings')
            }}
          >
            <Settings size={20} aria-hidden="true" />
          </IconButton>
        }
      />

      <Summary />

      <Tabs items={TAB_ITEMS} value={activeTab()} onChange={changeTab} listClass="px-2" />

      <div class="min-h-0 flex-1">
        <Switch>
          <Match when={activeTab() === 'songs'}>
            <SongsTab />
          </Match>
          <Match when={activeTab() === 'albums'}>
            <AlbumsTab />
          </Match>
          <Match when={activeTab() === 'artists'}>
            <ArtistsTab />
          </Match>
          <Match when={activeTab() === 'genres'}>
            <GenresTab />
          </Match>
          <Match when={activeTab() === 'folders'}>
            <FoldersTab />
          </Match>
        </Switch>
      </div>
    </Screen>
  )
}

/** Четыре счётчика из `library_stats`. */
function Summary() {
  return (
    <Show when={statsValue.data()} fallback={<SummarySkeleton />}>
      {(stats) => (
        <div class="grid shrink-0 grid-cols-4 gap-2 px-4 pb-2">
          <SummaryCell label="Песни" value={stats().tracks} />
          <SummaryCell label="Альбомы" value={stats().albums} />
          <SummaryCell label="Артисты" value={stats().artists} />
          <SummaryCell label="Плейлисты" value={stats().playlists} />
        </div>
      )}
    </Show>
  )
}

function SummaryCell(props: { label: string; value: number }) {
  return (
    <div class="depth-raised flex flex-col items-center gap-0.5 rounded-xl px-1 py-2">
      <span class="text-sm font-semibold text-foreground">{formatCount(props.value)}</span>
      <span class="w-full truncate text-center text-xs text-muted">{props.label}</span>
    </div>
  )
}

function SummarySkeleton() {
  return (
    <Show when={statsValue.error() === null}>
      <div class="grid shrink-0 grid-cols-4 gap-2 px-4 pb-2">
        <For each={[0, 1, 2, 3]}>{() => <Skeleton class="h-14 w-full rounded-xl" />}</For>
      </div>
    </Show>
  )
}

