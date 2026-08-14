package com.xima.music.player

import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject

/*
 * Мост между `org.json` и data-классами плагина. Без сторонних библиотек: Rust шлёт
 * обычный JSON, читаем его руками и ни на одном поле не падаем.
 */

/** `JSONObject.put(key, null)` удаляет ключ, а Rust ждёт явный `null` для `Option`. */
internal fun JSObject.putOrNull(key: String, value: Any?): JSObject = put(key, value ?: JSONObject.NULL)

/**
 * Аргументы команды. Команды без аргументов получают из Rust литерал `null`,
 * на котором конструктор `JSObject` бросает исключение — отсюда `null` вместо падения.
 */
internal fun Invoke.argsOrNull(): JSObject? = try {
    getArgs()
} catch (_: JSONException) {
    null
}

internal fun JSONObject.stringOrNull(key: String): String? {
    if (isNull(key)) return null
    val value = opt(key) ?: return null
    val text = if (value is String) value else value.toString()
    return text.takeIf { it.isNotEmpty() }
}

internal fun JSONObject.longOrNull(key: String): Long? {
    if (isNull(key)) return null
    return when (val value = opt(key)) {
        is Number -> value.toLong()
        is String -> value.trim().toLongOrNull()
        else -> null
    }
}

internal fun JSONObject.intOrNull(key: String): Int? {
    val value = longOrNull(key) ?: return null
    return when {
        value > Int.MAX_VALUE -> Int.MAX_VALUE
        value < Int.MIN_VALUE -> Int.MIN_VALUE
        else -> value.toInt()
    }
}

internal fun JSONObject.floatOrNull(key: String): Float? {
    if (isNull(key)) return null
    val value = when (val raw = opt(key)) {
        is Number -> raw.toFloat()
        is String -> raw.trim().toFloatOrNull()
        else -> null
    }
    return value?.takeIf { it.isFinite() }
}

internal fun JSONObject.booleanOrNull(key: String): Boolean? {
    if (isNull(key)) return null
    return when (val value = opt(key)) {
        is Boolean -> value
        is String -> value.trim().toBooleanStrictOrNull()
        else -> null
    }
}

/** Элементы очереди из `SetQueueRequest.items` / `ItemsArgs.items`. Мусорные элементы отбрасываем. */
internal fun JSONObject.queueItems(key: String): List<QueueItem> {
    val array = opt(key) as? JSONArray ?: return emptyList()
    val items = ArrayList<QueueItem>(array.length())
    for (index in 0 until array.length()) {
        val json = array.opt(index) as? JSONObject ?: continue
        val item = queueItemFrom(json) ?: continue
        items.add(item)
    }
    return items
}

/** Без `trackId` и `uri` элемент бесполезен: играть нечего и обратно в БД не смапить. */
internal fun queueItemFrom(json: JSONObject): QueueItem? {
    val trackId = json.longOrNull("trackId") ?: return null
    val uri = json.stringOrNull("uri") ?: return null
    return QueueItem(
        trackId = trackId,
        uri = uri,
        title = json.stringOrNull("title") ?: UNKNOWN_TITLE,
        artist = json.stringOrNull("artist"),
        album = json.stringOrNull("album"),
        durationMs = json.longOrNull("durationMs")?.coerceAtLeast(0L) ?: 0L,
        artworkUri = json.stringOrNull("artworkUri"),
    )
}

internal fun jsArrayOfLongs(values: List<Long>): JSArray {
    val array = JSArray()
    for (value in values) {
        array.put(value)
    }
    return array
}

private const val UNKNOWN_TITLE = "Unknown"
