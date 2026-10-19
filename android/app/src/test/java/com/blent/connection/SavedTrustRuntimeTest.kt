package com.blent.connection

import org.junit.Assert.*
import org.junit.Test

class SavedTrustRuntimeTest {
    @Test fun t1151ConnectionSetupCannotPublishMaintenanceMetadata() {
        val rig = RuntimeRig()
        val page = ConnectionWorkerEvent.SavedComputers(SavedComputersPage(null, 0, 0, 0, emptyList()))
        rig.setup.events.add(page)
        assertNull(rig.owner.poll())
        rig.start()
        rig.setup.events.add(page)
        assertNull(rig.owner.poll())
        rig.owner.stop()
        rig.setup.events.add(page)
        assertNull(rig.owner.poll())
    }
}
