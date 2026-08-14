package com.xima.music.player.library

import android.content.Context
import android.net.Uri
import android.provider.DocumentsContract
import android.util.Log

/**
 * Обход папки, выбранной пользователем через `ACTION_OPEN_DOCUMENT_TREE`.
 *
 * Дерево обходится итеративно, курсором по `buildChildDocumentsUriUsingTree`:
 * `DocumentFile.listFiles()` на каждый файл делает отдельный запрос к провайдеру
 * и на паре тысяч треков превращается в минуты.
 */
internal class SafScanner(
    private val context: Context,
    private val tagReader: TagReader,
    private val artwork: ArtworkCache,
) {

    /** Позиция обхода между вызовами `scanTree`: дерево не помещается в один ответ. */
    class Session(val treeUri: String, val since: Long?) {
        internal val root: Uri = Uri.parse(treeUri)
        internal val directories = ArrayDeque<Node>()
        internal val files = ArrayDeque<Entry>()
        internal var started = false
        internal var seen = 0L
    }

    /**
     * Каталог, до которого обход ещё не дошёл. [offset] — строка курсора, с
     * которой продолжается чтение: каталог на тысячи файлов вычитывается
     * частями, а не целиком в память.
     */
    internal class Node(val documentId: String, val folder: String?, val offset: Int = 0)

    /** Файл, найденный в каталоге, но ещё не прочитанный. */
    internal class Entry(
        val documentId: String,
        val displayName: String,
        val mimeType: String?,
        val size: Long,
        val lastModified: Long,
        val folder: String?,
    )

    fun newSession(treeUri: String, since: Long?): Session = Session(treeUri, since)

    fun next(session: Session): ScanBatchResult {
        if (!session.started) {
            session.started = true
            val rootId = try {
                DocumentsContract.getTreeDocumentId(session.root)
            } catch (error: Exception) {
                Log.w(TAG, "не tree URI: ${session.treeUri}", error)
                null
            }
            if (rootId == null) {
                return ScanBatchResult(emptyList(), complete = true, nextCursor = null)
            }
            session.directories.addLast(Node(rootId, folderOf(rootId, null, null)))
        }

        val tracks = ArrayList<ScannedTrackData>(SCAN_BATCH_SIZE)
        var visited = 0
        while (tracks.size < SCAN_BATCH_SIZE) {
            if (session.files.isNotEmpty()) {
                val entry = session.files.removeFirst()
                session.seen += 1
                val track = read(session, entry)
                if (track != null) tracks.add(track)
                continue
            }
            // Дерево из одних пустых папок иначе съело бы вызов целиком.
            if (session.directories.isEmpty() || visited >= MAX_DIRECTORIES_PER_BATCH) break
            visited += 1
            listChildren(session, session.directories.removeLast())
        }

        val complete = session.files.isEmpty() && session.directories.isEmpty()
        return ScanBatchResult(
            tracks = tracks,
            complete = complete,
            nextCursor = if (complete) null else "saf:${session.treeUri}#${session.seen}",
        )
    }

    private fun listChildren(session: Session, node: Node) {
        val children = try {
            DocumentsContract.buildChildDocumentsUriUsingTree(session.root, node.documentId)
        } catch (error: Exception) {
            Log.w(TAG, "не удалось построить URI детей ${node.documentId}", error)
            return
        }

        val cursor = try {
            context.contentResolver.query(children, PROJECTION, null, null, null)
        } catch (error: Exception) {
            // Отозванное разрешение или отвалившийся провайдер: пропускаем ветку, не скан.
            Log.w(TAG, "каталог не читается: ${node.documentId}", error)
            null
        } ?: return

        cursor.use { rows ->
            val id = rows.getColumnIndex(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
            val name = rows.getColumnIndex(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
            val mime = rows.getColumnIndex(DocumentsContract.Document.COLUMN_MIME_TYPE)
            val size = rows.getColumnIndex(DocumentsContract.Document.COLUMN_SIZE)
            val modified = rows.getColumnIndex(DocumentsContract.Document.COLUMN_LAST_MODIFIED)

            var position = node.offset
            var queued = 0
            while (rows.moveToPosition(position)) {
                position += 1

                val documentId = rows.rawStringOrNull(id) ?: continue
                val displayName = rows.rawStringOrNull(name)
                val mimeType = rows.rawStringOrNull(mime)

                if (mimeType == DocumentsContract.Document.MIME_TYPE_DIR) {
                    session.directories.addLast(
                        Node(documentId, folderOf(documentId, node.folder, displayName)),
                    )
                    continue
                }
                if (!isAudioDocument(mimeType, displayName)) continue

                val lastModified = rows.longOrNull(modified)?.coerceAtLeast(0L) ?: 0L
                val since = session.since
                // Инкрементальный скан: файл не трогали с прошлого раза — читать нечего.
                if (since != null && lastModified > 0L && lastModified <= since) continue

                session.files.addLast(
                    Entry(
                        documentId = documentId,
                        displayName = displayName ?: fileNameOf(documentId),
                        mimeType = mimeType,
                        size = rows.longOrNull(size)?.coerceAtLeast(0L) ?: 0L,
                        lastModified = lastModified,
                        folder = node.folder,
                    ),
                )

                queued += 1
                if (queued >= MAX_ENTRIES_PER_DIRECTORY) {
                    // Хвост каталога дочитаем следующим заходом, с той же строки.
                    session.directories.addLast(Node(node.documentId, node.folder, position))
                    break
                }
            }
        }
    }

    private fun read(session: Session, entry: Entry): ScannedTrackData? {
        val documentUri = try {
            DocumentsContract.buildDocumentUriUsingTree(session.root, entry.documentId)
        } catch (error: Exception) {
            Log.w(TAG, "не удалось построить URI документа ${entry.documentId}", error)
            return null
        }

        return try {
            build(entry, documentUri)
        } catch (error: Exception) {
            Log.w(TAG, "трек пропущен: ${entry.displayName}", error)
            null
        }
    }

    private fun build(entry: Entry, documentUri: Uri): ScannedTrackData {
        val uri = documentUri.toString()
        val extension = fileExtension(entry.displayName)
        val tags = tagReader.read(documentUri)

        val album = tags?.album
        val artist = tags?.artist
        val albumArtist = tags?.albumArtist
        val durationMs = tags?.durationMs?.coerceAtLeast(0L) ?: 0L
        val coverKey = artwork.ensure(
            key = artwork.keyFor(album, albumArtist, artist, uri),
            source = documentUri,
            albumId = null,
        )

        return ScannedTrackData(
            uri = uri,
            title = tags?.title ?: fileBaseName(entry.displayName),
            artist = artist,
            album = album,
            albumArtist = albumArtist,
            durationMs = durationMs,
            trackNumber = tags?.trackNumber,
            discNumber = tags?.discNumber,
            year = tags?.year?.takeIf { it > 0 },
            genre = tags?.genre,
            bitrate = tags?.bitrate ?: averageBitrate(entry.size, durationMs),
            sampleRate = tags?.sampleRate,
            size = entry.size,
            mimeType = mimeType(entry.mimeType, tags?.mimeType, extension),
            folder = entry.folder,
            // У документа нет «когда добавлен»: единственное время файла — mtime.
            dateAdded = entry.lastModified,
            lastModified = entry.lastModified,
            coverKey = coverKey,
        )
    }

    /** Провайдеры любят `application/octet-stream`; домен по нему формат не определит. */
    private fun mimeType(document: String?, fromTags: String?, extension: String): String? {
        val declared = normalizeText(document)?.lowercase()
        if (declared != null && declared.startsWith(AUDIO_MIME_PREFIX)) return declared

        val tagged = normalizeText(fromTags)?.lowercase()
        if (tagged != null && tagged.startsWith(AUDIO_MIME_PREFIX)) return tagged

        return mimeFromExtension(extension) ?: declared ?: tagged
    }

    /**
     * Человекочитаемый путь папки. У `ExternalStorageProvider` document id — это
     * `primary:Music/Rock`, из него путь берётся как есть; у провайдеров с
     * непрозрачными id он собирается из имён каталогов по дороге вниз.
     */
    private fun folderOf(documentId: String, parent: String?, displayName: String?): String? {
        val tail = documentId.substringAfter(':', "")
        if (tail.isNotEmpty() && !tail.all(Char::isDigit)) {
            displayFolderPath(tail)?.let { return it }
        }

        val name = normalizeText(displayName) ?: return parent
        return if (parent.isNullOrEmpty()) name else "$parent/$name"
    }

    private fun fileNameOf(documentId: String): String {
        val tail = documentId.substringAfterLast('/')
        return tail.substringAfterLast(':').takeIf { it.isNotEmpty() } ?: documentId
    }

    private companion object {
        const val TAG = "MusicLibrary/Saf"
        const val MAX_DIRECTORIES_PER_BATCH = 256
        const val MAX_ENTRIES_PER_DIRECTORY = 2 * SCAN_BATCH_SIZE

        val PROJECTION = arrayOf(
            DocumentsContract.Document.COLUMN_DOCUMENT_ID,
            DocumentsContract.Document.COLUMN_DISPLAY_NAME,
            DocumentsContract.Document.COLUMN_MIME_TYPE,
            DocumentsContract.Document.COLUMN_SIZE,
            DocumentsContract.Document.COLUMN_LAST_MODIFIED,
        )
    }
}
