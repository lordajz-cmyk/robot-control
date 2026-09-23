# CAN Communication Module Documentation

## Overview

The `comm_can.c` module implements CAN bus communication for the RC Controller system. It provides the infrastructure for communicating with various CAN devices including VESC motor controllers, IO boards, sensors, and other CAN-enabled peripherals.

## Key Features

- **CAN Bus Initialization**: Configures and starts the CAN interface with appropriate baud rate and settings
- **Dual-Thread Architecture**: Uses separate threads for receiving and processing CAN messages
- **VESC Communication**: Implements the protocol for communicating with Vedder Electronic Speed Controllers (VESC)
- **IO Board Support**: Handles communication with IO boards for ADC reading, angle sensors, and limit switches
- **ADDIO Protocol**: Supports the ADDIO CAN protocol for additional digital IO
- **Proportional Valve Control**: Implements PVG32 proportional valve control via CAN
- **FTR2 Sensor Support**: Handles FTR2 sensor communication
- **CAN Message Forwarding**: Forwards CAN messages to other interfaces (USB, etc.)
- **Status Message Storage**: Stores recent CAN status messages for debugging
- **Thread Safety**: Uses mutexes for thread-safe access to shared resources

## Module Architecture

### Thread Architecture

The module uses a **dual-thread architecture** for efficient CAN communication:

1. **Read Thread** (`cancom_read_thread`):
   - Priority: `NORMALPRIO + 1` (higher priority)
   - Stack size: 512 bytes
   - Responsibility: Receives CAN messages from the bus and stores them in a circular buffer
   - Operation: Waits for CAN receive events, then copies messages to the buffer

2. **Process Thread** (`cancom_process_thread`):
   - Priority: `NORMALPRIO` (normal priority)
   - Stack size: 4096 bytes
   - Responsibility: Processes received messages from the buffer and executes appropriate actions
   - Operation: Continuously processes messages from the circular buffer

### Circular Buffer

CAN messages are stored in a circular buffer for decoupling reception from processing:
- Buffer size: `RX_FRAMES_SIZE` (100 frames)
- Each frame: `CANRxFrame` structure (CAN ID + 8 data bytes + timestamp)
- Read index: `rx_frame_read`
- Write index: `rx_frame_write`

### CAN Configuration

The module supports two CAN configurations based on the `ADDIO` define:

#### Standard Mode (500 KBaud):
```c
CAN_MCR_ABOM | CAN_MCR_AWUM | CAN_MCR_TXFP  // Control register
CAN_BTR_SJW(0) | CAN_BTR_TS2(1) | CAN_BTR_TS1(8) | CAN_BTR_BRP(6)  // Bit timing
```
- **Baud Rate**: 500 KBaud
- **Features**: Automatic bus-off management, automatic wakeup, transmit FIFO priority
- **Timing**: SJW=0, TS2=1, TS1=8, BRP=6

#### ADDIO Mode (512.5 KBaud):
```c
CAN_MCR_ABOM | CAN_MCR_AWUM | CAN_MCR_TXFP
CAN_BTR_SJW(0) | CAN_BTR_TS2(1) | CAN_BTR_TS1(8) | CAN_BTR_BRP(27)
```
- **Baud Rate**: 512.5 KBaud
- **Features**: Same as standard mode
- **Timing**: SJW=0, TS2=1, TS1=8, BRP=27

## Hardware Configuration

### CAN Interface
- **Interface**: CAN1 (defined by `CANDx`)
- **RX Pin**: `CAN1_RX_GPIO`, `CAN1_RX_PIN`
- **TX Pin**: `CAN1_TX_GPIO`, `CAN1_TX_PIN`
- **Alternate Function**: `GPIO_AF_CAN1`

### GPIO Configuration
The module configures the CAN GPIO pins in alternate function mode:
```c
palSetPadMode(CAN1_RX_GPIO, CAN1_RX_PIN, PAL_MODE_ALTERNATE(GPIO_AF_CAN1));
palSetPadMode(CAN1_TX_GPIO, CAN1_TX_PIN, PAL_MODE_ALTERNATE(GPIO_AF_CAN1));
```

## Public API

### Initialization

```c
void comm_can_init(void);
```
Initializes the CAN communication module, including:
- Status message storage initialization
- Circular buffer initialization
- Mutex initialization
- GPIO configuration
- CAN interface startup
- BLDC interface initialization (unless in ADDIO mode)
- Thread creation (read and process threads)

### VESC ID Management

