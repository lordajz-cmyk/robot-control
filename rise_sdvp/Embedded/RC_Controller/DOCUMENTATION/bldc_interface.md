# BLDC Interface Module Documentation

## Overview

The `bldc_interface.c` module provides a communication and control interface for VESC (Vedder Electronic Speed Controller) BLDC (Brushless DC) motors. It implements the protocol for communicating with VESC controllers, handling packet serialization, deserialization, command execution, and data reception.

## Key Features

- **Packet-based Communication**: Implements the VESC communication protocol for sending and receiving packets
- **Multiple Control Modes**: Supports duty cycle, current, RPM, and position control modes
- **Asynchronous Callbacks**: Uses a callback-based architecture for handling received data
- **Motor Parameter Detection**: Supports motor parameter auto-detection
- **Fault Code Handling**: Provides fault code parsing and human-readable error messages
- **Simulation Support**: Includes simulation mode for testing without physical hardware
- **Configuration Management**: Supports retrieving and setting motor and application configurations

## Compatible Firmware Versions

The module is compatible with VESC firmware versions:
- 3.39
- 3.40
- 3.41
- ...
- 3.54

## Module Architecture

### Communication Model

The BLDC interface uses a **callback-based asynchronous communication model**:

1. **Initialization**: Register callbacks for sending packets and processing received data
2. **Command Sending**: Functions construct command packets and send them via the registered send callback
3. **Response Processing**: Received packets are processed by `bldc_interface_process_packet()`, which parses the data and invokes the appropriate registered callbacks
4. **Data Handling**: Callback functions receive parsed data (motor values, firmware version, etc.)

### Callback Architecture

The module uses function pointers to allow flexible integration:

- **Send Callback** (`send_func`): Called to transmit packets to the motor controller
- **Forward Callback** (`forward_func`): Optional callback for forwarding packets (e.g., for logging)
- **Receive Callbacks** (`rx_*_func`): Called when specific data is received from the controller
- **Simulation Callbacks** (`motor_control_set_func`, `values_requested_func`): Used for simulation/testing

## Data Structures

### mc_values

The primary data structure containing motor controller telemetry:

```c
typedef struct {
    float v_in;                // Input voltage (Volts)
    float temp_mos;            // MOSFET temperature (degrees C)
    float temp_motor;          // Motor temperature (degrees C)
    float current_motor;       // Motor current (Amps)
    float current_in;          // Input current (Amps)
    float id;                  // D-axis current (Amps)
    float iq;                  // Q-axis current (Amps)
    float duty_now;            // Current duty cycle (0.0-1.0)
    float rpm;                 // Motor RPM
    float amp_hours;           // Amp-hours consumed
    float amp_hours_charged;   // Amp-hours charged
    float watt_hours;          // Watt-hours consumed
    float watt_hours_charged;  // Watt-hours charged
    int tachometer;           // Tachometer count (relative)
    int tachometer_abs;        // Tachometer count (absolute)
    mc_fault_code fault_code; // Current fault code
    float pid_pos;             // PID position
    uint8_t vesc_id;           // VESC controller ID
} mc_values;
```

### mc_fault_code

Fault codes returned by the motor controller:

| Code | Description |
|------|-------------|
| `FAULT_CODE_NONE` | No fault |
| `FAULT_CODE_OVER_VOLTAGE` | Input voltage too high |
| `FAULT_CODE_UNDER_VOLTAGE` | Input voltage too low |
| `FAULT_CODE_DRV` | DRV8302 driver fault |
| `FAULT_CODE_ABS_OVER_CURRENT` | Absolute over-current |
| `FAULT_CODE_OVER_TEMP_FET` | MOSFET over-temperature |
| `FAULT_CODE_OVER_TEMP_MOTOR` | Motor over-temperature |

### motor_control_mode

Motor control modes:

| Mode | Description |
|------|-------------|
| `MOTOR_CONTROL_DUTY` | Duty cycle control (-1.0 to 1.0) |
| `MOTOR_CONTROL_CURRENT` | Current control (Amps) |
| `MOTOR_CONTROL_CURRENT_BRAKE` | Current brake mode (Amps) |
| `MOTOR_CONTROL_RPM` | RPM control (rotations per minute) |
| `MOTOR_CONTROL_POS` | Position control (degrees) |

## Public API

### Initialization

```c
void bldc_interface_init(void(*func)(unsigned char *data, unsigned int len));
```
Initializes the BLDC interface with a send callback function.
- `func`: Function pointer to the packet send function with signature `void func(unsigned char *data, unsigned int len)`

