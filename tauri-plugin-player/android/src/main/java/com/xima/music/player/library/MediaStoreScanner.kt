package com.xima.music.player.library

import android.content.ContentUris
import android.content.Context
import android.database.Cursor
import android.net.Uri
import android.os.Build
import android.provider.MediaStore
import android.util.Log

/**
 * Индексация общей медиатеки телефона через `MediaStore.Audio.Media`.
 *
 * Обход постраничный по `_ID` (keyset): порция не держит в памяти больше
 * [SCAN_BATCH_SIZE] треков, а следующий вызов продолжает с последнего прочитанного id.
 */
internal class MediaStoreScanner(
    private val context: Context,
    private val artwork: ArtworkCache,
) {

    /** Позиция обхода между вызовами `scanMediaStore`. */
    class Session(val since: Long?) {
        internal var lastId: Long = 0L
        internal var batch: Int = 0
    }

    fun newSession(since: Long?): Session = Session(since)

    fun next(session: Session): ScanBatchResult {
        val collection = audioCollection()
        val arguments = mutableListOf<String>()
        val selection = StringBuilder("${MediaStore.Audio.Media.IS_MUSIC} != 0")
        if (session.lastId > 0L) {
            selection.append(" AND ${MediaStore.Audio.Media._ID} > ?")
            arguments.add(session.lastId.toString())
        }
        val since = session.since
        if (since != null) {
            // MediaStore хранит время в секундах; DATE_ADDED ловит файлы,
            // которые появились в индексе позже своей mtime.
            val seconds = (since / 1000L).toString()
            selection.append(
                " AND (${MediaStore.Audio.Media.DATE_MODIFIED} > ?" +
                    " OR ${MediaStore.Audio.Media.DATE_ADDED} > ?)"
            )
            arguments.add(seconds)
            arguments.add(seconds)
        }

        val cursor = try {
            context.contentResolver.query(
                collection,
                projection(),
                selection.toString(),
                arguments.toTypedArray(),
                "${MediaStore.Audio.Media._ID} ASC",
            )
        } catch (error: Exception) {
            Log.w(TAG, "MediaStore недоступен", error)
            null
        } ?: return ScanBatchResult(emptyList(), complete = true, nextCursor = null)

        val tracks = ArrayList<ScannedTrackData>(SCAN_BATCH_SIZE)
        var rows = 0
        var hasMore = false
        cursor.use { rowSet ->
            val columns = Columns(rowSet)
            while (rows < SCAN_BATCH_SIZE && rowSet.moveToNext()) {
                rows += 1
                val id = rowSet.longOrNull(columns.id)
                if (id == null) {
                    continue
                }
                session.lastId = id
                val track = try {
                    read(rowSet, columns, ContentUris.withAppendedId(collection, id))
                } catch (error: Exception) {
                    Log.w(TAG, "пропущена запись MediaStore id=$id", error)
                    null
                }
                if (track != null) {
                    tracks.add(track)
                }
            }
            hasMore = rowSet.count > rows
        }

        session.batch += 1
        val complete = !hasMore
        return ScanBatchResult(
            tracks = tracks,
            complete = complete,
            nextCursor = if (complete) null else "media-store:${session.lastId}",
        )
    }

    private fun read(cursor: Cursor, columns: Columns, trackUri: Uri): ScannedTrackData? {
        val uri = trackUri.toString()
        val displayName = cursor.stringOrNull(columns.displayName)
        val title = cursor.stringOrNull(columns.title)
            ?: displayName?.let { fileBaseName(it) }
            ?: return null

        val durationMs = cursor.longOrNull(columns.duration)?.coerceAtLeast(0L) ?: 0L
        val size = cursor.longOrNull(columns.size)?.coerceAtLeast(0L) ?: 0L
        val raw = cursor.intOrNull(columns.track)
        val disc = if (raw != null && raw >= 1000) (raw / 1000).takeIf { it > 0 } else null
        val number = when {
            raw == null || raw <= 0 -> null
            raw >= 1000 -> (raw % 1000).takeIf { it > 0 }
            else -> raw
        }

        val album = cursor.stringOrNull(columns.album)
        val artist = cursor.stringOrNull(columns.artist)
        val albumArtist = cursor.stringOrNull(columns.albumArtist)
        val albumId = cursor.longOrNull(columns.albumId)
        val folder = displayFolderPath(cursor.stringOrNull(columns.relativePath))
            ?: displayFolderPath(cursor.stringOrNull(columns.data)?.substringBeforeLast('/', ""))

        val coverKey = artwork.ensure(
            key = artwork.keyFor(album, albumArtist, artist, uri),
            source = trackUri,
            albumId = albumId,
        )

        return ScannedTrackData(
            uri = uri,
            title = title,
            artist = artist,
            album = album,
            albumArtist = albumArtist,
            durationMs = durationMs,
            trackNumber = number,
            discNumber = disc,
            year = cursor.intOrNull(columns.year)?.takeIf { it > 0 },
            genre = cursor.stringOrNull(columns.genre),
            bitrate = cursor.intOrNull(columns.bitrate)?.takeIf { it > 0 }
                ?: averageBitrate(size, durationMs),
            sampleRate = null,
            size = size,
            mimeType = mimeType(cursor.stringOrNull(columns.mimeType), displayName),
            folder = folder,
            dateAdded = cursor.longOrNull(columns.dateAdded)?.let { it * 1000L } ?: 0L,
            lastModified = cursor.longOrNull(columns.dateModified)?.let { it * 1000L } ?: 0L,
            coverKey = coverKey,
        )
    }

    /** Формат домен выводит из MIME, а сканер MediaStore иногда отдаёт `application/octet-stream`. */
    private fun mimeType(declared: String?, displayName: String?): String? {
        val mime = declared?.lowercase()
        if (mime != null && mime.startsWith(AUDIO_MIME_PREFIX)) return mime

        val extension = displayName?.let { fileExtension(it) } ?: return mime
        return mimeFromExtension(extension) ?: mime
    }

    private fun audioCollection(): Uri =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            MediaStore.Audio.Media.getContentUri(MediaStore.VOLUME_EXTERNAL)
        } else {
            @Suppress("DEPRECATION")
            MediaStore.Audio.Media.EXTERNAL_CONTENT_URI
        }

    private fun projection(): Array<String> {
        val columns = mutableListOf(
            MediaStore.Audio.Media._ID,
            MediaStore.Audio.Media.TITLE,
            MediaStore.Audio.Media.DISPLAY_NAME,
            MediaStore.Audio.Media.ARTIST,
            MediaStore.Audio.Media.ALBUM,
            MediaStore.Audio.Media.ALBUM_ID,
            MediaStore.Audio.Media.DURATION,
            MediaStore.Audio.Media.TRACK,
            MediaStore.Audio.Media.YEAR,
            MediaStore.Audio.Media.MIME_TYPE,
            MediaStore.Audio.Media.SIZE,
            MediaStore.Audio.Media.DATE_ADDED,
            MediaStore.Audio.Media.DATE_MODIFIED,
        )
        @Suppress("DEPRECATION")
        columns.add(MediaStore.Audio.Media.DATA)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            columns.add(MediaStore.Audio.Media.RELATIVE_PATH)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            columns.add(MediaStore.Audio.Media.ALBUM_ARTIST)
            columns.add(MediaStore.Audio.Media.GENRE)
            columns.add(MediaStore.Audio.Media.BITRATE)
        }
        return columns.toTypedArray()
    }

    /** Индексы колонок считаются один раз на курсор: `getColumnIndex` внутри цикла — это поиск по имени. */
    private class Columns(cursor: Cursor) {
        val id = cursor.getColumnIndex(MediaStore.Audio.Media._ID)
        val title = cursor.getColumnIndex(MediaStore.Audio.Media.TITLE)
        val displayName = cursor.getColumnIndex(MediaStore.Audio.Media.DISPLAY_NAME)
        val artist = cursor.getColumnIndex(MediaStore.Audio.Media.ARTIST)
        val album = cursor.getColumnIndex(MediaStore.Audio.Media.ALBUM)
        val albumId = cursor.getColumnIndex(MediaStore.Audio.Media.ALBUM_ID)
        val duration = cursor.getColumnIndex(MediaStore.Audio.Media.DURATION)
        val track = cursor.getColumnIndex(MediaStore.Audio.Media.TRACK)
        val year = cursor.getColumnIndex(MediaStore.Audio.Media.YEAR)
        val mimeType = cursor.getColumnIndex(MediaStore.Audio.Media.MIME_TYPE)
        val size = cursor.getColumnIndex(MediaStore.Audio.Media.SIZE)
        val dateAdded = cursor.getColumnIndex(MediaStore.Audio.Media.DATE_ADDED)
        val dateModified = cursor.getColumnIndex(MediaStore.Audio.Media.DATE_MODIFIED)

        @Suppress("DEPRECATION")
        val data = cursor.getColumnIndex(MediaStore.Audio.Media.DATA)
        val relativePath =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                cursor.getColumnIndex(MediaStore.Audio.Media.RELATIVE_PATH)
            } else {
                -1
            }
        val albumArtist =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                cursor.getColumnIndex(MediaStore.Audio.Media.ALBUM_ARTIST)
            } else {
                -1
            }
        val genre =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                cursor.getColumnIndex(MediaStore.Audio.Media.GENRE)
            } else {
                -1
            }
        val bitrate =
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                cursor.getColumnIndex(MediaStore.Audio.Media.BITRATE)
            } else {
                -1
            }
    }

    private companion object {
        const val TAG = "MusicLibrary/MediaStore"
    }
}
