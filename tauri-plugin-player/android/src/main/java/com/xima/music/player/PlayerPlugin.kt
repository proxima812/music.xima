package com.xima.music.player

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.os.Build
import android.webkit.WebView
import androidx.activity.result.ActivityResult
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import com.xima.music.player.library.MusicLibrary
import com.xima.music.player.library.ScanBatchResult
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.util.concurrent.atomic.AtomicLong

/**
 * Единственная точка входа из Rust в нативный слой (docs/CONTRACTS.md §7.2).
 *
 * Воспроизведение уходит в [PlaybackController], работа с файлами — в [MusicLibrary]
 * на `Dispatchers.IO`. Наверх всё возвращается событиями `trigger()`, а не поллингом.
 */
internal fun needsLegacyMediaStoreWritePermission(sdkInt: Int, uri: String): Boolean =
    sdkInt <= Build.VERSION_CODES.P &&
        TrackFileDeleter.classifyTarget(uri, isDocumentUri = false, documentFlags = null) ==
        DeleteTarget.MEDIA_STORE

@TauriPlugin(
    permissions = [
        Permission(
            strings = [Manifest.permission.WRITE_EXTERNAL_STORAGE],
            alias = "legacyStorageWrite",
        ),
        // Чтение общей медиатеки. Объявить разрешение в манифесте мало: на 33+
        // без запроса в рантайме MediaStore молча отдаёт пустой курсор, и скан
        // заканчивается нулём треков (docs/BUGS.md, B6).
        Permission(
            strings = [Manifest.permission.READ_MEDIA_AUDIO],
            alias = "audioRead",
        ),
        Permission(
            strings = [Manifest.permission.READ_EXTERNAL_STORAGE],
            alias = "legacyAudioRead",
        ),
    ],
)
class PlayerPlugin(private val activity: Activity) : Plugin(activity) {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val library by lazy { MusicLibrary(activity) }
    private val trackFileDeleter by lazy { TrackFileDeleter(activity) }
    private val controller by lazy { PlaybackController(activity.applicationContext, Events()) }

    /** Найдено с начала текущего скана: сканер отдаёт треки порциями. */
    private val scannedSoFar = AtomicLong(0L)

    override fun load(webView: WebView) {
        super.load(webView)
        controller.connect()
    }

    /** Активность могли пересоздать — `connect()` идемпотентен и просто ничего не делает. */
    override fun onResume() {
        super.onResume()
        controller.connect()
    }

    override fun onDestroy(activity: AppCompatActivity) {
        super.onDestroy(activity)
        controller.release()
        scope.cancel()
    }

    // ── Плеер ───────────────────────────────────────────────────────────────

    @Command
    fun getState(invoke: Invoke) {
        invoke.resolve(controller.getState().toJson())
    }

    @Command
    fun getQueueIds(invoke: Invoke) {
        controller.getQueueIds { ids ->
            invoke.resolve(JSObject().put("trackIds", jsArrayOfLongs(ids)))
        }
    }

    @Command
    fun setQueue(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        controller.setQueue(
            items = args.queueItems("items"),
            startIndex = args.intOrNull("startIndex") ?: 0,
            autoplay = args.booleanOrNull("autoplay") ?: false,
        )
        invoke.resolve()
    }

    @Command
    fun play(invoke: Invoke) {
        controller.play()
        invoke.resolve()
    }

    @Command
    fun pause(invoke: Invoke) {
        controller.pause()
        invoke.resolve()
    }

    @Command
    fun toggle(invoke: Invoke) {
        controller.toggle()
        invoke.resolve()
    }

    @Command
    fun stop(invoke: Invoke) {
        controller.stop()
        invoke.resolve()
    }

    @Command
    fun next(invoke: Invoke) {
        controller.next()
        invoke.resolve()
    }

    @Command
    fun previous(invoke: Invoke) {
        controller.previous()
        invoke.resolve()
    }

    @Command
    fun seek(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val positionMs = args.longOrNull("positionMs") ?: return invoke.reject("seek: positionMs is required")
        controller.seek(positionMs)
        invoke.resolve()
    }

    @Command
    fun skipTo(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val index = args.intOrNull("index") ?: return invoke.reject("skipTo: index is required")
        controller.skipTo(index)
        invoke.resolve()
    }

    @Command
    fun setShuffle(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val enabled = args.booleanOrNull("enabled") ?: return invoke.reject("setShuffle: enabled is required")
        controller.setShuffle(enabled)
        invoke.resolve()
    }

    @Command
    fun setRepeat(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val raw = args.stringOrNull("mode")
        val mode = RepeatMode.fromToken(raw) ?: return invoke.reject("setRepeat: unknown mode ${raw ?: "null"}")
        controller.setRepeat(mode)
        invoke.resolve()
    }

    @Command
    fun setVolume(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val volume = args.floatOrNull("volume") ?: return invoke.reject("setVolume: volume is required")
        controller.setVolume(volume)
        invoke.resolve()
    }

