# Watchdog Implementation for Safety System

## Overview
This document describes the watchdog implementation that ensures the machine stops operating when it loses contact with the control system.

## Changes Made

### 1. Core Watchdog System (watchdog.c/watchdog.h)
- Software watchdog that monitors heartbeat signals
- Two states: `SYSTEM_STATE_SAFE` (no heartbeat) and `SYSTEM_STATE_OPERATIONAL` (heartbeat present)
- 500ms timeout for heartbeat loss detection
- Event broadcasting when heartbeat is lost

### 2. Integration Points

#### main.c
- Uncommented `#include "watchdog.h"`
- Added `watchdog_init()` call in main initialization

#### commands.c
- Added `#include "watchdog.h"`
- Modified `CMD_HEARTBEAT` case to signal the watchdog: `chEvtBroadcast(&heartbeat_event)`

#### motor_control.c
- Added `#include "watchdog.h"`
- Modified `motor_set_speed()` to check `system_state` and force speed to 0 if unsafe

#### steering_control.c
- Added `#include "watchdog.h"`
- Modified `steering_control_update()` to check `system_state` and return 0 output if unsafe

#### hydraulic.c
- Added `#include "watchdog.h"`
- Modified `hydraulic_set_speed()` to check `system_state` and force speed to 0 if unsafe

## How It Works

1. **Initialization**: Watchdog thread starts when `watchdog_init()` is called
2. **Heartbeat Monitoring**: Watchdog thread checks for heartbeat events every 10ms
3. **State Management**: 
   - If heartbeat received within 500ms: `system_state = SYSTEM_STATE_OPERATIONAL`
   - If no heartbeat for 500ms: `system_state = SYSTEM_STATE_SAFE`
4. **Safety Checks**: Critical systems check `system_state` before operating

## Usage

### Sending Heartbeats
The control system should send `CMD_HEARTBEAT` commands regularly (more frequently than 500ms):

```c
// Example heartbeat command
uint8_t heartbeat_cmd[] = {main_id, CMD_HEARTBEAT};
comm_usb_send_packet(heartbeat_cmd, sizeof(heartbeat_cmd));
```

### Checking System State
Critical systems should check the state before operating:

```c
if (system_state != SYSTEM_STATE_OPERATIONAL) {
    // System is in safe mode - don't operate actuators
    return;
}
```

## Testing

A basic test file `watchdog_test.c` has been provided to verify:
- Watchdog initialization
- State transitions
- Thread operation

## Future Enhancements

1. **Hardware Watchdog**: Implement hardware watchdog timer for additional safety
2. **Graceful Degradation**: Add procedures for safe shutdown when heartbeat is lost
3. **Recovery Procedures**: Define how the system should recover when heartbeat is restored
4. **Logging**: Add watchdog state changes to the logging system
5. **Configuration**: Make timeout period configurable

## Safety Considerations

- The watchdog timeout (500ms) should be longer than the normal communication interval
- All actuator control functions should check the system state
- The watchdog should be the last line of defense, not the primary safety mechanism
- Consider adding visual/audible indicators for watchdog state changes