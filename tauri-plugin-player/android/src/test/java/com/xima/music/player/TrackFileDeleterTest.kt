package com.xima.music.player

import android.app.Activity
import android.provider.DocumentsContract
import org.junit.Assert.assertEquals
import org.junit.Test

class TrackFileDeleterTest {

    @Test
    fun mediaStoreUriIsClassifiedForMediaStoreDeletion() {
        assertEquals(
            DeleteTarget.MEDIA_STORE,
            TrackFileDeleter.classifyTarget(
                uri = "content://media/external/audio/media/42",
                isDocumentUri = false,
                documentFlags = null,
            ),
        )
    }

    @Test
    fun deletableSafDocumentIsClassifiedForSafDeletion() {
        assertEquals(
            DeleteTarget.SAF_DOCUMENT,
            TrackFileDeleter.classifyTarget(
                uri = "content://com.android.externalstorage.documents/document/primary%3AMusic%2Fsong.mp3",
                isDocumentUri = true,
                documentFlags = DocumentsContract.Document.FLAG_SUPPORTS_DELETE.toLong(),
            ),
        )
    }

    @Test
    fun safDocumentWithoutDeleteSupportIsUnsupported() {
        assertEquals(
            DeleteTarget.UNSUPPORTED,
            TrackFileDeleter.classifyTarget(
                uri = "content://com.example.documents/document/read-only-song",
                isDocumentUri = true,
                documentFlags = 0L,
            ),
        )
    }

    @Test
    fun nonContentAndUnknownProviderUrisAreUnsupported() {
        assertEquals(
            DeleteTarget.UNSUPPORTED,
            TrackFileDeleter.classifyTarget(
                uri = "file:///storage/emulated/0/Music/song.mp3",
                isDocumentUri = false,
                documentFlags = null,
            ),
        )
        assertEquals(
            DeleteTarget.UNSUPPORTED,
            TrackFileDeleter.classifyTarget(
                uri = "content://com.example.unknown/items/42",
                isDocumentUri = false,
                documentFlags = null,
            ),
        )
    }

    @Test
    fun dismissedConfirmationMapsToCancellation() {
        assertEquals(
            DeleteStatus.CANCELLED,
            TrackFileDeleter.confirmationStatus(Activity.RESULT_CANCELED),
        )
    }

    @Test
    fun existenceQueryDistinguishesPresentMissingAndProviderFailure() {
        assertEquals(
            ExistenceProbe.PRESENT,
            TrackFileDeleter.classifyExistence(cursorReturned = true, hasRow = true),
        )
        assertEquals(
            ExistenceProbe.MISSING,
            TrackFileDeleter.classifyExistence(cursorReturned = true, hasRow = false),
        )
        assertEquals(
            ExistenceProbe.PROVIDER_FAILURE,
            TrackFileDeleter.classifyExistence(cursorReturned = false, hasRow = false),
        )
    }
}
