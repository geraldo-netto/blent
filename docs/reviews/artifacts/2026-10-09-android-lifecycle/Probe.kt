package com.blent

import android.content.Context
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow

private fun audioWaiters() = runBlocking {
    val root = SupervisorJob()
    val scope = CoroutineScope(root + Dispatchers.Unconfined)
    val release = CompletableDeferred<Unit>()
    val commands = MutableStateFlow<AudioEndpoint?>(null)
    var opens = 0
    val binding = AudioBinding(Context(), {}, { true }, { _, _, _, _ ->
        opens++
        try { awaitCancellation() }
        finally { withContext(NonCancellable) { release.await() } }
    }, commands, scope)
    try {
        binding.start()
        commands.value = AudioEndpoint(12345)
        for (count in 1..128) {
            commands.value = AudioEndpoint(12345 + count)
            if (count in listOf(8, 32, 128)) {
                println("audio_waiters\t$count\t${root.children.count()}\t$opens")
            }
        }
        binding.stopSharing()
        println("audio_stopped\t${root.children.count()}\t${binding.sharing}")
    } finally {
        binding.shutdown()
        release.complete(Unit)
        root.join()
    }
}

private fun cameraWaiters() = runBlocking {
    val root = SupervisorJob()
    val scope = CoroutineScope(root + Dispatchers.Unconfined)
    val commands = MutableStateFlow<CameraEndpoint?>(null)
    val entered = CountDownLatch(1)
    val release = CountDownLatch(1)
    val opens = AtomicInteger()
    val binding = CameraBinding(Context(), { 0 }, {}, { true }, { _, _, _, owned ->
        opens.incrementAndGet()
        owned.own { entered.countDown(); check(release.await(20, TimeUnit.SECONDS)) }
        awaitCancellation()
    }, commands, scope)
    val field = CameraBinding::class.java.getDeclaredField("retirementScope").apply { isAccessible = true }
    val retirement = field.get(binding) as CoroutineScope
    try {
        binding.start()
        commands.value = CameraEndpoint()
        binding.choose(null)
        check(entered.await(2, TimeUnit.SECONDS))
        for (count in 1..128) {
            binding.choose(CameraLens.REAR)
            binding.choose(null)
            if (count in listOf(8, 32, 128)) {
                println("camera_retirements\t$count\t${retirement.coroutineContext[Job]!!.children.count()}\t${opens.get()}")
            }
        }
    } finally {
        binding.shutdown()
        release.countDown()
        scope.cancel(); root.join()
        retirement.coroutineContext[Job]!!.children.toList().joinAll()
        retirement.cancel()
    }
}

private fun cleanupFailure() = runBlocking {
    var unhandled = 0
    val root = SupervisorJob()
    val handler = CoroutineExceptionHandler { _, error ->
        check(error.message == "injected service stop failure")
        unhandled++
    }
    val scope = CoroutineScope(root + Dispatchers.Unconfined + handler)
    val finish = CompletableDeferred<Unit>()
    val commands = MutableStateFlow<AudioEndpoint?>(null)
    val binding = AudioBinding(Context(), {}, { true }, { _, _, _, _ -> finish.await() }, commands, scope,
        { run -> if (run == null) error("injected service stop failure") })
    try {
        binding.start()
        commands.value = AudioEndpoint(12345, background = true)
        finish.complete(Unit)
        yield()
        println("audio_cleanup_unhandled\t$unhandled\t${root.isActive}")
        check(unhandled == 1 && root.isActive)
    } finally { binding.shutdown(); root.join() }
}

private fun cameraCloseFailure() {
    val resources = CameraResources()
    var attempted = false
    resources.own { attempted = true; error("injected native close failure") }
    val result = runCatching { resources.close() }
    check(attempted && result.isSuccess)
    println("camera_close_failure_reported\t${result.isFailure}")
}

fun main() {
    audioWaiters()
    cameraWaiters()
    cleanupFailure()
    cameraCloseFailure()
}
