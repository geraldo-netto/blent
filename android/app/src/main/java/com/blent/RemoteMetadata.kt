package com.blent

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

internal object RemoteMetadata {
    private var owner: WorkerQueries? = null
    @Synchronized private fun owner(context: Context): WorkerQueries = owner ?: WorkerQueries({ task, report ->
        AndroidWorkerRun(context.applicationContext, task, report = report)
    }).also { owner = it }

    suspend fun report(context: Context, width: Int, height: Int, fps: Int): JSONObject {
        val text = owner(context).query(MediaTask.Metadata(width, height, fps))
        return text?.let { WorkerJsonBoundary.parse(it, WorkerEventJson.LIMIT) } ?: DecoderCapabilities.describe(width, height, fps) { _, _, _, _ -> false }
            .put("hardware", JSONArray()).put("details", JSONArray()).put("status", "unavailable")
    }
    suspend fun shell(context: Context, version: Int): String =
        owner(context).query(MediaTask.Metadata(shellVersion = if (version == 2) 2 else 1)) ?: formatDecoderReport(version, null)
}
