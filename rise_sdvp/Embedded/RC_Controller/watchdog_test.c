/*
 * watchdog_test.c
 *
 * Simple test to verify watchdog functionality
 */

#include "watchdog.h"
#include "ch.h"
#include "hal.h"
#include "test.h"

// Test watchdog initialization and state transitions
static void test_watchdog_basic(void) {
    // Initialize watchdog
    watchdog_init();
    
    // Test initial state
    test_assert(system_state == SYSTEM_STATE_SAFE, "Initial state should be SAFE");
    
    // Simulate heartbeat
    chEvtBroadcast(&heartbeat_event);
    chThdSleepMilliseconds(100); // Give watchdog thread time to process
    
    // Test state after heartbeat
    test_assert(system_state == SYSTEM_STATE_OPERATIONAL, "State should be OPERATIONAL after heartbeat");
    
    // Test timeout (this would require more complex setup to test properly)
    // For now, just verify the watchdog is running
    test_assert(chThdStateI(&WatchdogThread) != THD_STATE_READY, "Watchdog thread should be running");
}

// Test that critical systems respect watchdog state
static void test_watchdog_integration(void) {
    // This would test that motor_control, steering_control, etc.
    // properly check system_state before operating
    // Implementation would depend on your testing framework
}

void watchdog_run_tests(void) {
    test_set_name("Watchdog Tests");
    
    test_watchdog_basic();
    // test_watchdog_integration(); // Uncomment when integration tests are implemented
    
    test_print_results();
}