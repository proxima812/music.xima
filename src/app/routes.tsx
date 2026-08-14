import { Navigate, type RouteDefinition } from '@solidjs/router'
import { lazy } from 'solid-js'

/**
 * Экраны грузятся лениво: стартовый бандл держим маленьким, WebView на телефоне
 * не любит большие чанки. Имя экспорта = имени файла (см. договорённости).
 */

const HomeScreen = lazy(async () => ({
  default: (await import('@/features/home/ui/HomeScreen')).HomeScreen,
}))

const LibraryScreen = lazy(async () => ({
  default: (await import('@/features/library/ui/LibraryScreen')).LibraryScreen,
}))

const AlbumScreen = lazy(async () => ({
  default: (await import('@/features/library/ui/AlbumScreen')).AlbumScreen,
}))

const ArtistScreen = lazy(async () => ({
  default: (await import('@/features/library/ui/ArtistScreen')).ArtistScreen,
}))

const GenreScreen = lazy(async () => ({
  default: (await import('@/features/library/ui/GenreScreen')).GenreScreen,
}))

const FolderScreen = lazy(async () => ({
  default: (await import('@/features/library/ui/FolderScreen')).FolderScreen,
}))

const SearchScreen = lazy(async () => ({
  default: (await import('@/features/search/ui/SearchScreen')).SearchScreen,
}))

const PlaylistsScreen = lazy(async () => ({
  default: (await import('@/features/playlists/ui/PlaylistsScreen')).PlaylistsScreen,
}))

const PlaylistScreen = lazy(async () => ({
  default: (await import('@/features/playlists/ui/PlaylistScreen')).PlaylistScreen,
}))

const SmartPlaylistsScreen = lazy(async () => ({
  default: (await import('@/features/smart-playlists/ui/SmartPlaylistsScreen'))
    .SmartPlaylistsScreen,
}))

const SmartPlaylistEditorScreen = lazy(async () => ({
  default: (await import('@/features/smart-playlists/ui/SmartPlaylistEditorScreen'))
    .SmartPlaylistEditorScreen,
}))

const SettingsScreen = lazy(async () => ({
  default: (await import('@/features/settings/ui/SettingsScreen')).SettingsScreen,
}))

const HiddenTracksScreen = lazy(async () => ({
  default: (await import('@/features/settings/ui/HiddenTracksScreen')).HiddenTracksScreen,
}))

export const routes: RouteDefinition[] = [
  { path: '/', component: HomeScreen },
  { path: '/library', component: LibraryScreen },
  { path: '/library/album/:id', component: AlbumScreen },
  { path: '/library/artist/:id', component: ArtistScreen },
  { path: '/library/genre/:name', component: GenreScreen },
  { path: '/library/folder/:path', component: FolderScreen },
  { path: '/search', component: SearchScreen },
  { path: '/playlists', component: PlaylistsScreen },
  { path: '/playlists/:id', component: PlaylistScreen },
  { path: '/smart', component: SmartPlaylistsScreen },
  { path: '/smart/new', component: SmartPlaylistEditorScreen },
  { path: '/smart/:id/edit', component: SmartPlaylistEditorScreen },
  { path: '/settings', component: SettingsScreen },
  { path: '/settings/hidden-tracks', component: HiddenTracksScreen },
  { path: '*', component: () => <Navigate href="/" /> },
]