```c
void bldc_interface_set_forward_func(void(*func)(unsigned char *data, unsigned int len));
```
Sets an optional forwarding callback for packets.
- `func`: Function pointer to the packet forward function

### Packet Transmission

```c
void bldc_interface_send_packet(unsigned char *data, unsigned int len);
```
Sends a packet using the registered send function.
- `data`: Packet data (byte array)
- `len`: Length of the packet in bytes

```c
void bldc_interface_process_packet(unsigned char *data, unsigned int len);
```
Processes a received packet and invokes appropriate callbacks.
- `data`: Received packet data (command ID followed by data)
- `len`: Length of the packet in bytes

### Callback Registration

These functions register callbacks that will be invoked when specific data is received:

```c
void bldc_interface_set_rx_value_func(void(*func)(mc_values *values));
```
Register callback for motor values (temperatures, currents, RPM, etc.)

```c
void bldc_interface_set_rx_printf_func(void(*func)(char *str));
```
Register callback for print/debug messages from controller

```c
void bldc_interface_set_rx_fw_func(void(*func)(int major, int minor));
```
Register callback for firmware version information

```c
void bldc_interface_set_rx_rotor_pos_func(void(*func)(float pos));
```
Register callback for rotor position updates

```c
void bldc_interface_set_rx_detect_func(void(*func)(float cycle_int_limit, float coupling_k,
        const signed char *hall_table, signed char hall_res));
```
Register callback for motor detection results

```c
void bldc_interface_set_rx_dec_ppm_func(void(*func)(float val, float ms));
```
Register callback for decoded PPM signal

```c
void bldc_interface_set_rx_dec_adc_func(void(*func)(float val, float voltage));
```
Register callback for decoded ADC values

```c
void bldc_interface_set_rx_dec_chuk_func(void(*func)(float val));
```
Register callback for decoded Chuck/encoder values

```c
void bldc_interface_set_rx_mcconf_received_func(void(*func)(void));
```
Register callback for motor configuration received confirmation

```c
void bldc_interface_set_rx_appconf_received_func(void(*func)(void));
```
Register callback for application configuration received confirmation

```c
void bldc_interface_set_sim_control_function(void(*func)(motor_control_mode mode, float value));
```
Register callback for motor control in simulation mode

```c
void bldc_interface_set_sim_values_func(void(*func)(void));
```
Register callback for values requested in simulation mode

### Motor Control Setters

These functions send commands to control the motor:

```c
void bldc_interface_terminal_cmd(char* cmd);
```
Send a terminal command string to the motor controller.
- `cmd`: Null-terminated command string

```c
void bldc_interface_set_duty_cycle(float dutyCycle);
```
Set the motor duty cycle.
- `dutyCycle`: Duty cycle value (-1.0 to 1.0, negative = reverse direction)

```c
void bldc_interface_set_current(float current);
```
Set the motor current.
- `current`: Current in Amps (positive or negative for direction)

```c
void bldc_interface_set_current_brake(float current);
```
Set the motor current brake.
- `current`: Braking current in Amps

```c
void bldc_interface_set_rpm(int rpm);
```
Set the motor RPM.
- `rpm`: Target RPM (positive or negative for direction)

```c
void bldc_interface_set_pos(float pos);
```
Set the motor position.
- `pos`: Target position in degrees (0-360 or relative)

```c
void bldc_interface_set_handbrake(float current);
```
Set the handbrake current.
- `current`: Handbrake current in Amps

```c
void bldc_interface_set_servo_pos(float pos);
```
Set the servo position.
- `pos`: Servo position (0.0 to 1.0, representing the pulse width range)

### Data Getters

These functions request data from the motor controller:

```c
void bldc_interface_get_fw_version(void);
```
Request firmware version from the motor controller.

```c
void bldc_interface_get_values(void);
```
Request motor values (temperatures, currents, RPM, voltage, etc.)

```c
void bldc_interface_get_mcconf(void);
```
Request motor configuration from the controller.

```c
void bldc_interface_get_appconf(void);
```
Request application configuration from the controller.

```c
void bldc_interface_get_decoded_ppm(void);
```
Request decoded PPM signal from the controller.

```c
void bldc_interface_get_decoded_adc(void);
```
Request decoded ADC values from the controller.

```c
void bldc_interface_get_decoded_chuk(void);
```
Request decoded Chuck/encoder values from the controller.

