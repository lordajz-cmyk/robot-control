# Commands Module Documentation

## Overview

The `commands.c` module is the central command processing hub for the RC Controller system. It handles incoming command packets, routes them to the appropriate handlers, and manages communication with various subsystems including positioning, autopilot, sensors, and CAN bus devices.

## Key Features

- **Command Packet Processing**: Parses and routes incoming command packets based on their IDs
- **Position Control**: Handles position updates and ENU (East-North-Up) reference settings
- **Autopilot Integration**: Manages autopilot route points and control commands
- **Sensor Data Processing**: Processes sensor data including angle measurements
- **Configuration Management**: Handles system configuration updates and retrieval
- **RTCM3 Support**: Processes differential GPS correction data (RTCM3 protocol)
- **CAN Bus Integration**: Forwards VESC packets and manages CAN communication
- **Emergency Stop Handling**: Manages emergency stop signals and system shutdown
- **Logging Support**: Handles logging commands and data forwarding
- **Time Synchronization**: Manages system time and timestamp updates
- **Hydraulic Control**: Processes hydraulic actuator commands
- **Keyboard Control**: Manages keyboard-based control input

## Module Architecture

### Packet Structure

All commands use a consistent packet format:

```
Byte 0: Device ID (target device address)
Byte 1: Command ID (CMD_PACKET enum value)
Bytes 2+: Command-specific data (variable length)
```

### Device IDs

- `main_id`: This device's unique ID
- `ID_ALL`: Broadcast to all devices (0xFF)
- `ID_VEHICLE_CLIENT`: Vehicle client ID
- Other device-specific IDs

### Command Processing Flow

1. **Packet Reception**: `commands_process_packet()` receives raw packet data
2. **Header Parsing**: Extract device ID and command ID
3. **Address Filtering**: Check if packet is addressed to this device
4. **Command Routing**: Switch on command ID to execute appropriate handler
5. **Response Generation**: Send acknowledgments or responses as needed

## Command Categories

### General Commands (0-49)

| Command | ID | Description |
|---------|----|-------------|
| `CMD_PRINTF` | 0 | Print formatted message |
| `CMD_TERMINAL_CMD` | 1 | Execute terminal command |
| ... | ... | ... |
| `CMD_VESC_FWD` | 50 | Forward VESC packet |

### Vehicle Commands (51-99)

| Command | ID | Description |
|---------|----|-------------|
| `CMD_SET_POS` | 51 | Set position (x, y, angle) |
| `CMD_SET_POS_ACK` | 52 | Set position with acknowledgment |
| `CMD_SET_ENU_REF` | 53 | Set ENU reference position |
| `CMD_GET_ENU_REF` | 54 | Get ENU reference position |
| `CMD_AP_ADD_POINTS` | 55 | Add autopilot route points |
| `CMD_AP_REMOVE_LAST_POINT` | 56 | Remove last autopilot point |
| `CMD_AP_CLEAR_POINTS` | 57 | Clear all autopilot points |
| `CMD_AP_GET_ROUTE_PART` | 58 | Get part of autopilot route |
| `CMD_AP_SET_ACTIVE` | 59 | Activate/deactivate autopilot |
| `CMD_AP_REPLACE_ROUTE` | 60 | Replace entire autopilot route |
| `CMD_AP_SYNC_POINT` | 61 | Synchronize autopilot point |
| `CMD_SEND_RTCM_USB` | 62 | Send RTCM data via USB |
| `CMD_SEND_NMEA_RADIO` | 63 | Send NMEA data via radio |
| `CMD_SET_YAW_OFFSET` | 64 | Set yaw offset |
| `CMD_SET_YAW_OFFSET_ACK` | 65 | Set yaw offset with acknowledgment |
| `CMD_LOG_LINE_USB` | 66 | Log line to USB |
| ... | ... | ... |
| `CMD_SET_MS_TODAY` | 71 | Set milliseconds since midnight |
| `CMD_SET_SYSTEM_TIME` | 72 | Set system time |
| `CMD_SET_SYSTEM_TIME_ACK` | 73 | Set system time with acknowledgment |
| `CMD_REBOOT_SYSTEM` | 74 | Reboot system |
| `CMD_REBOOT_SYSTEM_ACK` | 75 | Reboot system with acknowledgment |
| `CMD_EMERGENCY_STOP` | 76 | Emergency stop command |
| `CMD_SET_MAIN_CONFIG` | 77 | Set main configuration |
| `CMD_GET_MAIN_CONFIG` | 78 | Get main configuration |
| `CMD_GET_MAIN_CONFIG_DEFAULT` | 79 | Get default main configuration |
| `CMD_ADD_UWB_ANCHOR` | 80 | Add UWB anchor |
| `CMD_CLEAR_UWB_ANCHORS` | 81 | Clear all UWB anchors |
| `CMD_LOG_ETHERNET` | 82 | Log to Ethernet |
| `CMD_CAMERA_IMAGE` | 83 | Camera image data |
| `CMD_CAMERA_STREAM_START` | 84 | Start camera stream |
| `CMD_CAMERA_FRAME_ACK` | 85 | Camera frame acknowledgment |
| `CMD_IO_BOARD_SET_PWM_DUTY` | 86 | Set IO board PWM duty |
| `CMD_IO_BOARD_SET_VALVE` | 87 | Set IO board valve |
| `CMD_HYDRAULIC_MOVE` | 88 | Move hydraulic actuator |
| `CMD_HEARTBEAT` | 89 | Heartbeat signal |
| `CMD_SET_AP_MODE` | 90 | Set autopilot mode |
| `CMD_KB_SET_ACTIVE` | 91 | Set keyboard control active |

