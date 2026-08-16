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

    /**
     * Плавный переход между треками (`crossfadeMs` из настроек, 0 — выключено).
     *
     * Наложения звука нет: у сессии один ExoPlayer. Хвост трека уходит в тишину,
     * следующий из неё же и поднимается — стык перестаёт быть резким.
     */
    private var crossfadeMs = 0L

    /** Громкость, которую задал пользователь. Фейд множится на неё, а не затирает. */
    private var masterVolume = 1f
    private var fadeGain = 1f
    private var fadingOut = false
    private var fadeStep: Runnable? = null

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
            maybeFadeOut(player)
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
        cancelRamp()
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
        resetFade()
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
        resetFade()
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
            resetFade()
            player.play()
        }
    }

    fun stop() = withPlayer { player ->
        flushListening()
        resetFade()
        player.stop()
        player.seekTo(0L)
    }

    fun next() = withPlayer { player ->
        resetFade()
        player.seekToNext()
    }

    fun previous() = withPlayer { player ->
        resetFade()
        player.seekToPrevious()
    }

    fun seek(positionMs: Long) = withPlayer { player ->
        resetFade()
        player.seekTo(positionMs.coerceAtLeast(0L))
    }

    fun skipTo(index: Int) = withPlayer { player ->
        if (index in 0 until player.mediaItemCount) {
            resetFade()
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

    fun setVolume(volume: Float) = withPlayer { _ ->
        masterVolume = volume.coerceIn(0f, 1f)
        applyGain()
    }

    /** Длительность плавного перехода; 0 выключает его и возвращает громкость на место. */
    fun setCrossfade(durationMs: Long) = withPlayer { _ ->
        crossfadeMs = durationMs.coerceIn(0L, MAX_CROSSFADE_MS)
        if (crossfadeMs == 0L) resetFade()
    }

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
        // Сессия могла пережить перезапуск WebView — забираем её громкость как master.
        masterVolume = player.volume
        fadeGain = 1f
        fadingOut = false
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
            // Во время фейда реальная громкость плеера ниже — наружу отдаём выбранную.
            volume = masterVolume,
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

    // ── Плавный переход ─────────────────────────────────────────────────────

    /**
     * Хвост трека уводится в тишину за `crossfadeMs` до конца. Считается по тикеру
     * (500 мс) — этого хватает, чтобы начать двухсекундный спуск вовремя.
     *
     * Не гасим: слишком короткий трек (меньше двух окон перехода) и «повтор одного»,
     * где стыка между разными треками просто нет.
     */
    private fun maybeFadeOut(player: Player) {
        val fade = crossfadeMs
        if (fade <= 0L || !player.isPlaying) return
        if (player.repeatMode == Player.REPEAT_MODE_ONE) return

        val duration = player.duration
        if (duration == C.TIME_UNSET || duration <= 0L) return

        val remaining = duration - player.currentPosition
        if (remaining <= 0L || remaining > fade) {
            // Перемотали из хвоста назад — возвращаем громкость.
            if (fadingOut && remaining > fade) resetFade()
            return
        }

        if (fadingOut || duration < fade * 2) return
        fadingOut = true
        // Спуск идёт по часам, а музыка — со своей скоростью: на 2× хвост
        // отзвучит вдвое быстрее, и затухание должно успеть за ним.
        val speed = player.playbackParameters.speed
        val wallClock = if (speed > 0f) (remaining / speed).toLong() else remaining
        rampTo(0f, wallClock)
    }

    /**
     * Новый трек начинается с тишины и поднимается до выбранной громкости.
     *
     * Ручное переключение поднимается быстро: нажали «дальше» — звук должен
     * появиться сразу, а не через пару секунд. Плавно нарастает только тот трек,
     * что пришёл сам, следом за затухшим хвостом предыдущего.
     */
    private fun fadeIn(durationMs: Long) {
        if (crossfadeMs <= 0L || durationMs <= 0L) {
            resetFade()
            return
        }
        cancelRamp()
        fadingOut = false
        fadeGain = 0f
        applyGain()
        rampTo(1f, durationMs)
    }

    private fun resetFade() {
        cancelRamp()
        fadingOut = false
        fadeGain = 1f
        applyGain()
    }

    private fun rampTo(target: Float, durationMs: Long) {
        cancelRamp()
        if (durationMs <= 0L) {
            fadeGain = target
            applyGain()
            return
        }

        val from = fadeGain
        val startedAt = SystemClock.elapsedRealtime()
        val step = object : Runnable {
            override fun run() {
                val elapsed = SystemClock.elapsedRealtime() - startedAt
                val progress = (elapsed.toFloat() / durationMs.toFloat()).coerceIn(0f, 1f)
                fadeGain = from + (target - from) * progress
                applyGain()
                if (progress < 1f) {
                    main.postDelayed(this, FADE_STEP_MS)
                } else {
                    fadeStep = null
                }
            }
        }
        fadeStep = step
        main.post(step)
    }

    private fun cancelRamp() {
        fadeStep?.let { main.removeCallbacks(it) }
        fadeStep = null
    }

    private fun applyGain() {
        controller?.volume = (masterVolume * fadeGain).coerceIn(0f, 1f)
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
            when (reason) {
                Player.MEDIA_ITEM_TRANSITION_REASON_REPEAT -> resetFade()
                Player.MEDIA_ITEM_TRANSITION_REASON_AUTO -> fadeIn(crossfadeMs)
                else -> fadeIn(minOf(crossfadeMs, MANUAL_FADE_IN_MS))
            }
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
                // Очередь кончилась на тишине — следующий запуск не должен быть немым.
                resetFade()
            }
        }

        override fun onIsPlayingChanged(isPlaying: Boolean) {
            val now = SystemClock.elapsedRealtime()
            if (isPlaying) {
                listeningSince = now
            } else {
                accumulate(now)
                listeningSince = null
                // Пауза посреди затухания: спуск идёт по часам, а не по позиции,
                // поэтому его надо снять — иначе трек вернётся из паузы немым.
                if (fadingOut) resetFade()
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
        /** Шаг громкости при фейде: 50 мс — ухо не слышит ступенек. */
        const val FADE_STEP_MS = 50L
        /** Нарастание после ручного переключения: снимает щелчок, но не тянет. */
        const val MANUAL_FADE_IN_MS = 300L
        const val MAX_CROSSFADE_MS = 12_000L
        const val MAX_PENDING = 64
        const val MIN_SPEED = 0.25f
        const val MAX_SPEED = 4f
        const val NO_INDEX = -1
        const val ERROR_CONNECTION = "SESSION_UNAVAILABLE"
        const val ERROR_COMMAND = "PLAYER_COMMAND_FAILED"
    }
}