### Other Functions

```c
void bldc_interface_detect_motor_param(float current, float min_rpm, float low_duty);
```
Start motor parameter detection.
- `current`: Detection current in Amps
- `min_rpm`: Minimum RPM for detection
- `low_duty`: Low duty cycle for detection

```c
void bldc_interface_reboot(void);
```
Reboot the motor controller.

```c
void bldc_interface_send_alive(void);
```
Send alive/heartbeat signal to check if the controller is responsive.

```c
void send_values_to_receiver(mc_values *values);
```
Send motor values to a receiver (invokes the registered rx_value_func callback).
- `values`: Pointer to mc_values structure

### Helper Functions

```c
const char* bldc_interface_fault_to_string(mc_fault_code fault);
```
Convert a fault code to a human-readable string.
- `fault`: The fault code to convert
- Returns: String describing the fault, or "Unknown fault"

## Communication Protocol

### Packet Format

All communication uses a simple packet format:
- **Command ID**: 1 byte identifying the command type
- **Data**: Variable-length data specific to the command

### Command IDs

The module supports the following command IDs (from datatypes.h):

| Command ID | Name | Description |
|------------|------|-------------|
| 0 | `COMM_FW_VERSION` | Get firmware version |
| 1 | `COMM_JUMP_TO_BOOTLOADER` | Jump to bootloader |
| 2 | `COMM_ERASE_NEW_APP` | Erase new application |
| 3 | `COMM_WRITE_NEW_APP_DATA` | Write new application data |
| 4 | `COMM_GET_VALUES` | Get motor values |
| 5 | `COMM_SET_DUTY` | Set duty cycle |
| 6 | `COMM_SET_CURRENT` | Set current |
| 7 | `COMM_SET_CURRENT_BRAKE` | Set current brake |
| 8 | `COMM_SET_RPM` | Set RPM |
| 9 | `COMM_SET_POS` | Set position |
| 10 | `COMM_SET_SERVO_POS` | Set servo position |
| 11 | `COMM_PRINT` | Print message |
| 12 | `COMM_SAMPLE_PRINT` | Sample print |
| 13 | `COMM_ROTOR_POSITION` | Rotor position |
| ... | ... | ... |
| 27 | `COMM_GET_DECODED_PPM` | Get decoded PPM |
| 28 | `COMM_GET_DECODED_ADC` | Get decoded ADC |
| 29 | `COMM_GET_DECODED_CHUK` | Get decoded Chuck |
| 30 | `COMM_SET_MCCONF` | Set motor configuration |
| 31 | `COMM_SET_APPCONF` | Set application configuration |
| 32 | `COMM_DETECT_MOTOR_PARAM` | Detect motor parameters |
| ... | ... | ... |
| 34 | `COMM_REBOOT` | Reboot controller |
| 35 | `COMM_ALIVE` | Alive/heartbeat |

### Data Scaling

The VESC protocol uses scaling factors for float values to reduce packet size:

- **float16**: 16-bit float, scaled by a factor (e.g., 1e1, 1e2, 1e3)
- **float32**: 32-bit float, scaled by a factor (e.g., 1e1, 1e2, 1e3, 1e6)
- **int32**: 32-bit integer (no scaling)

Example scaling factors:
- Temperature: 1e1 (0.1 degree resolution)
- Current: 1e2 (0.01 Amp resolution)
- Duty cycle: 1e3 (0.001 resolution)
- RPM: 1e0 (1 RPM resolution)
- Position: 1e6 (microdegree resolution)
- PPM/ADC/Chuck: 1e6 (micro-unit resolution)

## Usage Example

