package com.xima.music.player

import android.app.Activity
import android.app.RecoverableSecurityException
import android.content.ContentResolver
import android.content.Context
import android.net.Uri
import android.os.Build
import android.provider.DocumentsContract
import android.provider.MediaStore
import androidx.activity.result.IntentSenderRequest
import androidx.annotation.RequiresApi
import java.io.FileNotFoundException
import java.net.URI

internal const val ERROR_UNSUPPORTED_DELETE = "UNSUPPORTED_DELETE"
internal const val ERROR_DELETE_FAILED = "TRACK_FILE_DELETE_FAILED"
internal const val ERROR_EXISTS_FAILED = "TRACK_FILE_EXISTS_FAILED"

internal enum class DeleteTarget {
    MEDIA_STORE,
    SAF_DOCUMENT,
    UNSUPPORTED,
}

internal enum class DeleteStatus(val token: String) {
    DELETED("deleted"),
    CANCELLED("cancelled"),
}

internal enum class ExistenceProbe {
    PRESENT,
    MISSING,
    PROVIDER_FAILURE,
}

internal sealed interface DeleteAction {
    data object Deleted : DeleteAction

    data class Confirm(val request: IntentSenderRequest) : DeleteAction
}

internal class TrackFileException(
    val code: String,
    message: String,
    cause: Throwable? = null,
) : Exception(message, cause)

/** Native shared-storage operations for MediaStore and SAF track URIs. */
internal class TrackFileDeleter(context: Context) {

    private val appContext = context.applicationContext
    private val resolver: ContentResolver = appContext.contentResolver

    fun delete(uriText: String): DeleteAction {
        val uri = parseContentUri(uriText)
        val isDocument = DocumentsContract.isDocumentUri(appContext, uri)
        val flags = if (isDocument) documentFlags(uri) else null

        return when (classifyTarget(uriText, isDocument, flags)) {
            DeleteTarget.MEDIA_STORE -> deleteMediaStore(uri)
            DeleteTarget.SAF_DOCUMENT -> deleteSafDocument(uri)
            DeleteTarget.UNSUPPORTED -> throw unsupported(uriText)
        }
    }

