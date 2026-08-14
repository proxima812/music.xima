package com.xima.music.player.library

import android.content.ContentUris
import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.os.Build
import android.provider.MediaStore
import android.util.Log
import android.util.Size
import java.io.File
import java.io.FileOutputStream
import java.security.MessageDigest
import kotlin.math.roundToInt

/**
 * Кэш обложек в `context.cacheDir/artwork`.
 *
 * `coverKey` — это имя файла внутри каталога: Rust-сторона резолвит его в
 * `file://`/asset-URL сама (`infrastructure/android/mod.rs`), поэтому наружу
 * уходит именно имя, а не путь.
 *
 * Ключ считается от альбома, а не от трека: одна обложка на альбом — это
 * десятки лишних декодирований, которых не будет.
 */
internal class ArtworkCache(private val context: Context) {

    private val directory: File by lazy {
        File(context.cacheDir, ARTWORK_DIR).also { if (!it.isDirectory) it.mkdirs() }
    }

    /** Ключи, для которых обложки нет: иначе каждый трек альбома снова открывает файл. */
    private val misses = LinkedHashSet<String>()

    fun keyFor(album: String?, albumArtist: String?, artist: String?, uri: String): String {
        val albumName = normalizeText(album)
        val owner = normalizeText(albumArtist) ?: normalizeText(artist)
        val seed = if (albumName != null) {
            "album\u0000${albumName.lowercase()}\u0000${owner?.lowercase().orEmpty()}"
        } else {
            "track\u0000$uri"
        }
        return "${sha1(seed)}.jpg"
    }

    /**
     * Кладёт обложку [source] в кэш под именем [key] и возвращает это имя.
     * Уже лежащий файл не пересоздаётся. `null` — обложки нет.
     */
    fun ensure(key: String, source: Uri, albumId: Long?): String? {
        val file = cachedFile(key) ?: return null
        if (file.isFile && file.length() > 0L) return key
        synchronized(misses) { if (misses.contains(key)) return null }

        val bitmap = decode(source, albumId)
        if (bitmap == null) {
            remember(key)
            return null
        }

        val stored = try {
            store(bitmap, file)
        } finally {
            bitmap.recycle()
        }
        if (!stored) {
            remember(key)
            return null
        }
        return key
    }

    /**
     * `extractArtwork` из §7.2: на вход либо `coverKey`, либо URI трека,
     * на выходе — `file://` на файл кэша, который умеет читать плеер.
     */
    fun extract(reference: String): String? {
        val value = reference.trim()
        if (value.isEmpty()) return null

        val uri = try {
            Uri.parse(value)
        } catch (error: Exception) {
            Log.w(TAG, "не URI и не coverKey: $value", error)
            return null
        }

        val scheme = uri.scheme
        if (scheme == null) return resolve(value)
        if (scheme.equals("file", ignoreCase = true)) return value

        val key = ensure(keyFor(null, null, null, value), uri, albumId = null) ?: return null
        return resolve(key)
    }

    /** `file://` на уже лежащую в кэше обложку. */
    fun resolve(key: String): String? {
        val file = cachedFile(key) ?: return null
        if (!file.isFile || file.length() <= 0L) return null
        return Uri.fromFile(file).toString()
    }

    private fun remember(key: String) {
        synchronized(misses) {
            if (misses.size >= MAX_MISSES) misses.clear()
            misses.add(key)
        }
    }

    /** Ключ — плоское имя файла; всё, что похоже на путь, до кэша не доходит. */
    private fun cachedFile(key: String): File? {
        val name = key.trim()
        if (name.isEmpty() || name.contains('/') || name.contains('\\') || name.contains("..")) {
            return null
        }
        return File(directory, name)
    }

    private fun decode(source: Uri, albumId: Long?): Bitmap? {
        val fromProvider = providerThumbnail(source, albumId)
        if (fromProvider != null) return fromProvider
        return embeddedPicture(source)
    }

    /**
     * MediaStore отдаёт миниатюру сам и делает это быстрее, чем распаковка тега:
     * с API 29 — `loadThumbnail`, ниже — устаревшая таблица album art.
     */
    private fun providerThumbnail(source: Uri, albumId: Long?): Bitmap? {
        val isMediaStore = albumId != null || source.authority == MediaStore.AUTHORITY
        if (!isMediaStore) return null

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            return try {
                context.contentResolver.loadThumbnail(
                    source,
                    Size(ARTWORK_MAX_SIZE, ARTWORK_MAX_SIZE),
                    null,
                )
            } catch (_: Exception) {
                null
            }
        }

