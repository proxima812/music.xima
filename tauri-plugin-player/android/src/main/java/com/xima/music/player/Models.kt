package com.xima.music.player

import app.tauri.plugin.JSObject

/** Имена событий `trigger()` — docs/CONTRACTS.md §7.2. */
internal const val EVENT_STATE = "state"
internal const val EVENT_TRACK_CHANGED = "trackChanged"
internal const val EVENT_QUEUE_CHANGED = "queueChanged"
internal const val EVENT_COMPLETED = "completed"
internal const val EVENT_ERROR = "error"
internal const val EVENT_SCAN_PROGRESS = "scanProgress"

/** Фазы сканирования: те же строки, что и константы `PHASE_*` в `domain/scan.rs`. */
internal const val PHASE_IDLE = "idle"
internal const val PHASE_READING = "reading"
internal const val PHASE_DONE = "done"

/** `PlaybackStatus` из docs/CONTRACTS.md §1.5: имя константы = значение в JSON. */
enum class PlaybackStatus { IDLE, BUFFERING, PLAYING, PAUSED, ENDED }

/** `RepeatMode` из docs/CONTRACTS.md §1.5. */
enum class RepeatMode {
    OFF, ALL, ONE;

    companion object {
        fun fromToken(raw: String?): RepeatMode? {
            val token = raw?.trim()?.uppercase() ?: return null
            return values().firstOrNull { it.name == token }
        }
    }
}

/**
 * `PlaybackState` из docs/CONTRACTS.md §1.5 — единственный источник истины о плеере.
 * Rust его только пробрасывает, поэтому имена полей и nullability трогать нельзя.
 */
data class PlaybackState(
    val status: PlaybackStatus,
    val trackId: Long?,
    val positionMs: Long,
    val durationMs: Long,
    val queueIndex: Int?,
    val queueLength: Int,
    val shuffle: Boolean,
    val repeat: RepeatMode,
    val volume: Float,
    val speed: Float,
) {
    fun toJson(): JSObject = JSObject()
        .put("status", status.name)
        .putOrNull("trackId", trackId)
        .put("positionMs", positionMs)
        .put("durationMs", durationMs)
        .putOrNull("queueIndex", queueIndex)
        .put("queueLength", queueLength)
        .put("shuffle", shuffle)
        .put("repeat", repeat.name)
        .put("volume", finiteOr(volume, 1f))
        .put("speed", finiteOr(speed, 1f))

    companion object {
        /** Ничего не загружено: что отдаёт свежая сессия и что видит фронт до первого трека. */
        fun idle(): PlaybackState = PlaybackState(
            status = PlaybackStatus.IDLE,
            trackId = null,
            positionMs = 0L,
            durationMs = 0L,
            queueIndex = null,
            queueLength = 0,
            shuffle = false,
            repeat = RepeatMode.OFF,
            volume = 1f,
            speed = 1f,
        )
    }
}

/** `QueueItem` из docs/CONTRACTS.md §7.1 — всё, что нужно плееру для звука и нотификации. */
data class QueueItem(
    val trackId: Long,
    val uri: String,
    val title: String,
    val artist: String?,
    val album: String?,
    val durationMs: Long,
    val artworkUri: String?,
) {
    fun toJson(): JSObject = JSObject()
        .put("trackId", trackId)
        .put("uri", uri)
        .put("title", title)
        .putOrNull("artist", artist)
        .putOrNull("album", album)
        .put("durationMs", durationMs)
        .putOrNull("artworkUri", artworkUri)
}

/** В JSON нет NaN/Infinity, а Rust ждёт `f32`: недопустимое значение заменяем дефолтом. */
private fun finiteOr(value: Float, fallback: Float): Double =
    if (value.isFinite()) value.toDouble() else fallback.toDouble()
