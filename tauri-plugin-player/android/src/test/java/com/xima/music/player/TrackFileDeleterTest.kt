package com.xima.music.player

import android.app.Activity
import android.os.Build
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
    fun mediaStoreCollectionAndNonAudioUrisAreUnsupported() {
        for (uri in listOf(
            "content://media/external/audio/media",
            "content://media/external/images/media/42",
            "content://media/external/audio/media/0",
            "content://media/external/audio/media/not-a-number",
            "content://media/external/audio/media/42/extra",
        )) {
            assertEquals(
                "unsafe MediaStore URI must be rejected: $uri",
                DeleteTarget.UNSUPPORTED,
                TrackFileDeleter.classifyTarget(
                    uri = uri,
                    isDocumentUri = false,
                    documentFlags = null,
                ),
            )
        }
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

    @Test
    fun legacyWritePermissionIsOnlyForConcreteMediaStoreItemsThroughAndroid28() {
        val mediaItem = "content://media/external/audio/media/42"
        assertEquals(
            true,
            needsLegacyMediaStoreWritePermission(Build.VERSION_CODES.P, mediaItem),
        )
        assertEquals(
            false,
            needsLegacyMediaStoreWritePermission(Build.VERSION_CODES.Q, mediaItem),
        )
        for (uri in listOf(
            "content://media/external/audio/media",
            "content://com.android.externalstorage.documents/document/primary%3AMusic%2Fsong.mp3",
            "file:///storage/emulated/0/Music/song.mp3",
            "content://com.example.unknown/items/42",
        )) {
            assertEquals(
                "must not request broad storage permission for $uri",
                false,
                needsLegacyMediaStoreWritePermission(Build.VERSION_CODES.P, uri),
            )
        }
    }
}