```c
// Include the interface
#include "bldc_interface.h"

// Define a callback for sending packets
void my_send_function(unsigned char *data, unsigned int len) {
    // Send data via UART, CAN, USB, etc.
    uart_send(data, len);
}

// Define a callback for received motor values
void my_rx_values_callback(mc_values *values) {
    printf("RPM: %.1f, Current: %.2f A, Temp: %.1f C\n",
           values->rpm, values->current_motor, values->temp_motor);
    
    // Check for faults
    if (values->fault_code != FAULT_CODE_NONE) {
        printf("Fault: %s\n", bldc_interface_fault_to_string(values->fault_code));
    }
}

// Initialize the interface
void init_motor_control(void) {
    // Register send callback
    bldc_interface_init(my_send_function);
    
    // Register receive callback for motor values
    bldc_interface_set_rx_value_func(my_rx_values_callback);
    
    // Optionally register other callbacks
    bldc_interface_set_rx_fw_func(my_fw_callback);
    bldc_interface_set_rx_printf_func(my_print_callback);
}

// Control the motor
void control_motor(void) {
    // Set duty cycle
    bldc_interface_set_duty_cycle(0.5); // 50% duty cycle
    
    // Or set RPM
    bldc_interface_set_rpm(1000); // 1000 RPM
    
    // Or set current
    bldc_interface_set_current(5.0); // 5 Amps
    
    // Request motor values
    bldc_interface_get_values();
    
    // Request firmware version
    bldc_interface_get_fw_version();
}

// Process received data
void on_uart_receive(unsigned char *data, unsigned int len) {
    // Pass received data to the interface for processing
    bldc_interface_process_packet(data, len);
}
```

## Simulation Mode

The module supports a simulation mode for testing without physical hardware:

```c
// Define simulation callbacks
void my_sim_motor_control(motor_control_mode mode, float value) {
    printf("Sim: Mode=%d, Value=%.2f\n", mode, value);
}

void my_sim_values_requested(void) {
    // Simulate motor values
    mc_values sim_values;
    sim_values.rpm = 1000.0;
    sim_values.current_motor = 2.5;
    sim_values.temp_motor = 45.0;
    // ... set other values
    send_values_to_receiver(&sim_values);
}

// Enable simulation mode
bldc_interface_set_sim_control_function(my_sim_motor_control);
bldc_interface_set_sim_values_func(my_sim_values_requested);

// Now commands will use simulation callbacks instead of sending packets
bldc_interface_set_duty_cycle(0.5); // Calls my_sim_motor_control
bldc_interface_get_values(); // Calls my_sim_values_requested
```

## Packet Forwarding

The module supports packet forwarding for logging or debugging:

```c
// Define a forward function
void my_forward_function(unsigned char *data, unsigned int len) {
    // Log the packet
    log_packet(data, len);
    
    // Or forward to another destination
    forward_to_debug_port(data, len);
}

// Enable forwarding
bldc_interface_set_forward_func(my_forward_function);

// Now all received packets will be forwarded before processing
// And all sent packets will be forwarded instead of being sent
```

## Debugging

The module includes debug output for certain operations:

- `iDebug == 43`: Enables debug output for motor control commands (duty cycle, current, RPM)
- Debug output uses `commands_printf()` for printing

Example debug output:
```
bldc action: 0  (MOTOR_CONTROL_DUTY)
bldc DUTY: 0.500000
bldc action: 1  (MOTOR_CONTROL_CURRENT)
bldc CURRENT: 5.000000
```

## Dependencies

The module depends on:
- `bldc_interface.h`: Header file with function declarations
- `buffer.h`: Buffer manipulation functions (buffer_get_float16, buffer_get_float32, buffer_append_float16, buffer_append_float32, buffer_append_int32)
- `datatypes.h`: Data type definitions (mc_values, mc_fault_code, motor_control_mode, COMM_* constants)
- `string.h`: Standard C string functions (strlen, memcpy)

## Implementation Notes

1. **Buffer Management**: Uses a static 1024-byte send buffer for packet construction
2. **Thread Safety**: The module is not inherently thread-safe. If used in a multi-threaded environment, external synchronization is required
3. **Error Handling**: Most functions silently ignore errors (e.g., unregistered callbacks, invalid data lengths)
4. **Scaling**: All float values are scaled according to the VESC protocol specification
5. **Endianness**: The protocol uses little-endian format for multi-byte values
6. **Packet Size**: Maximum packet size is limited by the send buffer (1024 bytes)

## Memory Usage

- Send buffer: 1024 bytes (static)
- Received data storage: ~100 bytes (static variables for values, firmware version, etc.)
- Function pointers: ~50 bytes (12 function pointers)

## Future Enhancements

Based on commented code and TODOs:
- Complete implementation of motor configuration (COMM_GET_MCCONF, COMM_GET_MCCONF_DEFAULT)
- Complete implementation of application configuration (COMM_GET_APPCONF, COMM_GET_APPCONF_DEFAULT)
- Implementation of sample print handling (COMM_SAMPLE_PRINT)
- Implementation of experiment sample handling (COMM_EXPERIMENT_SAMPLE)
- Implementation of various motor detection methods (R/L, flux linkage, encoder, Hall FOC)
- Support for second ADC channel (adc2)
