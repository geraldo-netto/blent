package com.blent

/** Independent BLCAM003 golden bytes shared by transport test peers. */
internal object CameraTuningFixture {
    val capabilities = CameraTuningCapabilities(0, 0, 0, 1, false, 1, 1)
    val command = byteArrayOf(0,0,0,0,0,0,0,1, 0,0, 0,0,0, 11,-72, 3,-24, 1)
    fun greeting(token: String) = "BLCAM003".toByteArray() + token.toByteArray() + byteArrayOf(0,0) +
        byteArrayOf(0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0,1,1,0)
    fun ack(sequence: Long) = okio.Buffer().writeLong(sequence).write(command)
    fun reply(output: java.io.OutputStream, sequence: Long) {
        java.io.DataOutputStream(output).writeLong(sequence)
        output.write(command)
    }
}
