package com.xima.music.player.library

import android.database.Cursor
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import org.json.JSONObject

/**
 * Трек, найденный нативным сканером.
 *
 * Поля 1:1 с `ScannedTrack` из docs/CONTRACTS.md §3 — Rust десериализует этот JSON
 * напрямую, поэтому имена и nullability менять нельзя.
 */
data class ScannedTrackData(
    val uri: String,
    val title: String,
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
    val size: Long,
    val mimeType: String?,
    val folder: String?,
    val dateAdded: Long,
    val lastModified: Long,
    val coverKey: String?,
)

/** Порция обхода: `ScanBatch` из docs/CONTRACTS.md §7.1. */
data class ScanBatchResult(
    val tracks: List<ScannedTrackData>,
    val complete: Boolean,
    val nextCursor: String?,
)

internal fun ScannedTrackData.toJsonObject(): JSObject = JSObject()
    .put("uri", uri)
    .put("title", title)
    .putOrNull("artist", artist)
    .putOrNull("album", album)
    .putOrNull("albumArtist", albumArtist)
    .put("durationMs", durationMs)
    .putOrNull("trackNumber", trackNumber)
    .putOrNull("discNumber", discNumber)
    .putOrNull("year", year)
    .putOrNull("genre", genre)
    .putOrNull("bitrate", bitrate)
    .putOrNull("sampleRate", sampleRate)
    .put("size", size)
    .putOrNull("mimeType", mimeType)
    .putOrNull("folder", folder)
    .put("dateAdded", dateAdded)
    .put("lastModified", lastModified)
    .putOrNull("coverKey", coverKey)

internal fun ScanBatchResult.toJsonObject(): JSObject {
    val items = JSArray()
    for (track in tracks) {
        items.put(track.toJsonObject())
    }
    return JSObject()
        .put("tracks", items)
        .put("complete", complete)
        .putOrNull("nextCursor", nextCursor)
}

/** `JSONObject.put(key, null)` удаляет ключ, а Rust ждёт явный `null` для `Option`. */
private fun JSObject.putOrNull(key: String, value: Any?): JSObject = put(key, value ?: JSONObject.NULL)

internal fun Cursor.stringOrNull(index: Int): String? =
    if (index < 0 || isNull(index)) null else normalizeText(getString(index))

/** Значение как есть: document id нельзя ни тримить, ни превращать в `null` по содержимому. */
internal fun Cursor.rawStringOrNull(index: Int): String? =
    if (index < 0 || isNull(index)) null else getString(index)?.takeIf { it.isNotEmpty() }

internal fun Cursor.longOrNull(index: Int): Long? =
    if (index < 0 || isNull(index)) null else getLong(index)

internal fun Cursor.intOrNull(index: Int): Int? =
    if (index < 0 || isNull(index)) null else getInt(index)

/** MediaStore отдаёт `<unknown>` вместо пустого тега — для домена это отсутствие значения. */
internal fun normalizeText(value: String?): String? {
    val trimmed = value?.trim() ?: return null
    if (trimmed.isEmpty() || trimmed.equals(UNKNOWN_TAG, ignoreCase = true)) return null
    return trimmed
}

/**
 * Display-путь папки: без схемы тома, без корня внешнего хранилища и без ведущего слэша.
 * Принимает и `RELATIVE_PATH` (`Music/Rock/`), и `DATA`, и SAF document id (`primary:Music/Rock`).
 */
internal fun displayFolderPath(raw: String?): String? {
    val source = raw?.trim() ?: return null
    if (source.isEmpty()) return null

    var path = source.replace('\\', '/')
    val volume = path.indexOf(':')
    val firstSlash = path.indexOf('/')
    // Префикс тома идёт до первого слэша (`primary:Music/...`); двоеточие внутри
    // имени папки — это уже часть пути.
    if (volume >= 0 && (firstSlash < 0 || volume < firstSlash)) {
        path = path.substring(volume + 1)
    }
    path = REPEATED_SLASHES.replace(path, "/")
    for (root in ABSOLUTE_ROOTS) {
        val stripped = root.replaceFirst(path, "")
        if (stripped != path) {
            path = stripped
            break
        }
    }
    return path.trim('/').takeIf { it.isNotEmpty() }
}