### Sensor Commands (100-149)

| Command | ID | Description |
|---------|----|-------------|
| `CMD_GET_ANGLE` | 100 | Get angle from sensor |
| ... | ... | ... |
| `CMD_GET_STATE` | 120 | Get system state |
| `CMD_RC_CONTROL` | 121 | RC control command |
| `CMD_SET_SERVO_DIRECT` | 122 | Set servo directly |
| `CMD_GET_ACTUATORS` | 123 | Get actuator states |
| `CMD_SET_ACTUATORS` | 124 | Set actuator states |

## Public API

### Initialization

```c
void commands_init(void);
```
Initializes the commands module, including:
- Send function pointer initialization
- GPS print mutex initialization
- Virtual timer initialization
- RTCM3 state initialization and callback registration
- Debug variable initialization

### Send Function Management

```c
void commands_set_send_func(void(*func)(unsigned char *data, unsigned int len));
```
Registers the callback function for sending packets.
- `func`: Function pointer with signature `void func(unsigned char *data, unsigned int len)`

```c
void commands_send_packet(unsigned char *data, unsigned int len);
```
Sends a packet using the registered send function.
- `data`: Packet data (byte array)
- `len`: Length of the packet in bytes

### Packet Processing

```c
void commands_process_packet(unsigned char *data, unsigned int len,
        void (*func)(unsigned char *data, unsigned int len));
```
Processes a received command packet and executes the appropriate action.
- `data`: Received packet data (device ID, command ID, and payload)
- `len`: Length of the packet in bytes
- `func`: Send function for responses (optional)

### Printing and Logging

```c
void commands_printf(const char* format, ...);
```
Prints a formatted string to the output. Thread-safe for GPS printing.
- `format`: printf-style format string
- `...`: Variable arguments

```c
void commands_printf_log_usb(char* format, ...);
```
Prints a formatted string and logs it to USB.
- `format`: printf-style format string
- `...`: Variable arguments

### CAN Bus Forwarding

```c
void commands_forward_vesc_packet(unsigned char *data, unsigned int len);
```
Forwards a VESC packet to the CAN bus.
- `data`: VESC packet data
- `len`: Length of the packet

### NMEA Forwarding

```c
void commands_send_nmea(unsigned char *data, unsigned int len);
```
Sends NMEA data (typically GPS data).
- `data`: NMEA data
- `len`: Length of the data

### RTCM3 Access

