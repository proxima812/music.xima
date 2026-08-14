package com.xima.music.player.library

import android.content.Context
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.os.Build
import android.util.Log

/** Теги, вычитанные из самого файла. Всё, чего в файле нет, остаётся `null`. */
internal data class TrackTags(
    val title: String?,
    val artist: String?,
    val album: String?,
    val albumArtist: String?,
    val durationMs: Long,
    val trackNumber: Int?,
    val discNumber: Int?,
    val year: Int?,
    val genre: String?,
    val bitrate: Int?,
    val sampleRate: Int?,
    val mimeType: String?,
)

/**
 * Метаданные файлов, до которых MediaStore не дотягивается (SAF-дерево пользователя).
 *
 * Один битый файл не должен ронять обход: любая ошибка — это `null` и запись в лог.
 */
internal class TagReader(private val context: Context) {

    fun read(uri: Uri): TrackTags? {
        val retriever = MediaMetadataRetriever()
        var tags: TrackTags? = null
        try {
            retriever.setDataSource(context, uri)
            tags = TrackTags(
                title = retriever.text(MediaMetadataRetriever.METADATA_KEY_TITLE),
                artist = retriever.text(MediaMetadataRetriever.METADATA_KEY_ARTIST),
                album = retriever.text(MediaMetadataRetriever.METADATA_KEY_ALBUM),
                albumArtist = retriever.text(MediaMetadataRetriever.METADATA_KEY_ALBUMARTIST),
                durationMs = retriever.number(MediaMetadataRetriever.METADATA_KEY_DURATION) ?: 0L,
                trackNumber = leadingNumber(retriever.text(MediaMetadataRetriever.METADATA_KEY_CD_TRACK_NUMBER)),
                discNumber = leadingNumber(retriever.text(MediaMetadataRetriever.METADATA_KEY_DISC_NUMBER)),
                year = parseYear(
                    retriever.text(MediaMetadataRetriever.METADATA_KEY_YEAR),
                    retriever.text(MediaMetadataRetriever.METADATA_KEY_DATE),
                ),
                genre = parseGenre(retriever.text(MediaMetadataRetriever.METADATA_KEY_GENRE)),
                bitrate = retriever.number(MediaMetadataRetriever.METADATA_KEY_BITRATE)
                    ?.takeIf { it > 0L }
                    ?.coerceAtMost(Int.MAX_VALUE.toLong())
                    ?.toInt(),
                sampleRate = sampleRate(retriever),
                mimeType = retriever.text(MediaMetadataRetriever.METADATA_KEY_MIMETYPE),
            )
        } catch (error: Exception) {
            Log.w(TAG, "не удалось прочитать метаданные: $uri", error)
        } finally {
            runCatching { retriever.release() }
        }
        return tags
    }

    /** `METADATA_KEY_SAMPLERATE` появился только в Android 12. */
    private fun sampleRate(retriever: MediaMetadataRetriever): Int? {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) return null
        return retriever.number(MediaMetadataRetriever.METADATA_KEY_SAMPLERATE)
            ?.takeIf { it > 0L }
            ?.coerceAtMost(Int.MAX_VALUE.toLong())
            ?.toInt()
    }

    private fun MediaMetadataRetriever.text(key: Int): String? =
        normalizeText(runCatching { extractMetadata(key) }.getOrNull())

    private fun MediaMetadataRetriever.number(key: Int): Long? = text(key)?.toLongOrNull()

    /** Теги пишут «3/12» — нужен номер, а не «из скольких». */
    private fun leadingNumber(value: String?): Int? {
        val head = value?.substringBefore('/')?.trim() ?: return null
        val number = head.toIntOrNull() ?: return null
        return number.takeIf { it > 0 }
    }

    private fun parseYear(year: String?, date: String?): Int? =
        year?.let { YEAR_PATTERN.find(it)?.value?.toIntOrNull() }
            ?: date?.let { YEAR_PATTERN.find(it)?.value?.toIntOrNull() }

    /** ID3v1 нумерует жанры: «(17)Rock». */
    private fun parseGenre(value: String?): String? {
        val genre = value ?: return null
        val match = NUMERIC_GENRE.matchEntire(genre) ?: return genre
        return normalizeText(match.groupValues.getOrNull(1))
    }

    private companion object {
        const val TAG = "MusicLibrary/Tags"
        val YEAR_PATTERN = Regex("(19|20)\\d{2}")
        val NUMERIC_GENRE = Regex("\\(\\d+\\)\\s*(.+)")
    }
}
