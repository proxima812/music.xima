package com.xima.music.player.library

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import android.util.Log
import app.tauri.plugin.JSObject

/**
 * Всё, что плагин умеет про библиотеку: индексация MediaStore и SAF-деревьев,
 * доступ к выбранным папкам и кэш обложек.
 *
 * Обходы длиннее одного ответа, а Rust-сторона курсор обратно не передаёт
 * (`scanMediaStore(since)` / `scanTree(treeUri, since)` из §7.1), поэтому
 * позиция обхода живёт здесь: повторный вызов с теми же аргументами
 * продолжает с места остановки, а завершённый скан начинается заново.
 */
class MusicLibrary(activity: Activity) {

    private val context: Context = activity.applicationContext
    private val artwork = ArtworkCache(context)
    private val tagReader = TagReader(context)
    private val mediaStoreScanner = MediaStoreScanner(context, artwork)
    private val safScanner = SafScanner(context, tagReader, artwork)

    private val lock = Any()
    private var mediaStoreSession: MediaStoreScanner.Session? = null
    private val treeSessions = LinkedHashMap<String, SafScanner.Session>()

    fun scanMediaStore(since: Long?): ScanBatchResult {
        val session = synchronized(lock) {
            val existing = mediaStoreSession?.takeIf { it.since == since }
            existing ?: mediaStoreScanner.newSession(since).also { mediaStoreSession = it }
        }

        val result = synchronized(session) {
            try {
                mediaStoreScanner.next(session)
            } catch (error: Exception) {
                Log.w(TAG, "скан MediaStore прерван", error)
                EMPTY
            }
        }

        if (result.complete) {
            synchronized(lock) { if (mediaStoreSession === session) mediaStoreSession = null }
        }
        return result
    }

    fun scanTree(treeUri: String, since: Long?): ScanBatchResult {
        val root = treeUri.trim()
        if (root.isEmpty()) return EMPTY

        val key = sessionKey(root, since)
        val session = synchronized(lock) {
            treeSessions[key] ?: newTreeSession(root, since, key)
        }

        val result = synchronized(session) {
            try {
                safScanner.next(session)
            } catch (error: Exception) {
                Log.w(TAG, "обход дерева $root прерван", error)
                EMPTY
            }
        }

        if (result.complete) {
            synchronized(lock) { treeSessions.remove(key) }
        }
        return result
    }

    /** Корни, доступ к которым система за нами сохранила. */
    fun persistedRoots(): List<String> = try {
        context.contentResolver.persistedUriPermissions
            .filter { it.isReadPermission && DocumentsContract.isTreeUri(it.uri) }
            .map { it.uri.toString() }
    } catch (error: Exception) {
        Log.w(TAG, "список сохранённых разрешений недоступен", error)
        emptyList()
    }

    fun releaseRoot(treeUri: String) {
        val root = treeUri.trim()
        if (root.isEmpty()) return

        synchronized(lock) {
            val stale = treeSessions.filterValues { it.treeUri == root }.keys.toList()
            for (key in stale) treeSessions.remove(key)
        }

        try {
            context.contentResolver.releasePersistableUriPermission(
                Uri.parse(root),
                Intent.FLAG_GRANT_READ_URI_PERMISSION,
            )
        } catch (error: Exception) {
            Log.w(TAG, "разрешение на $root не отозвано", error)
        }
    }

    /** `coverKey` или URI трека -> `file://` на обложку в кэше. */
    fun extractArtwork(uri: String): String? = try {
        artwork.extract(uri)
    } catch (error: Exception) {
        Log.w(TAG, "обложка не извлечена: $uri", error)
        null
    }

    /** Берём только чтение: писать в папку пользователя плеер не должен. */
    fun takePersistableUriPermission(uri: Uri) {
        try {
            context.contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION,
            )
        } catch (error: Exception) {
            Log.w(TAG, "разрешение на $uri не сохранено", error)
        }
    }

    fun openDocumentTreeIntent(): Intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
        .addFlags(
            Intent.FLAG_GRANT_READ_URI_PERMISSION or
                Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION,
        )

    /** Новый скан корня отменяет незаконченный обход того же корня с другим `since`. */
    private fun newTreeSession(root: String, since: Long?, key: String): SafScanner.Session {
        val stale = treeSessions.filterValues { it.treeUri == root }.keys.toList()
        for (existing in stale) treeSessions.remove(existing)

        return safScanner.newSession(root, since).also { treeSessions[key] = it }
    }

    private fun sessionKey(treeUri: String, since: Long?): String = "$treeUri|${since ?: "full"}"

    companion object {
        private const val TAG = "MusicLibrary"

        private val EMPTY = ScanBatchResult(emptyList(), complete = true, nextCursor = null)

        fun toJson(result: ScanBatchResult): JSObject = result.toJsonObject()
    }
}
