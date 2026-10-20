package com.blent

import org.junit.runners.model.FrameworkMethod
import org.robolectric.RobolectricTestRunner
import org.robolectric.internal.bytecode.InstrumentationConfiguration

/** T1171: a fixture class owns its Compose platform sandbox on each SDK.
 * Native Activity teardown cannot lend cached platform dispatchers to another
 * fixture. Robolectric owns sandbox disposal and the original SDK executor.
 * Its instrumentation policy excludes @RunWith test classes; this identity key
 * does not transform application classes or filter their native counters. */
class ComposeFixtureRunner(private val fixtureClass: Class<*>) : RobolectricTestRunner(fixtureClass) {
    override fun createClassLoaderConfig(method: FrameworkMethod): InstrumentationConfiguration =
        InstrumentationConfiguration.Builder(super.createClassLoaderConfig(method))
            .addInstrumentedPackage("androidx.compose.ui.platform")
            .addInstrumentedClass(fixtureClass.name)
            .build()
}