    @Command
    fun setCrossfade(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val durationMs = args.longOrNull("durationMs")
            ?: return invoke.reject("setCrossfade: durationMs is required")
        controller.setCrossfade(durationMs)
        invoke.resolve()
    }

    @Command
    fun setSpeed(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val speed = args.floatOrNull("speed") ?: return invoke.reject("setSpeed: speed is required")
        controller.setSpeed(speed)
        invoke.resolve()
    }

    @Command
    fun addNext(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        controller.addNext(args.queueItems("items"))
        invoke.resolve()
    }

    @Command
    fun addToQueue(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        controller.addToQueue(args.queueItems("items"))
        invoke.resolve()
    }

    @Command
    fun removeQueueItem(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val index = args.intOrNull("index") ?: return invoke.reject("removeQueueItem: index is required")
        controller.removeQueueItem(index)
        invoke.resolve()
    }

    @Command
    fun moveQueueItem(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val from = args.intOrNull("from") ?: return invoke.reject("moveQueueItem: from is required")
        val to = args.intOrNull("to") ?: return invoke.reject("moveQueueItem: to is required")
        controller.moveQueueItem(from, to)
        invoke.resolve()
    }

    @Command
    fun clearQueue(invoke: Invoke) {
        controller.clearQueue()
        invoke.resolve()
    }

    // ── Библиотека ──────────────────────────────────────────────────────────

