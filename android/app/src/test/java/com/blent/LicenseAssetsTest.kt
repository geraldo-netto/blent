package com.blent

import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class LicenseAssetsTest {
    private fun notice(name: String): String =
        RuntimeEnvironment.getApplication().assets.open("licenses/$name")
            .bufferedReader().use { it.readText() }

    @Test fun t845_standaloneAppCarriesBothCopyrightNoticesAndMitTerms() {
        val license = notice("LICENSE")
        assertTrue(license.contains("Copyright (c) 2026 majmichu1"))
        assertTrue(license.contains("Permission is hereby granted"))
        assertTrue(license.contains("THE SOFTWARE IS PROVIDED \"AS IS\""))
        val copyright = notice("COPYRIGHT.md")
        assertTrue(copyright.contains("Copyright (c) 2026 Geraldo Netto"))
        assertTrue(copyright.contains("96fce89c2b3e57e092d59a4890235aec9ae777c3"))
    }

    @Test fun t845_runtimeLicensesAndPublicSuffixSourceNoticeTravelWithApp() {
        assertTrue(notice("Apache-2.0.txt").contains("END OF TERMS AND CONDITIONS"))
        assertTrue(notice("MPL-2.0.txt").contains("Mozilla Public License Version 2.0"))
        assertTrue(notice("publicsuffix-NOTICE.txt").contains("https://publicsuffix.org/list/public_suffix_list.dat"))
        val index = notice("README.txt")
        assertTrue(index.contains("Apache-2.0.txt"))
        assertTrue(index.contains("MPL-2.0.txt"))
        assertTrue(index.contains("runtime-dependencies.json"))
    }
}
