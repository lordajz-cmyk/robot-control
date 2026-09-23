/*
 * watchdog.h
 *
 *  Created on: Sep 15, 2025
 *      Author: gunnar
 */

#ifndef WATCHDOG_H_
#define WATCHDOG_H_


#include "ch.h" // For ChibiOS types, if needed

typedef enum {
    SYSTEM_STATE_SAFE,      // No heartbeat, system is stopped
    SYSTEM_STATE_OPERATIONAL // Heartbeat present, system can run
} system_state_t;

// Declare the global state variable
extern system_state_t system_state;

// Declare events or functions for other threads to use
extern event_source_t heartbeat_event;
extern event_source_t heartbeat_lost;

// Declare event sources
// CH_EVENT_SOURCE_DECL(heartbeat_event);
// CH_EVENT_SOURCE_DECL(heartbeat_lost);


#endif /* WATCHDOG_H_ */