```c
void comm_can_set_vesc_id(int id);
```
Sets the current VESC controller ID.
- `id`: The VESC ID to set (e.g., `VESC_LEFT`, `VESC_RIGHT`, `VESC_ID`)

This function also updates the motor simulation to match the selected VESC.

### VESC Locking

```c
void comm_can_lock_vesc(void);
void comm_can_unlock_vesc(void);
```
These functions provide thread-safe access to VESC resources:
- `comm_can_lock_vesc()`: Locks the VESC mutex
- `comm_can_unlock_vesc()`: Unlocks the VESC mutex

Use these functions to protect critical sections when accessing VESC resources from multiple threads.

### CAN Transmission

```c
void comm_can_transmit(uint32_t id, uint8_t *data, uint8_t len);
```
Transmits a CAN message.
- `id`: CAN message ID (32-bit)
- `data`: Data bytes (up to 8 bytes)
- `len`: Number of data bytes (0-8)

```c
void comm_can_send_buffer(uint8_t *data, unsigned int len, uint32_t id);
```
Sends a buffer as a CAN message.
- `data`: Buffer containing data to send
- `len`: Length of the buffer
- `id`: CAN message ID

### VESC Communication

```c
void comm_can_set_vesc_id(int id);
```
Sets the VESC ID for subsequent communications.

```c
void comm_can_lock_vesc(void);
void comm_can_unlock_vesc(void);
```
Locks/unlocks VESC access for thread safety.

```c
void comm_can_forward_vesc_packet(unsigned char *data, unsigned int len);
```
Forwards a VESC packet to the CAN bus.
- `data`: VESC packet data
- `len`: Length of the packet

### IO Board Communication

```c
void comm_can_io_board_set_pwm(uint8_t pwm, float duty);
```
Sets PWM duty cycle on the IO board.
- `pwm`: PWM channel number
- `duty`: Duty cycle (0.0 to 1.0)

```c
void comm_can_io_board_set_valve(uint8_t valve, float pos);
```
Sets valve position on the IO board.
- `valve`: Valve number
- `pos`: Position (0.0 to 1.0)

```c
void comm_can_io_board_as5047_setangle(float angle);
```
Sets the AS5047 angle sensor value.
- `angle`: Angle in degrees

```c
float comm_can_io_board_get_adc_voltage(uint8_t adc);
```
Gets the ADC voltage from the IO board.
- `adc`: ADC channel number (0-7)
- Returns: Voltage in volts

```c
bool comm_can_io_board_get_lim_sw(uint8_t sw);
```
Gets the limit switch state from the IO board.
- `sw`: Switch number (0-7)
- Returns: Switch state (true = activated, false = not activated)

```c
float comm_can_io_board_get_as5047_angle(void);
```
Gets the AS5047 angle sensor value.
- Returns: Angle in degrees

### ADDIO Communication

```c
void comm_can_addio_set_outputs(uint32_t outputs);
```
Sets ADDIO outputs.
- `outputs`: Output state bitmask

```c
void comm_can_addio_set_pwm(uint8_t pwm, uint16_t duty);
```
Sets ADDIO PWM output.
- `pwm`: PWM channel
- `duty`: Duty cycle (0-65535)

```c
bool comm_can_addio_get_lim_sw(uint8_t sw);
```
Gets ADDIO limit switch state.
- `sw`: Switch number (0-7)
- Returns: Switch state

### Proportional Valve Control (PVG32)

```c
void comm_can_pvg32_set_pos(uint8_t node_id, float pos);
```
Sets PVG32 proportional valve position.
- `node_id`: PVG32 node ID
- `pos`: Position (0.0 to 1.0)

```c
void comm_can_pvg32_set_zero(uint8_t node_id);
```
Sets PVG32 valve to zero position.
- `node_id`: PVG32 node ID

### FTR2 Sensor

```c
void comm_can_ftr2_activate(void);
```
Activates the FTR2 sensor.

```c
float comm_can_ftr2_get_angle(void);
```
Gets the FTR2 sensor angle.
- Returns: Angle in degrees

### Status Messages

```c
void comm_can_store_status(uint32_t id, uint8_t *data, uint8_t len);
```
Stores a CAN status message for debugging.
- `id`: CAN message ID
- `data`: Message data
- `len`: Data length

```c
can_status_msg* comm_can_get_status_msgs(void);
```
Gets the array of stored status messages.
- Returns: Pointer to the status message array

```c
int comm_can_get_status_msgs_stored(void);
```
Gets the number of stored status messages.
- Returns: Number of stored messages

### Callback Registration

