package com.blent

import kotlinx.coroutines.*
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [27, 34])
class WorkerContractTest {
    private val token = "a".repeat(64)
    private fun audio(direction: Int = 1) = MediaTask.Audio(AudioEndpoint(token, 1234, direction, 1, 40, false), AudioPreferences())
    @Test fun t809_requestsRoundTripWithoutNativeTypesOrFrameCopies() {
        val tasks = listOf(audio(), audio(2),
            MediaTask.Camera(CameraEndpoint(token, 1234, 1280, 720, 30, 3000), CameraLens.FRONT, 0),
            MediaTask.Camera(CameraEndpoint(token, 1234, 640, 480, 10, 2000, requestedLens = CameraLens.REAR), CameraLens.REAR, 3),
            MediaTask.Display(AdbDisplayRoutes.create(token), DecoderFormat("video/avc", 1280, 720, 60)),
            MediaTask.Display(AdbDisplayRoutes.create(token), DecoderFormat("video/hevc", 1920, 1080, 30, selection = DecoderSelection("c2.fixture", "hevc", "main", 1, 8, true, 60))),
            MediaTask.Display(AdbDisplayRoutes.create(token), DecoderFormat("video/avc", 1, 1, 10, selection = DecoderSelection("fixture", "h264", "baseline", 1, 8, false, null))),
            MediaTask.Metadata(640, 480, 60), MediaTask.Metadata(shellVersion = 2))
        for (task in tasks) assertEquals(task, MediaTaskJson.decode(MediaTaskJson.encode(task)))
        assertThrows(IllegalArgumentException::class.java) { MediaTaskJson.encode(MediaTask.Display(AdbDisplayRoutes.create(token), DecoderFormat("video/avc", 1, 1, 30, byteArrayOf(1)))) }
    }
    @Test fun t809_requestBoundariesRejectMalformedAndOutOfRangeFields() {
        val encoded = MediaTaskJson.encode(audio())
        for (length in 0 until encoded.length) assertTrue(runCatching { MediaTaskJson.decode(encoded.take(length)) }.isFailure)
        for (field in listOf("version", "port", "direction", "processing", "buffer", "gain", "override")) {
            for (value in listOf(Int.MIN_VALUE, -1, Int.MAX_VALUE)) {
                val bad = JSONObject(encoded).put(field, value)
                assertTrue("$field=$value", runCatching { MediaTaskJson.decode(bad.toString()) }.isFailure)
            }
        }
        val badTasks = listOf(
            audio().copy(endpoint = audio().endpoint.copy(background = true)),
            audio().copy(endpoint = audio().endpoint.copy(token = "bad")),
            MediaTask.Camera(CameraEndpoint(token, 1, 0, 720, 30, 1000), CameraLens.REAR, 0),
            MediaTask.Camera(CameraEndpoint(token, 1, 640, 480, 30, 1000), CameraLens.REAR, 4),
            MediaTask.Display(AdbDisplayRoutes.create("bad"), DecoderFormat("video/avc", 640, 480, 30)),
            MediaTask.Display(AdbDisplayRoutes.create(token), DecoderFormat("unknown", 640, 480, 30)),
            MediaTask.Display(AdbDisplayRoutes.create(token), DecoderFormat("video/avc", Int.MAX_VALUE, 480, 30)),
            MediaTask.Metadata(0, 0, 0), MediaTask.Metadata(shellVersion = 1), MediaTask.Metadata(shellVersion = 3))
        for (task in badTasks) assertTrue(runCatching { MediaTaskJson.decode(MediaTaskJson.encode(task)) }.isFailure)
        assertTrue(runCatching { MediaTaskJson.decode(JSONObject(encoded).put("kind", "unknown").toString()) }.isFailure)
        assertTrue(runCatching { MediaTaskJson.decode(" ".repeat(MediaTaskJson.LIMIT + 1)) }.isFailure)
        assertTrue(runCatching { MediaTaskJson.encode(MediaTask.Display(AdbDisplayRoutes.create(token), DecoderFormat("x".repeat(MediaTaskJson.LIMIT), 1, 1, 30))) }.isFailure)
    }
    @Test fun t809_eventsRoundTripNullDiagnosticsAndBoundedText() {
        val active = ActiveDecoderDiagnostics("codec", "video/avc", 1280, 720, 60, true, false, null, listOf("Standard low latency=1", "quote \\\" [{}]"))
        val events = listOf(WorkerEvent.Status("ready"), WorkerEvent.Result("{\"value\":1}"), WorkerEvent.Ended(null), WorkerEvent.Ended("failed"),
            WorkerEvent.Rendered(Int.MIN_VALUE, -1, null), WorkerEvent.Rendered(Int.MAX_VALUE, 0, "receipt"),
            WorkerEvent.Display(false, 0f, 0f, null, null, DecoderDiagnostics()),
            WorkerEvent.Display(true, 59f, 2f, "starting", "stopping", DecoderDiagnostics(active, true)))
        for (event in events) assertEquals(event, WorkerEventJson.decode(WorkerEventJson.encode(event)))
        assertEquals(2048, (WorkerEventJson.decode(WorkerEventJson.encode(WorkerEvent.Status("x".repeat(3000)))) as WorkerEvent.Status).text.length)
        assertTrue(runCatching { WorkerEventJson.encode(WorkerEvent.Result("x".repeat(WorkerEventJson.LIMIT))) }.isFailure)
        assertTrue(runCatching { WorkerEventJson.decode(" ".repeat(WorkerEventJson.LIMIT + 1)) }.isFailure)
        assertTrue(runCatching { WorkerEventJson.decode("{\"type\":\"wrong\"}") }.isFailure)
        val corrupt = WorkerEvent.Display(true, 1f, 1f, null, null, DecoderDiagnostics(active.copy(requested = List(17) { "hint" })))
        assertTrue(runCatching { WorkerEventJson.decode(WorkerEventJson.encode(corrupt)) }.isFailure)
    }
    @Test fun t809_boundedJsonRejectsDeepMalformedAndFuzzedInput() {
        for (depth in listOf(17, 100, 1000)) {
            val text = "{\"v\":" + "[".repeat(depth) + "0" + "]".repeat(depth) + "}"
            assertThrows(IllegalArgumentException::class.java) { WorkerJsonBoundary.parse(text, 10000) }
        }
        for (text in listOf("}", "{", "{\"x\":\"unfinished", "{\"x\":\"escape\\", "[0]")) {
            assertTrue(runCatching { WorkerJsonBoundary.parse(text, 10000) }.isFailure)
        }
        val random = java.util.Random(809)
        val chars = "{}[]\\\"012a: ,"
        repeat(300) {
            val text = CharArray(random.nextInt(128)) { chars[random.nextInt(chars.length)] }.concatToString()
            try { WorkerJsonBoundary.parse(text, 128) } catch (_: Exception) {}
        }
    }
    @Test fun t809_mailboxBoundsBytesCountAndReleasesCapacity() = runBlocking {
        val mailbox = WorkerMailbox()
        assertFalse(mailbox.offer("x".repeat(262145)))
        val large = "x".repeat(262144)
        assertTrue(mailbox.offer(large)); assertFalse(mailbox.offer("a"))
        assertEquals(large, mailbox.receive())
        repeat(32) { assertTrue(mailbox.offer("$it")) }
        assertFalse(mailbox.offer("overflow"))
        repeat(32) { assertEquals("$it", mailbox.receive()) }
        mailbox.close(); assertFalse(mailbox.offer("closed"))
        assertTrue(runCatching { mailbox.receive() }.isFailure)
    }
    @Test fun t809_queryCancellationKeepsAdmissionUntilRetirementAndQuarantinesFailure() = runBlocking {
        val retiring = CompletableDeferred<Unit>()
        val entered = CompletableDeferred<Unit>()
        var creates = 0
        val queries = WorkerQueries({ _, _ ->
            creates++
            object : LifecycleRun {
                override suspend fun execute() { entered.complete(Unit); awaitCancellation() }
                override fun cancel() {}
                override suspend fun retire() { retiring.await() }
            }
        }, 20)
        assertNull(queries.query(MediaTask.Metadata(1, 1, 1)))
        entered.await(); assertTrue(queries.busy)
        repeat(100) { assertNull(queries.query(MediaTask.Metadata(1, 1, 1))) }
        assertEquals(1, creates)
        retiring.complete(Unit)
        withTimeout(1000) { while (queries.busy) delay(1) }
        val retired = CompletableDeferred<Unit>()
        val failing = WorkerQueries({ _, report -> object : LifecycleRun {
            override suspend fun execute() { report(WorkerEvent.Status("ignored")); report(WorkerEvent.Result("ok")) }
            override fun cancel() { error("cleanup failed") }
            override suspend fun retire() { retired.complete(Unit) }
        } })
        assertEquals("ok", failing.query(MediaTask.Metadata(1, 1, 1)))
        withTimeout(1000) { retired.await() }
        assertTrue(failing.busy)
        assertNull(failing.query(MediaTask.Metadata(1, 1, 1)))
        val broken = WorkerQueries({ _, _ -> error("creation failed") })
        assertNull(broken.query(MediaTask.Metadata(1, 1, 1)))
        withTimeout(1000) { while (broken.busy) delay(1) }
    }
}