```c
rtcm3_state* commands_get_rtcm3_state(void);
```
Returns a pointer to the RTCM3 state structure.
- Returns: Pointer to the RTCM3 state

### Sleep

```c
void commands_sleep(void);
```
Puts the system to sleep (implementation depends on hardware).

## Command Handlers

### Terminal Command (CMD_TERMINAL_CMD)

Handles terminal command strings for system configuration and control.

**Data Format**: Null-terminated string

**Action**:
- Resets system timeout
- Registers send function for responses
- Processes the command string via `terminal_process_string()`

### Arduino Status (CMD_ARDUINO_STATUS)

Reports the connection status of an Arduino device.

**Data Format**: [status (1 byte)]
- 0: Disconnected
- Non-zero: Connected

**Action**: Updates `arduino_connected` flag

### Get Angle (CMD_GETANGLE)

Receives and processes angle sensor data.

**Data Format**: [start_byte (0xAA), voltage_high (1 byte), voltage_low (1 byte), checksum (1 byte)]

**Action**:
1. Validates packet length (minimum 4 bytes)
2. Validates start byte (0xAA)
3. Validates checksum (XOR of start byte + 2 data bytes)
4. Extracts 16-bit voltage value (big-endian)
5. Converts to float voltage (scaled by 1000)
6. Calculates front angle using configuration parameters:
   ```
   frontangle = (voltage - sensorcentre) * (degreeinterval / sensorinterval)
   ```
7. Updates IO board AS5047 angle via CAN bus
8. Optionally outputs debug information (when iDebug == 31)

**Configuration Parameters Used**:
- `main_config.vehicle.sensorcentre`: Center voltage value
- `main_config.vehicle.degreeinterval`: Degrees per interval
- `main_config.vehicle.sensorinterval`: Voltage interval

### Set Position (CMD_SET_POS, CMD_SET_POS_ACK)

Sets the vehicle position and angle.

**Data Format**: [x (float32), y (float32), angle (float32)]
- x, y: Position coordinates (scaled by 1e4)
- angle: Heading angle (scaled by 1e6)

**Action**:
1. Resets system timeout
2. Extracts x, y, angle values from packet
3. Updates position via `pos_set_xya()`
4. Updates UWB position via `pos_uwb_set_xya()`
5. If CMD_SET_POS_ACK: Sends acknowledgment packet

### Set ENU Reference (CMD_SET_ENU_REF)

Sets the East-North-Up reference position.

**Data Format**: [enu_ref (float32 array)]

**Action**:
1. Resets system timeout
2. Registers send function for responses
3. Extracts ENU reference values
4. Updates position reference
5. Sends acknowledgment if requested

### Autopilot Commands

#### Add Points (CMD_AP_ADD_POINTS)

Adds route points to the autopilot.

**Data Format**: Variable-length array of ROUTE_POINT structures

**Action**:
1. Resets system timeout
2. Registers send function for responses
3. Extracts route points from packet
4. Adds points to autopilot via `autopilot_add_point()`
5. Sends acknowledgment

#### Remove Last Point (CMD_AP_REMOVE_LAST_POINT)

Removes the last point from the autopilot route.

**Action**:
1. Resets system timeout
2. Calls `autopilot_remove_last_point()`
3. Sends acknowledgment

#### Clear Points (CMD_AP_CLEAR_POINTS)

Clears all points from the autopilot route.

**Action**:
1. Resets system timeout
2. Calls `autopilot_clear_route()`
3. Sends acknowledgment

#### Get Route Part (CMD_AP_GET_ROUTE_PART)

Retrieves a portion of the autopilot route.

**Data Format**: [first (int32), last (int32)]

**Action**:
1. Resets system timeout
2. Registers send function for responses
3. Extracts first and last indices
4. Retrieves route points via `autopilot_get_route_point()`
5. Sends the requested route points as response

#### Set Active (CMD_AP_SET_ACTIVE)

Activates or deactivates the autopilot.

**Data Format**: [active (1 byte, 0=inactive, 1=active)]

**Action**:
1. Resets system timeout
2. Extracts active flag
3. Calls `autopilot_set_active()`
4. Sends acknowledgment

