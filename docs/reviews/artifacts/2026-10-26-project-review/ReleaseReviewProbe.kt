package com.blent
import okhttp3.*
import okhttp3.ResponseBody.Companion.toResponseBody
import org.junit.Test
import org.junit.Assert.*
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.TimeUnit

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class ReleaseReviewProbe {
    @Test fun malformedReleaseResponse() {
        val client = OkHttpClient.Builder().addInterceptor { chain ->
            Response.Builder().request(chain.request()).protocol(Protocol.HTTP_1_1)
                .code(200).message("fixture")
                .body("""{"tag_name":"v9.0.0"} garbage""".toResponseBody()).build()
        }.build()
        val call = HttpReleaseChecks("http://localhost/release", client).newCall("1.2.3")
        val result = ArrayBlockingQueue<String>(1)
        try {
            call.start { result.add(it ?: "none") }
            val actual = result.poll(5, TimeUnit.SECONDS)
            println("T1265 malformed release response accepted as: $actual")
            assertEquals("9.0.0", actual)
        } finally {
            call.cancel()
            client.dispatcher.executorService.shutdown()
            assertTrue(client.dispatcher.executorService.awaitTermination(5, TimeUnit.SECONDS))
            client.connectionPool.evictAll()
        }
    }
}
