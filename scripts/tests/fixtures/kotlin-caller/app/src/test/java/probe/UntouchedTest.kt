package probe
import org.junit.Assert.assertEquals
import org.junit.Test
class UntouchedTest { @Test fun currentCall() { assertEquals(7, Api().open(7)) } }
