/*
 * watchdog.c
 *
 *  Created on: Sep 15, 2025
 *      Author: gunnar
 */

// watchdog.c
#include "watchdog.h"

// Define the global state
system_state_t system_state = SYSTEM_STATE_SAFE;

// Define events
event_source_t heartbeat_event;
event_source_t heartbeat_lost;

static THD_FUNCTION(WatchdogThread, arg);
// Thread working area (stack)
static THD_WORKING_AREA(waWatchdog, 256);

static THD_FUNCTION(WatchdogThread, arg) {
    (void)arg;
    chRegSetThreadName("Watchdog");

    systime_t last_heartbeat = chVTGetSystemTime();
    const systime_t timeout = MS2ST(500); // 500ms timeout

    while (true) {
        // Check for heartbeat event (non-blocking)
        eventflags_t flags = chEvtGetAndClearFlags(&heartbeat_event);
        if (flags & 1) { // Assuming you use the first flag for heartbeat
            last_heartbeat = chVTGetSystemTime();
            system_state = SYSTEM_STATE_OPERATIONAL; // Heartbeat received
        }

        // Check for timeout
        if (chVTTimeElapsedSinceX(last_heartbeat) > timeout) {
            system_state = SYSTEM_STATE_SAFE; // Heartbeat lost
            chEvtBroadcast(&heartbeat_lost); // Notify other threads
        }

        chThdSleepMilliseconds(10);
    }
}


void watchdog_init(void) {
// Create and start the watchdog thread
chThdCreateStatic(
    waWatchdog,               // Working area (stack)
    sizeof(waWatchdog),       // Size of the working area
    NORMALPRIO + 1,           // Priority (adjust as needed)
    WatchdogThread,           // Thread function
    NULL                      // Argument to pass to the thread
);
}