```c
void comm_can_set_range_func(void(*func)(uint8_t id, uint8_t dest, float range));
```
Registers a callback for range measurements.
- `func`: Callback function with signature `void func(uint8_t id, uint8_t dest, float range)`

```c
void comm_can_set_dw_ping_func(void(*func)(uint8_t id));
```
Registers a callback for distance sensor ping.
- `func`: Callback function with signature `void func(uint8_t id)`

```c
void comm_can_set_dw_uptime_func(void(*func)(uint8_t id, uint32_t uptime));
```
Registers a callback for distance sensor uptime.
- `func`: Callback function with signature `void func(uint8_t id, uint32_t uptime)`

## CAN Message IDs

### ADDIO CAN IDs
- `CAN_ADDIO_MASTER`: 0x28 - ADDIO master device ID
- `CAN_ANGLE`: 0x19F - Angle sensor CAN ID

### VESC CAN IDs
- `VESC_ID`: Default VESC controller ID
- `VESC_LEFT`: Left motor VESC ID
- `VESC_RIGHT`: Right motor VESC ID

### FTR2 CAN ID
- `ftr2_id`: 0x61F - FTR2 sensor CAN ID

## Data Structures

### CANRxFrame
Standard ChibiOS CAN receive frame structure:
- `IDE`: Identifier Extension bit
- `RTR`: Remote Transmission Request bit
- `DLC`: Data Length Code (0-8)
- `SID`: Standard Identifier (11-bit)
- `EID`: Extended Identifier (18-bit)
- `data`: Data bytes (8 bytes)
- `timestamp`: Reception timestamp

### can_status_msg
Custom structure for storing status messages:
```c
typedef struct {
    uint32_t id;         // CAN message ID
    uint8_t data[8];    // Message data
    uint8_t len;        // Data length
    uint32_t timestamp; // Timestamp
} can_status_msg;
```

## Thread Functions

### Read Thread (`cancom_read_thread`)

**Priority**: NORMALPRIO + 1 (higher than process thread)
**Stack Size**: 512 bytes

**Responsibilities**:
1. Register event listener for CAN receive events
2. Wait for CAN receive events with timeout
3. Receive CAN messages from the bus
4. Store messages in the circular buffer
5. Handle buffer wrap-around

**Operation**:
```c
while(!chThdShouldTerminateX()) {
    if (chEvtWaitAnyTimeout(ALL_EVENTS, MS2ST(10)) == 0) {
        continue; // Timeout, try again
    }
    
    msg_t result = canReceive(&CANDx, CAN_ANY_MAILBOX, &rxmsg, TIME_IMMEDIATE);
    
    while (result == MSG_OK) {
        rx_frames[rx_frame_write++] = rxmsg; // Store in buffer
        if (rx_frame_write == RX_FRAMES_SIZE) {
            rx_frame_write = 0; // Wrap around
        }
        result = canReceive(...); // Get next message
    }
}
```

### Process Thread (`cancom_process_thread`)

**Priority**: NORMALPRIO
**Stack Size**: 4096 bytes

**Responsibilities**:
1. Process messages from the circular buffer
2. Parse CAN messages and execute appropriate actions
3. Handle VESC messages
4. Handle IO board messages
5. Handle ADDIO messages
6. Handle PVG32 messages
7. Handle FTR2 messages
8. Forward messages as needed

**Operation**:
```c
while(!chThdShouldTerminateX()) {
    if (rx_frame_read != rx_frame_write) { // Messages available
        CANRxFrame frame = rx_frames[rx_frame_read++];
        if (rx_frame_read == RX_FRAMES_SIZE) {
            rx_frame_read = 0; // Wrap around
        }
        
        // Process the frame based on CAN ID
        process_can_frame(&frame);
    }
    chThdSleepMilliseconds(1); // Small delay
}
```

## Message Processing

### VESC Message Processing

VESC messages are identified by their CAN IDs and processed accordingly:
- Messages are forwarded to the BLDC interface
- VESC ID filtering is applied
- Motor control commands are executed

### IO Board Message Processing

IO board messages include:
- ADC voltage readings (8 channels)
- Limit switch states (8 switches)
- AS5047 angle sensor data

### ADDIO Message Processing

ADDIO messages include:
- Digital input/output states
- PWM outputs
- Limit switch states

### PVG32 Message Processing

PVG32 proportional valve messages include:
- Position commands
- Status updates
- Zero position commands

### FTR2 Message Processing

FTR2 sensor messages include:
- Angle measurements
- Activation commands
- Status updates

## Wrapper Functions

