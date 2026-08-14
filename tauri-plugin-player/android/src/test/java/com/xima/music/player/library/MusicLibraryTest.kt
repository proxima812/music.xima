package com.xima.music.player.library

import android.content.Intent
import org.junit.Assert.assertEquals
import org.junit.Test

class MusicLibraryTest {

    @Test
    fun persistedGrantUsesOnlyReturnedReadAndWriteFlags() {
        val returned = Intent.FLAG_GRANT_READ_URI_PERMISSION or
            Intent.FLAG_GRANT_WRITE_URI_PERMISSION or
            Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION or
            Intent.FLAG_GRANT_PREFIX_URI_PERMISSION

        assertEquals(
            Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
            MusicLibrary.persistableGrantFlags(returned),
        )
        assertEquals(
            Intent.FLAG_GRANT_READ_URI_PERMISSION,
            MusicLibrary.persistableGrantFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION),
        )
        assertEquals(0, MusicLibrary.persistableGrantFlags(0))
    }
}