#### Replace Route (CMD_AP_REPLACE_ROUTE)

Replaces the entire autopilot route.

**Data Format**: Variable-length array of ROUTE_POINT structures

**Action**:
1. Resets system timeout
2. Registers send function for responses
3. Extracts route points from packet
4. Calls `autopilot_replace_route()` for each point
5. Sends acknowledgment

#### Sync Point (CMD_AP_SYNC_POINT)

Synchronizes a specific route point with timing information.

**Data Format**: [point (int32), time (int32), min_time_diff (int32)]

**Action**:
1. Resets system timeout
2. Registers send function for responses
3. Extracts point index, time, and minimum time difference
4. Calls `autopilot_sync_point()`
5. Sends acknowledgment

### RTCM3 Commands

#### Send RTCM USB (CMD_SEND_RTCM_USB)

Sends RTCM3 differential GPS data via USB.

**Data Format**: RTCM3 packet data

**Action**:
1. Resets system timeout
2. Forwards data to USB via `comm_usb_send_buffer()`

#### Send NMEA Radio (CMD_SEND_NMEA_RADIO)

Sends NMEA data via radio.

**Data Format**: NMEA string

**Action**:
1. Resets system timeout
2. Forwards data to radio

### Yaw Offset (CMD_SET_YAW_OFFSET, CMD_SET_YAW_OFFSET_ACK)

Sets the yaw offset for the positioning system.

**Data Format**: [yaw_offset (float32)]

**Action**:
1. Resets system timeout
2. Extracts yaw offset value
3. Updates position yaw offset via `pos_set_yaw_offset()`
4. If CMD_SET_YAW_OFFSET_ACK: Sends acknowledgment

### Logging Commands

#### Log Line USB (CMD_LOG_LINE_USB)

Logs a line to USB storage.

**Data Format**: Text line to log

**Action**:
1. Resets system timeout
2. Writes to log via `log_line_usb()`

#### Log Ethernet (CMD_LOG_ETHERNET)

Logs data to Ethernet.

**Data Format**: Data to log

**Action**:
1. Resets system timeout
2. Forwards to Ethernet logging

### Camera Commands

#### Camera Image (CMD_CAMERA_IMAGE)

Handles camera image data.

**Data Format**: Camera image data

**Action**:
1. Resets system timeout
2. Processes camera image

#### Camera Stream Start (CMD_CAMERA_STREAM_START)

Starts camera streaming.

**Action**:
1. Resets system timeout
2. Starts camera stream

#### Camera Frame ACK (CMD_CAMERA_FRAME_ACK)

Acknowledges camera frame reception.

**Action**:
1. Resets system timeout
2. Sends frame acknowledgment

### IO Board Commands

#### Set PWM Duty (CMD_IO_BOARD_SET_PWM_DUTY)

Sets PWM duty cycle on IO board.

**Data Format**: [pwm_number (1 byte), duty (float32)]

**Action**:
1. Resets system timeout
2. Extracts PWM number and duty cycle
3. Sets PWM via CAN bus

#### Set Valve (CMD_IO_BOARD_SET_VALVE)

Sets valve position on IO board.

**Data Format**: [valve_number (1 byte), position (float32)]

**Action**:
1. Resets system timeout
2. Extracts valve number and position
3. Sets valve via CAN bus

### Hydraulic Command (CMD_HYDRAULIC_MOVE)

Moves a hydraulic actuator.

**Data Format**: [actuator (1 byte), position (float32)]

**Action**:
1. Resets system timeout
2. Extracts actuator ID and position
3. Calls `hydraulic_move()` to control the actuator

### Time Commands

#### Set Milliseconds Today (CMD_SET_MS_TODAY)

Sets the milliseconds since midnight (used for time synchronization).

**Data Format**: [ms_today (int32)]

**Action**:
1. Resets system timeout
2. Extracts milliseconds value
3. Updates position time via `pos_set_ms_today()`
4. Sends acknowledgment

#### Set System Time (CMD_SET_SYSTEM_TIME)

Sets the system time.

**Data Format**: [time (int32)]

