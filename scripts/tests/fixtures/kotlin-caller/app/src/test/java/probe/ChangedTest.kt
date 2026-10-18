package probe
import org.junit.Assert.assertEquals
import org.junit.Test
class ChangedTest { @Test fun currentCall() { assertEquals(7, Api().open(7)) } }