### send_packet_wrapper

Wrapper function for sending packets via CAN:
```c
static void send_packet_wrapper(unsigned char *data, unsigned int len);
```
- Converts packet data to CAN message format
- Transmits via CAN bus
- Used as callback for BLDC interface

### printf_wrapper

Wrapper function for printf output:
```c
static void printf_wrapper(char *str);
```
- Outputs string via `commands_printf()`
- Used as callback for BLDC interface

## Configuration Options

### ADDIO Mode

The module can be compiled with or without ADDIO support:
- `#define ADDIO`: Enables ADDIO protocol support
- Without ADDIO: Standard CAN configuration (500 KBaud)
- With ADDIO: ADDIO CAN configuration (512.5 KBaud)

### VESC IDs

- `VESC_ID`: Default VESC controller ID
- `VESC_LEFT`: Left motor VESC ID
- `VESC_RIGHT`: Right motor VESC ID

## Debugging Support

The module includes debugging support:
- `iDebug == 43`: Enables debug output for motor selection
- Status message storage for debugging
- Debug output via `commands_printf()`

## Usage Example

```c
// Include the CAN communication module
#include "comm_can.h"

// Initialize CAN communication
void init_can(void) {
    comm_can_init();
    
    // Optionally set VESC ID
    comm_can_set_vesc_id(VESC_LEFT);
    
    // Register callbacks if needed
    comm_can_set_range_func(my_range_callback);
    comm_can_set_dw_ping_func(my_ping_callback);
}

// Send a CAN message
void send_can_message(uint32_t id, uint8_t *data, uint8_t len) {
    comm_can_transmit(id, data, len);
}

// Set IO board PWM
void set_io_pwm(uint8_t pwm, float duty) {
    comm_can_io_board_set_pwm(pwm, duty);
}

// Get IO board ADC voltage
float get_adc_voltage(uint8_t channel) {
    return comm_can_io_board_get_adc_voltage(channel);
}

// Use VESC locking for thread safety
void safe_vesc_operation(void) {
    comm_can_lock_vesc();
    // Critical section
    comm_can_set_vesc_id(VESC_RIGHT);
    // ... other VESC operations
    comm_can_unlock_vesc();
}
```

## Thread Safety

The module includes thread safety mechanisms:

- **CAN Mutex** (`can_mtx`): Protects CAN bus access
- **VESC Mutex** (`vesc_mtx_ext`): Protects VESC resources
- **Circular Buffer**: Thread-safe design with separate read/write indices

For thread-safe operations:
- Use `comm_can_lock_vesc()` and `comm_can_unlock_vesc()` for VESC access
- The circular buffer is designed for single-writer, single-reader access
- External synchronization may be needed for complex multi-threaded scenarios

## Memory Usage

- Read thread stack: 512 bytes
- Process thread stack: 4096 bytes
- CAN frame buffer: 100 * sizeof(CANRxFrame) ≈ 100 * 20 = 2000 bytes
- Status messages: 10 * sizeof(can_status_msg) ≈ 10 * 20 = 200 bytes
- Other variables: ~100 bytes
- Total: ~7000 bytes

## Dependencies

The module depends on:
- `comm_can.h`: Header file with function declarations and CAN IDs
- `ch.h`, `hal.h`: ChibiOS/RT headers for threading and hardware abstraction
- `stm32f4xx_conf.h`: STM32 configuration
- `datatypes.h`: Data type definitions
- `buffer.h`: Buffer manipulation functions
- `crc.h`: CRC calculation functions
- `packet.h`: Packet definitions
- `bldc_interface.h`: BLDC interface for VESC communication
- `commands.h`: Commands interface for printing
- `motor_sim.h`: Motor simulation interface

## Implementation Notes

1. **CAN Interface**: Uses ChibiOS CAN driver (`CAND1`)
2. **Baud Rate**: Configurable via `cancfg` structure
3. **Message IDs**: 32-bit CAN IDs (extended format)
4. **Data Length**: Up to 8 bytes per CAN message
5. **Timestamps**: CAN messages include reception timestamps
6. **Buffering**: Circular buffer prevents message loss during processing
7. **Priorities**: Read thread has higher priority than process thread
8. **Timeouts**: Read thread uses timeout to prevent blocking

## Future Enhancements

Based on commented code and TODOs:
- Terminal command for EID filter (currently commented out)
- Enhanced ADDIO protocol support
- Improved error handling and recovery
- Better debugging and logging support
- Support for additional CAN devices
- Improved thread safety for complex scenarios
