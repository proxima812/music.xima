package com.xima.music.player

import android.net.Uri
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.Player

/**
 * `QueueItem` (JSON из Rust) ↔ `MediaItem` Media3.
 *
 * `mediaId` — это `trackId` строкой: по нему события плеера возвращаются в строки БД.
 * URI дублируется в `RequestMetadata`, потому что `MediaItem.localConfiguration`
 * не переживает сериализацию между `MediaController` и `MediaSession`.
 */
internal object QueueMapper {

    fun toMediaItem(item: QueueItem): MediaItem {
        val uri = Uri.parse(item.uri)
        val metadata = MediaMetadata.Builder()
            .setTitle(item.title)
            .setArtist(item.artist)
            .setAlbumTitle(item.album)
            .setArtworkUri(item.artworkUri?.let(Uri::parse))
            .setDurationMs(item.durationMs.takeIf { it > 0L })
            .setIsBrowsable(false)
            .setIsPlayable(true)
            .build()

        return MediaItem.Builder()
            .setMediaId(item.trackId.toString())
            .setUri(uri)
            .setRequestMetadata(MediaItem.RequestMetadata.Builder().setMediaUri(uri).build())
            .setMediaMetadata(metadata)
            .build()
    }

    fun trackIdOf(item: MediaItem?): Long? = item?.mediaId?.toLongOrNull()

    fun trackIds(player: Player): List<Long> {
        if (!player.isCommandAvailable(Player.COMMAND_GET_TIMELINE)) return emptyList()
        val count = player.mediaItemCount
        if (count == 0) return emptyList()

        val ids = ArrayList<Long>(count)
        for (index in 0 until count) {
            val id = trackIdOf(player.getMediaItemAt(index))
            if (id != null) {
                ids.add(id)
            }
        }
        return ids
    }
}