    /** На 33+ доступ к аудио даёт `READ_MEDIA_AUDIO`, ниже — `READ_EXTERNAL_STORAGE`. */
    private val audioReadPermission: String
        get() = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            Manifest.permission.READ_MEDIA_AUDIO
        } else {
            Manifest.permission.READ_EXTERNAL_STORAGE
        }

    private val audioReadAlias: String
        get() = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            "audioRead"
        } else {
            "legacyAudioRead"
        }

    private fun hasAudioReadPermission(): Boolean =
        ContextCompat.checkSelfPermission(activity, audioReadPermission) ==
            PackageManager.PERMISSION_GRANTED

    @Command
    fun scanMediaStore(invoke: Invoke) {
        if (!hasAudioReadPermission()) {
            requestPermissionForAlias(audioReadAlias, invoke, "onAudioReadGranted")
            return
        }
        scanMediaStoreGranted(invoke)
    }

    /**
     * Диалог закрыт. Отказ важно отличить от пустой библиотеки: молчаливый
     * «0 треков» пользователю не объяснить, поэтому наверх уходит ошибка.
     */
    @PermissionCallback
    fun onAudioReadGranted(invoke: Invoke) {
        if (!hasAudioReadPermission()) {
            invoke.reject("scanMediaStore: permission to read audio was denied")
            return
        }
        scanMediaStoreGranted(invoke)
    }

    private fun scanMediaStoreGranted(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val since = args.longOrNull("since")
        runScan(invoke) { library.scanMediaStore(since) }
    }

    @Command
    fun scanTree(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val treeUri = args.stringOrNull("treeUri") ?: return invoke.reject("scanTree: treeUri is required")
        val since = args.longOrNull("since")
        runScan(invoke) { library.scanTree(treeUri, since) }
    }

    @Command
    fun pickFolder(invoke: Invoke) {
        val intent = try {
            library.openDocumentTreeIntent()
        } catch (error: Exception) {
            return invoke.reject(error.message ?: "cannot open the folder picker")
        }
        startActivityForResult(invoke, intent, "onFolderPicked")
    }

    @ActivityCallback
    fun onFolderPicked(invoke: Invoke, result: ActivityResult) {
        val data = result.data
        val uri = data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) {
            invoke.resolve()
            return
        }
        scope.launch {
            try {
                library.takePersistableUriPermission(uri, data.flags)
                invoke.resolveObject(uri.toString())
            } catch (error: Exception) {
                invoke.reject(error.message ?: "cannot persist access to the picked folder")
            }
        }
    }

    @Command
    fun persistedRoots(invoke: Invoke) {
        scope.launch {
            try {
                invoke.resolveObject(library.persistedRoots())
            } catch (error: Exception) {
                invoke.reject(error.message ?: "cannot read persisted roots")
            }
        }
    }

    @Command
    fun releaseRoot(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val treeUri = args.stringOrNull("treeUri") ?: return invoke.reject("releaseRoot: treeUri is required")
        scope.launch {
            try {
                library.releaseRoot(treeUri)
                invoke.resolve()
            } catch (error: Exception) {
                invoke.reject(error.message ?: "cannot release the root")
            }
        }
    }

    @Command
    fun extractArtwork(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val uri = args.stringOrNull("uri") ?: return invoke.reject("extractArtwork: uri is required")
        scope.launch {
            try {
                val artwork = library.extractArtwork(uri)
                if (artwork == null) invoke.resolve() else invoke.resolveObject(artwork)
            } catch (error: Exception) {
                invoke.reject(error.message ?: "cannot extract the artwork")
            }
        }
    }

    @Command
    fun deleteTrackFile(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val uri = args.stringOrNull("uri") ?: return invoke.reject("deleteTrackFile: uri is required")
        if (needsLegacyMediaStoreWritePermission(Build.VERSION.SDK_INT, uri) &&
            ContextCompat.checkSelfPermission(
                activity,
                Manifest.permission.WRITE_EXTERNAL_STORAGE,
            ) != PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissionForAlias(
                "legacyStorageWrite",
                invoke,
                "onLegacyStorageWriteGranted",
            )
            return
        }
        deleteTrackFileGranted(invoke, uri)
    }

    @PermissionCallback
    fun onLegacyStorageWriteGranted(invoke: Invoke) {
        deleteTrackFile(invoke)
    }

    private fun deleteTrackFileGranted(invoke: Invoke, uri: String) {
        scope.launch {
            try {
                when (val action = trackFileDeleter.delete(uri)) {
                    DeleteAction.Deleted -> invoke.resolveDeleteStatus(DeleteStatus.DELETED)
                    is DeleteAction.Confirm -> withContext(Dispatchers.Main) {
                        startIntentSenderForResult(
                            invoke,
                            action.request,
                            "onTrackFileDeleteConfirmed",
                        )
                    }
                }
            } catch (error: TrackFileException) {
                invoke.reject(error.message, error.code, error)
            } catch (error: Exception) {
                invoke.reject(error.message ?: "track file deletion failed", ERROR_DELETE_FAILED, error)
            }
        }
    }

    @ActivityCallback
    fun onTrackFileDeleteConfirmed(invoke: Invoke, result: ActivityResult) {
        invoke.resolveDeleteStatus(TrackFileDeleter.confirmationStatus(result.resultCode))
    }

    @Command
    fun trackFileExists(invoke: Invoke) {
        val args = argsOf(invoke) ?: return
        val uri = args.stringOrNull("uri") ?: return invoke.reject("trackFileExists: uri is required")
        scope.launch {
            try {
                invoke.resolve(
                    JSObject().put("exists", trackFileDeleter.exists(uri)),
                )
            } catch (error: TrackFileException) {
                invoke.reject(error.message, error.code, error)
            } catch (error: Exception) {
                invoke.reject(error.message ?: "track file existence check failed", ERROR_EXISTS_FAILED, error)
            }
        }
    }

    // ── Внутреннее ──────────────────────────────────────────────────────────

    /**
     * Обход длинный и порционный — уводим его с главного потока и отвечаем по завершении
     * порции. Найденное копится между вызовами, пока сканер не отдаст `complete`.
     */
    private fun runScan(invoke: Invoke, scan: () -> ScanBatchResult) {
        scope.launch {
            try {
                val batch = scan()
                val scanned = scannedSoFar.addAndGet(batch.tracks.size.toLong())
                if (batch.complete) {
                    scannedSoFar.set(0L)
                    emitScanProgress(scanned, scanned, PHASE_DONE)
                } else {
                    // Сколько всего файлов, заранее неизвестно: total = 0 значит «прогресс не считаем».
                    emitScanProgress(scanned, 0L, PHASE_READING)
                }
                invoke.resolve(MusicLibrary.toJson(batch))
            } catch (error: Exception) {
                scannedSoFar.set(0L)
                emitScanProgress(0L, 0L, PHASE_IDLE)
                invoke.reject(error.message ?: "scan failed")
            }
        }
    }

    private fun emitScanProgress(scanned: Long, total: Long, phase: String) {
        trigger(
            EVENT_SCAN_PROGRESS,
            JSObject()
                .put("scanned", scanned)
                .put("total", total)
                .put("phase", phase),
        )
    }

    private fun argsOf(invoke: Invoke): JSObject? {
        val args = invoke.argsOrNull()
        if (args == null) {
            invoke.reject("${invoke.command}: an arguments object is required")
        }
        return args
    }

    private fun Invoke.resolveDeleteStatus(status: DeleteStatus) {
        resolve(JSObject().put("status", status.token))
    }

    private inner class Events : PlaybackController.Listener {

        override fun onState(state: PlaybackState) {
            trigger(EVENT_STATE, state.toJson())
        }

        override fun onTrackChanged(trackId: Long?, index: Int) {
            trigger(
                EVENT_TRACK_CHANGED,
                JSObject()
                    .putOrNull("trackId", trackId)
                    .put("index", index),
            )
        }

        override fun onQueueChanged(trackIds: List<Long>) {
            trigger(EVENT_QUEUE_CHANGED, JSObject().put("trackIds", jsArrayOfLongs(trackIds)))
        }

        override fun onCompleted(trackId: Long, durationPlayedMs: Long) {
            trigger(
                EVENT_COMPLETED,
                JSObject()
                    .put("trackId", trackId)
                    .put("durationPlayedMs", durationPlayedMs),
            )
        }

        override fun onError(code: String, message: String) {
            trigger(
                EVENT_ERROR,
                JSObject()
                    .put("code", code)
                    .put("message", message),
            )
        }
    }
}