**Action**:
1. Resets system timeout
2. Extracts time value
3. Updates system time
4. Sends acknowledgment

### Emergency Stop (CMD_EMERGENCY_STOP)

Handles emergency stop command.

**Data Format**: None (or optional data)

**Action**:
1. Resets system timeout
2. Triggers emergency stop sequence:
   - Stops motor via `motor_control_stop()`
   - Deactivates autopilot via `autopilot_set_active(false)`
   - Stops state control
   - Resets position
   - Sends acknowledgment

### Configuration Commands

#### Set Main Config (CMD_SET_MAIN_CONFIG)

Sets the main system configuration.

**Data Format**: Configuration data structure

**Action**:
1. Resets system timeout
2. Extracts configuration data
3. Updates main configuration
4. Sends acknowledgment

#### Get Main Config (CMD_GET_MAIN_CONFIG)

Retrieves the main system configuration.

**Action**:
1. Resets system timeout
2. Registers send function for responses
3. Sends current main configuration as response

#### Get Main Config Default (CMD_GET_MAIN_CONFIG_DEFAULT)

Retrieves the default main system configuration.

**Action**: Similar to CMD_GET_MAIN_CONFIG but sends default values

### UWB Anchor Commands

#### Add UWB Anchor (CMD_ADD_UWB_ANCHOR)

Adds a UWB (Ultra-Wideband) anchor for positioning.

**Data Format**: UWB anchor configuration

**Action**:
1. Resets system timeout
2. Extracts anchor data
3. Adds anchor to UWB positioning system
4. Sends acknowledgment

#### Clear UWB Anchors (CMD_CLEAR_UWB_ANCHORS)

Clears all UWB anchors.

**Action**:
1. Resets system timeout
2. Clears all UWB anchors
3. Sends acknowledgment

### Get State (CMD_GET_STATE)

Retrieves the current system state.

**Action**:
1. Resets system timeout
2. Registers send function for responses
3. Collects system state information
4. Sends state as response

### RC Control (CMD_RC_CONTROL)

Handles RC (Remote Control) input.

**Data Format**: RC control data

**Action**:
1. Resets system timeout
2. Processes RC control input
3. Updates motor control based on RC input

### Servo Direct (CMD_SET_SERVO_DIRECT)

Sets servo position directly.

**Data Format**: [servo_number (1 byte), position (float32)]

**Action**:
1. Resets system timeout
2. Extracts servo number and position
3. Sets servo via `servo_simple_set_pos()`

### Actuator Commands

#### Get Actuators (CMD_GET_ACTUATORS)

Retrieves the current state of all actuators.

**Action**:
1. Resets system timeout
2. Registers send function for responses
3. Collects actuator states
4. Sends actuator states as response

#### Set Actuators (CMD_SET_ACTUATORS)

Sets the state of actuators.

**Data Format**: Actuator configuration

**Action**:
1. Resets system timeout
2. Extracts actuator configuration
3. Updates actuator states
4. Sends acknowledgment

## RTCM3 Support

The module includes support for RTCM3 differential GPS correction data:

- **RTCM3 Preamble**: 0xD3 (used to detect RTCM3 packets)
- **Callback Registration**: `rtcm3_set_rx_callback()` and `rtcm3_set_rx_callback_1005_1006()`
- **State Management**: `rtcm3_init_state()` initializes the RTCM3 parser state
- **Data Processing**: `rtcm3_input_data()` processes each byte of RTCM3 data

RTCM3 packets are processed separately from regular commands and are fed directly to the RTCM3 parser.

## CAN Bus Integration

The module forwards VESC packets to the CAN bus:

```c
void commands_forward_vesc_packet(unsigned char *data, unsigned int len);
```

This function:
1. Sets up the send function for responses
2. Forwards the packet via `comm_can_forward_vesc_packet()`

## Emergency Stop Handling

The emergency stop sequence includes:
1. Stopping motor control
2. Deactivating autopilot
3. Stopping state control
4. Resetting position
5. Sending acknowledgment

This ensures the vehicle comes to a safe stop when an emergency command is received.

## Debugging Support