        val id = albumId ?: return null
        val artUri = ContentUris.withAppendedId(LEGACY_ALBUM_ART, id)
        return try {
            context.contentResolver.openInputStream(artUri)?.use { stream ->
                BitmapFactory.decodeStream(stream)
            }
        } catch (_: Exception) {
            null
        }
    }

    private fun embeddedPicture(source: Uri): Bitmap? {
        val retriever = MediaMetadataRetriever()
        var bytes: ByteArray? = null
        try {
            retriever.setDataSource(context, source)
            bytes = retriever.embeddedPicture
        } catch (error: Exception) {
            Log.w(TAG, "обложка не читается: $source", error)
        } finally {
            try {
                retriever.release()
            } catch (error: Exception) {
                Log.w(TAG, "MediaMetadataRetriever не закрылся", error)
            }
        }

        val picture = bytes ?: return null
        if (picture.isEmpty()) return null
        return decodeBytes(picture)
    }

    /** Двухпроходное декодирование: сначала размеры, потом уже пиксели с `inSampleSize`. */
    private fun decodeBytes(bytes: ByteArray): Bitmap? {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)

        val options = BitmapFactory.Options().apply {
            inSampleSize = sampleSize(bounds.outWidth, bounds.outHeight)
            inPreferredConfig = Bitmap.Config.ARGB_8888
        }
        return try {
            BitmapFactory.decodeByteArray(bytes, 0, bytes.size, options)
        } catch (error: OutOfMemoryError) {
            Log.w(TAG, "обложка не влезла в память", error)
            null
        }
    }

    private fun sampleSize(width: Int, height: Int): Int {
        var longest = maxOf(width, height)
        if (longest <= 0) return 1
        var sample = 1
        while (longest / 2 >= ARTWORK_MAX_SIZE) {
            longest /= 2
            sample *= 2
        }
        return sample
    }

    private fun store(source: Bitmap, file: File): Boolean {
        if (!directory.isDirectory && !directory.mkdirs()) return false

        val scaled = scale(source)
        val temp = File(directory, "${file.name}.part")
        var written = false
        try {
            FileOutputStream(temp).use { out ->
                written = scaled.compress(Bitmap.CompressFormat.JPEG, ARTWORK_QUALITY, out)
                out.flush()
            }
        } catch (error: Exception) {
            Log.w(TAG, "не удалось записать обложку ${file.name}", error)
            written = false
        } finally {
            if (scaled !== source) scaled.recycle()
        }

        if (!written) {
            temp.delete()
            return false
        }
        if (file.exists()) file.delete()
        if (temp.renameTo(file)) return true

        temp.delete()
        return false
    }

    private fun scale(bitmap: Bitmap): Bitmap {
        val longest = maxOf(bitmap.width, bitmap.height)
        if (longest <= ARTWORK_MAX_SIZE || longest <= 0) return bitmap

        val ratio = ARTWORK_MAX_SIZE.toFloat() / longest
        val width = (bitmap.width * ratio).roundToInt().coerceAtLeast(1)
        val height = (bitmap.height * ratio).roundToInt().coerceAtLeast(1)
        return try {
            Bitmap.createScaledBitmap(bitmap, width, height, true)
        } catch (error: OutOfMemoryError) {
            Log.w(TAG, "обложку не удалось уменьшить", error)
            bitmap
        }
    }

    private fun sha1(value: String): String {
        val digest = MessageDigest.getInstance("SHA-1").digest(value.toByteArray(Charsets.UTF_8))
        val hex = StringBuilder(digest.size * 2)
        for (byte in digest) {
            val unsigned = byte.toInt() and 0xFF
            hex.append(HEX[unsigned ushr 4])
            hex.append(HEX[unsigned and 0x0F])
        }
        return hex.toString()
    }

    private companion object {
        const val TAG = "MusicLibrary/Artwork"
        const val MAX_MISSES = 4096
        val HEX = "0123456789abcdef".toCharArray()

        /** До API 29 обложку альбома отдаёт только эта таблица; публичной константы у неё нет. */
        val LEGACY_ALBUM_ART: Uri = Uri.parse("content://media/external/audio/albumart")
    }
}
