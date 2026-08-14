package com.xima.music.player

import android.content.ComponentName
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import androidx.core.content.ContextCompat
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.Timeline
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import com.google.common.util.concurrent.ListenableFuture

/**
 * Тонкая обёртка над `MediaController` — зеркало команд из docs/CONTRACTS.md §7.2.
 *
 * Плеер живёт в [PlaybackService], поэтому здесь только маршалинг: любое обращение
 * к нему уходит на главный поток, а всё, что плеер сообщает в ответ, превращается
 * в события контракта. Команды, пришедшие до подключения сессии, копятся и
 * выполняются, как только контроллер готов.
 */
internal class PlaybackController(
    private val context: Context,
    private val listener: Listener,
) {

    internal interface Listener {
        fun onState(state: PlaybackState)
        fun onTrackChanged(trackId: Long?, index: Int)
        fun onQueueChanged(trackIds: List<Long>)
        fun onCompleted(trackId: Long, durationPlayedMs: Long)
        fun onError(code: String, message: String)
    }

    private val main = Handler(Looper.getMainLooper())
    private val playerEvents = PlayerEvents()
    private val pending = ArrayDeque<(Player) -> Unit>()

    private var connection: ListenableFuture<MediaController>? = null
    private var controller: MediaController? = null
    private var queueIds: List<Long> = emptyList()
    private var ticking = false

    /** Сколько реально отзвучало у текущего трека — по «стенным» часам, а не по позиции. */
    private var listenedTrackId: Long? = null
    private var listenedMs = 0L
    private var listeningSince: Long? = null

    @Volatile
    private var lastState: PlaybackState = PlaybackState.idle()

    private val tick = object : Runnable {
        override fun run() {
            val player = controller
            if (player == null) {
                ticking = false
                return
            }
            publishState(player)
            if (player.isPlaying) {
                main.postDelayed(this, TICK_MS)
            } else {
                ticking = false
            }
        }
    }

    fun connect() = onMain {
        if (connection != null) return@onMain
        val future = try {
            val token = SessionToken(context, ComponentName(context, SERVICE_CLASS))
            MediaController.Builder(context, token).buildAsync()
        } catch (error: Exception) {
            listener.onError(ERROR_CONNECTION, error.message ?: "media session is unavailable")
            return@onMain
        }
        connection = future
        future.addListener({ attach(future) }, ContextCompat.getMainExecutor(context))
    }

    fun release() = onMain {
        main.removeCallbacks(tick)
        ticking = false
        pending.clear()
        controller?.removeListener(playerEvents)
        controller = null
        connection?.let { MediaController.releaseFuture(it) }
        connection = null
    }

    /**
     * Состояние отдаётся синхронно: команды плагина уже выполняются на главном потоке,
     * поэтому плеер можно спросить напрямую. До подключения возвращаем последний снимок.
     */
    fun getState(): PlaybackState {
        val player = controller
        if (player != null && Looper.myLooper() == Looper.getMainLooper()) {
            val state = snapshot(player)
            lastState = state
            return state
        }
        return lastState
    }

    /** Resolves only after MediaController attaches, so startup sees the service queue. */
    fun getQueueIds(resolve: (List<Long>) -> Unit) = withPlayer { player ->
        val ids = QueueMapper.trackIds(player)
        queueIds = ids
        resolve(ids)
    }

    fun setQueue(items: List<QueueItem>, startIndex: Int, autoplay: Boolean) = withPlayer { player ->
        flushListening()
        if (items.isEmpty()) {
            player.clearMediaItems()
            player.stop()
            return@withPlayer
        }
        val mediaItems = items.map { QueueMapper.toMediaItem(it) }
        player.setMediaItems(mediaItems, startIndex.coerceIn(0, mediaItems.size - 1), C.TIME_UNSET)
        player.prepare()
        if (autoplay) player.play() else player.pause()
    }

    fun play() = withPlayer { player ->
        if (player.playbackState == Player.STATE_IDLE) {
            player.prepare()
        }
        if (player.playbackState == Player.STATE_ENDED && player.mediaItemCount > 0) {
            player.seekTo(player.currentMediaItemIndex, 0L)
        }
        player.play()
    }

    fun pause() = withPlayer { player -> player.pause() }

    fun toggle() = withPlayer { player ->
        if (player.isPlaying) {
            player.pause()
        } else {
            if (player.playbackState == Player.STATE_IDLE) {
                player.prepare()
            }
            player.play()
        }
    }

    fun stop() = withPlayer { player ->
        flushListening()
        player.stop()
        player.seekTo(0L)
    }

    fun next() = withPlayer { player -> player.seekToNext() }

    fun previous() = withPlayer { player -> player.seekToPrevious() }

    fun seek(positionMs: Long) = withPlayer { player -> player.seekTo(positionMs.coerceAtLeast(0L)) }

    fun skipTo(index: Int) = withPlayer { player ->
        if (index in 0 until player.mediaItemCount) {
            player.seekTo(index, 0L)
        }
    }

    fun setShuffle(enabled: Boolean) = withPlayer { player -> player.shuffleModeEnabled = enabled }

    fun setRepeat(mode: RepeatMode) = withPlayer { player ->
        player.repeatMode = when (mode) {
            RepeatMode.OFF -> Player.REPEAT_MODE_OFF
            RepeatMode.ALL -> Player.REPEAT_MODE_ALL
            RepeatMode.ONE -> Player.REPEAT_MODE_ONE
        }
    }

    fun setVolume(volume: Float) = withPlayer { player -> player.volume = volume.coerceIn(0f, 1f) }

    fun setSpeed(speed: Float) = withPlayer { player ->
        player.setPlaybackSpeed(speed.coerceIn(MIN_SPEED, MAX_SPEED))
    }

    fun addNext(items: List<QueueItem>) = withPlayer { player ->
        if (items.isEmpty()) return@withPlayer
        val count = player.mediaItemCount
        val target = if (count == 0) 0 else (player.currentMediaItemIndex + 1).coerceIn(0, count)
        player.addMediaItems(target, items.map { QueueMapper.toMediaItem(it) })
        if (count == 0) player.prepare()
    }

    fun addToQueue(items: List<QueueItem>) = withPlayer { player ->
        if (items.isEmpty()) return@withPlayer
        val wasEmpty = player.mediaItemCount == 0
        player.addMediaItems(items.map { QueueMapper.toMediaItem(it) })
        if (wasEmpty) player.prepare()
    }

    fun removeQueueItem(index: Int) = withPlayer { player ->
        if (index in 0 until player.mediaItemCount) {
            player.removeMediaItem(index)
        }
    }

    fun moveQueueItem(from: Int, to: Int) = withPlayer { player ->
        val count = player.mediaItemCount
        if (from in 0 until count && to in 0 until count && from != to) {
            player.moveMediaItem(from, to)
        }
    }

    fun clearQueue() = withPlayer { player ->
        flushListening()
        player.clearMediaItems()
    }

    private fun attach(future: ListenableFuture<MediaController>) {
        if (connection !== future) {
            // Плагин успели отпустить, пока сессия подключалась.
            MediaController.releaseFuture(future)
            return
        }

        val player = try {
            future.get()
        } catch (error: Exception) {
            connection = null
            listener.onError(ERROR_CONNECTION, error.message ?: "media session is unavailable")
            return
        }

        controller = player
        player.addListener(playerEvents)
        listenedTrackId = QueueMapper.trackIdOf(player.currentMediaItem)
        listenedMs = 0L
        listeningSince = if (player.isPlaying) SystemClock.elapsedRealtime() else null

        while (pending.isNotEmpty()) {
            val action = pending.removeFirst()
            try {
                action(player)
            } catch (error: Exception) {
                listener.onError(ERROR_COMMAND, error.message ?: "queued player command failed")
            }
        }

        publishQueue(player)
        publishState(player)
        syncTicker(player)
    }

    private fun withPlayer(action: (Player) -> Unit) = onMain {
        val player = controller
        if (player != null) {
            action(player)
            return@onMain
        }
        if (pending.size >= MAX_PENDING) {
            pending.removeFirst()
        }
        pending.addLast(action)
    }

    private fun onMain(action: () -> Unit) {
        if (Looper.myLooper() == Looper.getMainLooper()) action() else main.post { action() }
    }

    private fun publishState(player: Player) {
        val state = snapshot(player)
        if (state == lastState) return
        lastState = state
        listener.onState(state)
    }

    private fun publishQueue(player: Player) {
        val ids = QueueMapper.trackIds(player)
        if (ids == queueIds) return
        queueIds = ids
        listener.onQueueChanged(ids)
    }

    private fun snapshot(player: Player): PlaybackState {
        val count = player.mediaItemCount
        val duration = player.duration
        return PlaybackState(
            status = statusOf(player),
            trackId = QueueMapper.trackIdOf(player.currentMediaItem),
            positionMs = player.currentPosition.coerceAtLeast(0L),
            durationMs = if (duration == C.TIME_UNSET || duration < 0L) metadataDuration(player) else duration,
            queueIndex = if (count == 0) null else player.currentMediaItemIndex.coerceIn(0, count - 1),
            queueLength = count,
            shuffle = player.shuffleModeEnabled,
            repeat = repeatOf(player.repeatMode),
            volume = player.volume,
            speed = player.playbackParameters.speed,
        )
    }

    /** Пока файл не подготовлен, длительность известна только из тегов, отданных Rust. */
    private fun metadataDuration(player: Player): Long =
        player.currentMediaItem?.mediaMetadata?.durationMs?.coerceAtLeast(0L) ?: 0L

    private fun statusOf(player: Player): PlaybackStatus = when (player.playbackState) {
        Player.STATE_BUFFERING -> PlaybackStatus.BUFFERING
        Player.STATE_READY -> if (player.isPlaying) PlaybackStatus.PLAYING else PlaybackStatus.PAUSED
        Player.STATE_ENDED -> PlaybackStatus.ENDED
        else -> PlaybackStatus.IDLE
    }

    private fun repeatOf(mode: Int): RepeatMode = when (mode) {
        Player.REPEAT_MODE_ALL -> RepeatMode.ALL
        Player.REPEAT_MODE_ONE -> RepeatMode.ONE
        else -> RepeatMode.OFF
    }

    private fun syncTicker(player: Player) {
        if (player.isPlaying) {
            if (!ticking) {
                ticking = true
                main.postDelayed(tick, TICK_MS)
            }
        } else if (ticking) {
            ticking = false
            main.removeCallbacks(tick)
        }
    }

    private fun accumulate(now: Long) {
        val since = listeningSince ?: return
        if (now > since) {
            listenedMs += now - since
        }
        listeningSince = now
    }

    /** Закрывает счётчик текущего трека и отдаёт наверх `completed`, если было что слушать. */
    private fun flushListening() {
        accumulate(SystemClock.elapsedRealtime())
        val trackId = listenedTrackId
        val played = listenedMs
        listenedMs = 0L
        listeningSince = null
        if (trackId != null && played > 0L) {
            listener.onCompleted(trackId, played)
        }
    }

    private inner class PlayerEvents : Player.Listener {

        override fun onEvents(player: Player, events: Player.Events) {
            publishState(player)
        }

        override fun onMediaItemTransition(mediaItem: MediaItem?, reason: Int) {
            val player = controller ?: return
            flushListening()
            listenedTrackId = QueueMapper.trackIdOf(mediaItem)
            listeningSince = if (player.isPlaying) SystemClock.elapsedRealtime() else null
            listener.onTrackChanged(
                listenedTrackId,
                if (mediaItem == null) NO_INDEX else player.currentMediaItemIndex,
            )
        }

        override fun onTimelineChanged(timeline: Timeline, reason: Int) {
            val player = controller ?: return
            publishQueue(player)
        }

        override fun onPlaybackStateChanged(playbackState: Int) {
            if (playbackState == Player.STATE_ENDED || playbackState == Player.STATE_IDLE) {
                flushListening()
            }
        }

        override fun onIsPlayingChanged(isPlaying: Boolean) {
            val now = SystemClock.elapsedRealtime()
            if (isPlaying) {
                listeningSince = now
            } else {
                accumulate(now)
                listeningSince = null
            }
            controller?.let { syncTicker(it) }
        }

        override fun onPlayerError(error: PlaybackException) {
            listener.onError(error.errorCodeName, error.message ?: error.errorCodeName)
        }
    }

    private companion object {
        /**
         * Имя сервиса, а не `PlaybackService::class.java`: класс помечен `@UnstableApi`,
         * и ссылка на него потребовала бы opt-in во всей цепочке вызовов.
         * Строка зафиксирована в AndroidManifest.xml (`.PlaybackService`).
         */
        const val SERVICE_CLASS = "com.xima.music.player.PlaybackService"

        const val TICK_MS = 500L
        const val MAX_PENDING = 64
        const val MIN_SPEED = 0.25f
        const val MAX_SPEED = 4f
        const val NO_INDEX = -1
        const val ERROR_CONNECTION = "SESSION_UNAVAILABLE"
        const val ERROR_COMMAND = "PLAYER_COMMAND_FAILED"
    }
}