The module includes extensive debugging support:

- **Debug Values**: 15 debug value variables (debugvalue1-15) for storing intermediate values
- **Debug Output**: Conditional debug output based on `iDebug` value
- **Command Counter**: `iCounterCommands` tracks the number of commands processed

Specific debug modes:
- `iDebug == 31`: Angle sensor debugging
- Other values: Various command-specific debugging

## Configuration Dependencies

The module depends on configuration parameters from `main_config`:

- `vehicle.sensorcentre`: Center voltage for angle sensor
- `vehicle.degreeinterval`: Degrees per interval for angle calculation
- `vehicle.sensorinterval`: Voltage interval for angle calculation
- Various other vehicle and system parameters

## Usage Example

```c
// Include the commands module
#include "commands.h"

// Define a send function
void my_send_function(unsigned char *data, unsigned int len) {
    // Send data via UART, CAN, USB, etc.
    uart_send(data, len);
}

// Initialize the commands module
void init_commands(void) {
    commands_init();
    commands_set_send_func(my_send_function);
}

// Process received data
void on_receive(unsigned char *data, unsigned int len) {
    // Process the command packet
    commands_process_packet(data, len, my_send_function);
}

// Send a command
void send_command(CMD_PACKET cmd, void *data, int len) {
    unsigned char packet[256];
    int index = 0;
    
    packet[index++] = main_id; // Target device
    packet[index++] = cmd; // Command ID
    
    // Copy command data
    memcpy(packet + index, data, len);
    index += len;
    
    // Send the packet
    commands_send_packet(packet, index);
}
```

## Thread Safety

The module includes some thread safety mechanisms:

- **GPS Print Mutex**: `m_print_gps` protects GPS printing operations
- **Virtual Timer**: Used for timeout handling

However, most of the module is not inherently thread-safe. If used in a multi-threaded environment, external synchronization may be required for:
- Shared debug variables
- Command processing
- Send function registration

## Memory Usage

- Send buffer: `PACKET_MAX_PL_LEN` bytes (defined in packet.h)
- RTCM3 state: ~1KB (depends on rtcm3_simple.h)
- Debug variables: ~60 bytes (15 float values)
- Other variables: ~20 bytes

## Dependencies

The module depends on:
- `commands.h`: Header file with function declarations
- `ch.h`, `hal.h`: ChibiOS/RT headers for threading and timers
- `packet.h`: Packet definitions and constants
- `pos.h`, `pos_uwb.h`: Position system interfaces
- `buffer.h`: Buffer manipulation functions
- `terminal.h`: Terminal command processing
- `motor_control.h`: Motor control interface
- `servo_simple.h`: Servo control interface
- `utils.h`: Utility functions
- `sensor_control.h`, `state_control.h`: Sensor and state control
- `autopilot.h`: Autopilot interface
- `comm_usb.h`, `comm_can.h`: Communication interfaces
- `timeout.h`: Timeout handling
- `log.h`: Logging interface
- `ublox.h`, `m8t_base.h`: GPS interfaces
- `adconv.h`: ADC conversion
- `motor_sim.h`: Motor simulation
- `fi.h`: Field interface
- `hydraulic.h`: Hydraulic control
- `watchdog.h`: Watchdog timer
- `rtcm3_simple.h`: RTCM3 protocol implementation

## Implementation Notes

1. **Packet Size**: Maximum packet size is limited by `PACKET_MAX_PL_LEN`
2. **Endianness**: The protocol uses big-endian for some values (e.g., voltage in CMD_GETANGLE)
3. **Scaling**: Float values in packets are typically scaled (e.g., by 1e4, 1e6) to maintain precision
4. **Checksums**: Some commands include checksums for error detection
5. **Acknowledgments**: Many commands send acknowledgment packets in response
6. **Timeouts**: Most commands reset the system timeout to prevent watchdog reset

## Future Enhancements

Based on commented code and TODOs:
- Heartbeat event handling (currently commented out)
- Complete implementation of all command handlers
- Enhanced error handling and validation
- Improved thread safety for multi-threaded environments
- Better debugging and logging support