/** Расширение файла в нижнем регистре, без точки. */
internal fun fileExtension(name: String): String {
    val dot = name.lastIndexOf('.')
    if (dot < 0 || dot == name.length - 1) return ""
    return name.substring(dot + 1).lowercase()
}

internal fun fileBaseName(name: String): String {
    val dot = name.lastIndexOf('.')
    return if (dot <= 0) name else name.substring(0, dot)
}

/** Средний битрейт в бит/с — MediaStore точного не отдаёт, а домену он нужен для Smart Playlists. */
internal fun averageBitrate(size: Long, durationMs: Long): Int? {
    if (size <= 0L || durationMs <= 0L) return null
    val bits = size * 8L * 1000L / durationMs
    if (bits <= 0L) return null
    return if (bits > Int.MAX_VALUE) Int.MAX_VALUE else bits.toInt()
}

/** Размер порции обхода: `ScanBatch` из docs/CONTRACTS.md §7.1 отдаётся кусками. */
internal const val SCAN_BATCH_SIZE = 500

/** Подкаталог `context.cacheDir`, из которого Rust резолвит `coverKey`. */
internal const val ARTWORK_DIR = "artwork"
internal const val ARTWORK_MAX_SIZE = 512
internal const val ARTWORK_QUALITY = 85

internal const val AUDIO_MIME_PREFIX = "audio/"

/** Форматы, которые Media3 умеет играть, а провайдеры часто отдают как `application/octet-stream`. */
internal val AUDIO_EXTENSIONS = setOf(
    "mp3", "flac", "m4a", "m4b", "aac", "ogg", "oga", "opus", "wav", "wma", "mp4", "mka", "aif", "aiff",
)

// Файлы с MIME-типом из группы audio, которые музыкой не являются: плейлисты.
// (Комментарий строчный намеренно: последовательность "/" + "*" открыла бы
// вложенный блочный комментарий — в Kotlin они вкладываются.)
internal val NON_TRACK_EXTENSIONS = setOf("m3u", "m3u8", "pls", "cue", "wpl", "xspf")

private val NON_TRACK_MIMES = setOf(
    "audio/x-mpegurl",
    "audio/mpegurl",
    "audio/x-scpls",
    "application/vnd.apple.mpegurl",
    "application/xspf+xml",
)

/**
 * Музыкальный ли это документ. MIME от SAF-провайдера — подсказка, а не истина:
 * многие отдают `application/octet-stream` даже для flac.
 */
internal fun isAudioDocument(mimeType: String?, displayName: String?): Boolean {
    val extension = displayName?.let { fileExtension(it) }.orEmpty()
    if (extension in NON_TRACK_EXTENSIONS) return false

    val mime = mimeType?.trim()?.lowercase()?.substringBefore(';')
    if (mime != null && mime in NON_TRACK_MIMES) return false
    if (mime != null && mime.startsWith(AUDIO_MIME_PREFIX)) return true
    return extension in AUDIO_EXTENSIONS
}

/** Домен выводит формат трека из MIME, поэтому для `application/octet-stream` он берётся из расширения. */
internal fun mimeFromExtension(extension: String): String? = when (extension) {
    "mp3" -> "audio/mpeg"
    "flac" -> "audio/flac"
    "m4a", "m4b", "mp4" -> "audio/mp4"
    "aac" -> "audio/aac"
    "ogg", "oga" -> "audio/ogg"
    "opus" -> "audio/opus"
    "wav" -> "audio/wav"
    "wma" -> "audio/x-ms-wma"
    "mka" -> "audio/x-matroska"
    "aif", "aiff" -> "audio/aiff"
    else -> null
}

private const val UNKNOWN_TAG = "<unknown>"

private val REPEATED_SLASHES = Regex("/{2,}")

private val ABSOLUTE_ROOTS = listOf(
    Regex("^/storage/emulated/\\d+/?"),
    Regex("^/storage/self/primary/?"),
    Regex("^/storage/[^/]+/?"),
    Regex("^/mnt/sdcard/?"),
    Regex("^/sdcard/?"),
)