    fun exists(uriText: String): Boolean {
        val uri = parseContentUri(uriText)
        val isDocument = DocumentsContract.isDocumentUri(appContext, uri)
        val projection = when {
            isMediaStoreAudioItem(uriText) -> arrayOf(MediaStore.MediaColumns._ID)
            isDocument -> arrayOf(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
            else -> throw unsupported(uriText)
        }

        val probe = try {
            val cursor = resolver.query(uri, projection, null, null, null)
            cursor?.use { classifyExistence(cursorReturned = true, hasRow = it.moveToFirst()) }
                ?: ExistenceProbe.PROVIDER_FAILURE
        } catch (_: FileNotFoundException) {
            ExistenceProbe.MISSING
        } catch (error: SecurityException) {
            throw TrackFileException(
                ERROR_EXISTS_FAILED,
                "cannot determine whether the track file exists: provider access denied",
                error,
            )
        } catch (error: RuntimeException) {
            throw TrackFileException(
                ERROR_EXISTS_FAILED,
                "cannot determine whether the track file exists: provider query failed",
                error,
            )
        }

        return when (probe) {
            ExistenceProbe.PRESENT -> true
            ExistenceProbe.MISSING -> false
            ExistenceProbe.PROVIDER_FAILURE -> throw TrackFileException(
                ERROR_EXISTS_FAILED,
                "cannot determine whether the track file exists: provider returned no result",
            )
        }
    }

    private fun deleteMediaStore(uri: Uri): DeleteAction {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val pendingIntent = MediaStore.createDeleteRequest(resolver, listOf(uri))
            return DeleteAction.Confirm(IntentSenderRequest.Builder(pendingIntent.intentSender).build())
        }

        val deletedRows = try {
            resolver.delete(uri, null, null)
        } catch (error: SecurityException) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                return recoverableDeleteAction(error)
            }
            throw TrackFileException(ERROR_DELETE_FAILED, "MediaStore denied track-file deletion", error)
        } catch (error: RuntimeException) {
            throw TrackFileException(ERROR_DELETE_FAILED, "MediaStore could not delete the track file", error)
        }
        if (deletedRows <= 0) {
            throw TrackFileException(ERROR_DELETE_FAILED, "MediaStore did not delete the track file")
        }
        return DeleteAction.Deleted
    }

    @RequiresApi(Build.VERSION_CODES.Q)
    private fun recoverableDeleteAction(error: SecurityException): DeleteAction {
        if (error !is RecoverableSecurityException) {
            throw TrackFileException(ERROR_DELETE_FAILED, "MediaStore denied track-file deletion", error)
        }
        return DeleteAction.Confirm(
            IntentSenderRequest.Builder(error.userAction.actionIntent.intentSender).build(),
        )
    }

    private fun deleteSafDocument(uri: Uri): DeleteAction {
        val deleted = try {
            DocumentsContract.deleteDocument(resolver, uri)
        } catch (error: RuntimeException) {
            throw TrackFileException(ERROR_DELETE_FAILED, "the document provider could not delete the track file", error)
        }
        if (!deleted) {
            throw TrackFileException(ERROR_DELETE_FAILED, "the document provider did not delete the track file")
        }
        return DeleteAction.Deleted
    }

    private fun documentFlags(uri: Uri): Long {
        val cursor = try {
            resolver.query(
                uri,
                arrayOf(DocumentsContract.Document.COLUMN_FLAGS),
                null,
                null,
                null,
            )
        } catch (error: RuntimeException) {
            throw TrackFileException(ERROR_DELETE_FAILED, "cannot inspect document delete support", error)
        } ?: throw TrackFileException(ERROR_DELETE_FAILED, "document provider returned no flags")

        cursor.use {
            if (!it.moveToFirst()) {
                throw TrackFileException(ERROR_DELETE_FAILED, "track document was not found")
            }
            val index = it.getColumnIndex(DocumentsContract.Document.COLUMN_FLAGS)
            if (index < 0) {
                throw TrackFileException(ERROR_DELETE_FAILED, "document provider omitted delete flags")
            }
            return it.getLong(index)
        }
    }

    companion object {
        fun classifyTarget(
            uri: String,
            isDocumentUri: Boolean,
            documentFlags: Long?,
        ): DeleteTarget {
            val parsed = runCatching { URI(uri) }.getOrNull() ?: return DeleteTarget.UNSUPPORTED
            if (!parsed.scheme.equals(ContentResolver.SCHEME_CONTENT, ignoreCase = true)) {
                return DeleteTarget.UNSUPPORTED
            }
            if (parsed.authority.equals(MediaStore.AUTHORITY, ignoreCase = true)) {
                return if (isMediaStoreAudioItem(uri)) {
                    DeleteTarget.MEDIA_STORE
                } else {
                    DeleteTarget.UNSUPPORTED
                }
            }
            if (!isDocumentUri) return DeleteTarget.UNSUPPORTED

            val supportsDelete = documentFlags?.let {
                it and DocumentsContract.Document.FLAG_SUPPORTS_DELETE.toLong() != 0L
            } ?: false
            return if (supportsDelete) DeleteTarget.SAF_DOCUMENT else DeleteTarget.UNSUPPORTED
        }

        fun confirmationStatus(resultCode: Int): DeleteStatus =
            if (resultCode == Activity.RESULT_OK) DeleteStatus.DELETED else DeleteStatus.CANCELLED

        fun classifyExistence(cursorReturned: Boolean, hasRow: Boolean): ExistenceProbe = when {
            !cursorReturned -> ExistenceProbe.PROVIDER_FAILURE
            hasRow -> ExistenceProbe.PRESENT
            else -> ExistenceProbe.MISSING
        }

        private fun isMediaStoreAudioItem(uri: String): Boolean {
            val parsed = runCatching { URI(uri) }.getOrNull() ?: return false
            if (!parsed.scheme.equals(ContentResolver.SCHEME_CONTENT, ignoreCase = true) ||
                !parsed.authority.equals(MediaStore.AUTHORITY, ignoreCase = true)
            ) {
                return false
            }
            val segments = parsed.path.orEmpty().split('/').filter(String::isNotEmpty)
            if (segments.size != 4 || segments[1] != "audio" || segments[2] != "media") {
                return false
            }
            return segments[3].toLongOrNull()?.let { it > 0L } == true
        }

        private fun parseContentUri(uriText: String): Uri {
            val uri = Uri.parse(uriText)
            if (!uri.scheme.equals(ContentResolver.SCHEME_CONTENT, ignoreCase = true)) {
                throw unsupported(uriText)
            }
            return uri
        }

        private fun unsupported(uri: String) = TrackFileException(
            ERROR_UNSUPPORTED_DELETE,
            "track URI is not a deletable MediaStore or SAF document URI: $uri",
        )
    }
}
