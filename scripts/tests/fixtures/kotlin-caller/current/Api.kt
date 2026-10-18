package probe
internal fun interface Permit { fun current(): Boolean }
internal object Allowed : Permit { override fun current() = true }
internal class Api {
    fun open(value: Int, permit: Permit = Allowed): Int { check(permit.current()); return value }
}
