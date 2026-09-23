/*
	Copyright 2016-2018 Benjamin Vedder	benjamin@vedder.se

	This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

/*
 * bldc_interface.c
 *
 * BLDC (Brushless DC) Motor Interface Module
 *
 * This module provides communication and control interface for VESC (Vedder Electronic Speed Controller)
 * BLDC motors. It handles packet serialization, deserialization, and command execution.
 *
 * Compatible Firmware Versions:
 * - 3.39
 * - 3.40
 * - ...
 * - 3.54
 *
 * Key Features:
 * - Packet-based communication with VESC controllers
 * - Support for multiple command types (duty cycle, current, RPM, position)
 * - Asynchronous callback-based data reception
 * - Motor parameter detection and configuration
 * - Fault code handling and reporting
 * - Support for both direct control and simulation modes
 *
 * The module uses a callback architecture where received data triggers registered
 * callback functions, allowing for flexible integration with different applications.
 */

#include "bldc_interface.h"
#include "buffer.h"
#include "commands.h"
#include <string.h>

// Private variables
static unsigned char send_buffer[1024]; // Transmit buffer for packet serialization (1024 bytes max)

// Private variables for received data
static mc_values values; // Last received motor controller values (temperatures, currents, RPM, etc.)
static int fw_major; // Firmware major version number
static int fw_minor; // Firmware minor version number
static float rotor_pos; // Current rotor position (radians or degrees, depending on firmware)
static float detect_cycle_int_limit; // Motor detection: cycle integral limit
static float detect_coupling_k; // Motor detection: coupling constant
static signed char detect_hall_table[8]; // Motor detection: Hall sensor table (8 entries)
static signed char detect_hall_res; // Motor detection: Hall sensor resolution
static float dec_ppm; // Decoded PPM signal value
static float dec_ppm_len; // Decoded PPM signal length/pulse width (milliseconds)
static float dec_adc; // Decoded ADC value
static float dec_adc_voltage; // Decoded ADC voltage (volts)
static float dec_chuk; // Decoded Chuck (encoder) value

extern float showData; // External variable for data display (used elsewhere in the system)
extern int iDebug; // External debug flag for conditional debug output

// Private functions
void send_packet_no_fwd(unsigned char *data, unsigned int len); // Send packet without forwarding (internal)

// Function pointers for packet transmission
static void(*send_func)(unsigned char *data, unsigned int len) = 0; // Callback for sending packets
static void(*forward_func)(unsigned char *data, unsigned int len) = 0; // Callback for forwarding packets

// Function pointers for received data
// These callbacks are invoked when corresponding data is received from the motor controller
static void(*rx_value_func)(mc_values *values) = 0; // Callback for motor values (temperatures, currents, RPM)
static void(*rx_printf_func)(char *str) = 0; // Callback for print/debug messages from controller
static void(*rx_fw_func)(int major, int minor) = 0; // Callback for firmware version info
static void(*rx_rotor_pos_func)(float pos) = 0; // Callback for rotor position updates
static void(*rx_detect_func)(float cycle_int_limit, float coupling_k,
		const signed char *hall_table, signed char hall_res) = 0; // Callback for motor detection results
static void(*rx_dec_ppm_func)(float val, float ms) = 0; // Callback for decoded PPM signal
static void(*rx_dec_adc_func)(float val, float voltage) = 0; // Callback for decoded ADC values
static void(*rx_dec_chuk_func)(float val) = 0; // Callback for decoded Chuck/encoder values
static void(*rx_mcconf_received_func)(void) = 0; // Callback when motor configuration is received
static void(*rx_appconf_received_func)(void) = 0; // Callback when application configuration is received

// Function pointers for simulation and control
static void(*motor_control_set_func)(motor_control_mode mode, float value) = 0; // Callback for motor control in simulation mode
static void(*values_requested_func)(void) = 0; // Callback when values are requested (for simulation)

// Initialize the BLDC interface module
// This function sets the callback for sending packets to the motor controller.
//
// @param func Function pointer to the packet send function
//             The function should accept (unsigned char *data, unsigned int len)
void bldc_interface_init(void(*func)(unsigned char *data, unsigned int len)) {
	send_func = func; // Register the send callback
}

// Set the packet forwarding function
// This allows packets to be forwarded to another destination (e.g., for logging or debugging).
//
// @param func Function pointer to the packet forward function
//             The function should accept (unsigned char *data, unsigned int len)
void bldc_interface_set_forward_func(void(*func)(unsigned char *data, unsigned int len)) {
	forward_func = func; // Register the forward callback
}

/**
 * Send a packet using the set send function.
 * This function transmits a packet to the motor controller using the callback registered with bldc_interface_init().
 *
 * @param data
 * The packet data to send (byte array).
 *
 * @param len
 * The length of the data in bytes.
 */
void bldc_interface_send_packet(unsigned char *data, unsigned int len) {
	// Only send if send function is registered
	if (send_func) {
		send_func(data, len); // Invoke the registered send callback
	}
}

/**
 * Process a received buffer with commands and data.
 * This is the main packet processing function that parses incoming data from the motor controller
 * and invokes the appropriate callbacks based on the command ID.
 *
 * @param data
 * The buffer to process (byte array containing command ID followed by data).
 *
 * @param len
 * The length of the buffer in bytes.
 */
void bldc_interface_process_packet(unsigned char *data, unsigned int len) {
	// Return immediately if buffer is empty
	if (!len) {
		return;
	}

	// If forwarding is enabled, forward the packet and return
	if (forward_func) {
		forward_func(data, len);
		return;
	}

	// Parse the packet
	int32_t ind = 0; // Index for parsing data
	int i = 0; // Loop counter
	unsigned char id = data[0]; // Extract command ID from first byte
	data++; // Move to data portion
	len--; // Decrement length (excluding command ID)

	// Process based on command ID
	switch (id) {
		// Firmware version response (COMM_FW_VERSION = 0)
		// Data format: [major_version (1 byte), minor_version (1 byte)]
	case COMM_FW_VERSION:
		if (len == 2) {
			ind = 0;
			fw_major = data[ind++]; // Extract major version
			fw_minor = data[ind++]; // Extract minor version
			// Note: Callback is invoked elsewhere when firmware version is requested
		} else {
			// Invalid length, set to error values
			fw_major = -1;
			fw_minor = -1;
		}
		break;

	case COMM_ERASE_NEW_APP:
	case COMM_WRITE_NEW_APP_DATA:
		// TODO
		break;

		// Motor values response (COMM_GET_VALUES)
		// Contains comprehensive motor controller telemetry data
		// Data format: Multiple float16/float32/int32 values in sequence
	case COMM_GET_VALUES:
		ind = 0;
		// Parse all motor values from the buffer
		values.temp_mos = buffer_get_float16(data, 1e1, &ind); // MOSFET temperature (degrees C)
		values.temp_motor = buffer_get_float16(data, 1e1, &ind); // Motor temperature (degrees C)
		values.current_motor = buffer_get_float32(data, 1e2, &ind); // Motor current (Amps)
		values.current_in = buffer_get_float32(data, 1e2, &ind); // Input current (Amps)
		values.id = buffer_get_float32(data, 1e2, &ind); // D-axis current (Amps)
		values.iq = buffer_get_float32(data, 1e2, &ind); // Q-axis current (Amps)
		values.duty_now = buffer_get_float16(data, 1e3, &ind); // Current duty cycle (0.0-1.0)
		values.rpm = buffer_get_float32(data, 1e0, &ind); // Motor RPM
		values.v_in = buffer_get_float16(data, 1e1, &ind); // Input voltage (Volts)
		values.amp_hours = buffer_get_float32(data, 1e4, &ind); // Amp-hours consumed
		values.amp_hours_charged = buffer_get_float32(data, 1e4, &ind); // Amp-hours charged
		values.watt_hours = buffer_get_float32(data, 1e4, &ind); // Watt-hours consumed
		values.watt_hours_charged = buffer_get_float32(data, 1e4, &ind); // Watt-hours charged
		values.tachometer = buffer_get_int32(data, &ind); // Tachometer count (relative)
		values.tachometer_abs = buffer_get_int32(data, &ind); // Tachometer count (absolute)
		values.fault_code = (mc_fault_code)data[ind++]; // Fault code (mc_fault_code enum)

		// Parse optional fields (may not be present in all firmware versions)
		if (ind < (int)len) {
			values.pid_pos = buffer_get_float32(data, 1e6, &ind); // PID position
		} else {
			values.pid_pos = 0.0;
		}

		if (ind < (int)len) {
			values.vesc_id = data[ind++]; // VESC controller ID
		} else {
			values.vesc_id = 255; // Default/invalid ID
		}

		// Invoke callback if registered
		if (rx_value_func) {
			rx_value_func(&values); // Pass parsed values to callback
		}
		break;

		// Print message from controller (COMM_PRINT)
		// Data format: Null-terminated string
	case COMM_PRINT:
		if (rx_printf_func) {
			data[len] = '\0'; // Null-terminate the string
			rx_printf_func((char*)data); // Pass to print callback
		}
		break;

		// Sample print (COMM_SAMPLE_PRINT)
		// Currently not implemented
	case COMM_SAMPLE_PRINT:
		// TODO: Parse and handle sample print data
		break;

		// Rotor position update (COMM_ROTOR_POSITION)
		// Data format: [position (float32, scaled by 100000)]
	case COMM_ROTOR_POSITION:
		ind = 0;
		rotor_pos = buffer_get_float32(data, 100000.0, &ind); // Parse rotor position

		if (rx_rotor_pos_func) {
			rx_rotor_pos_func(rotor_pos); // Invoke rotor position callback
		}
		break;

	case COMM_EXPERIMENT_SAMPLE:
		// TODO
		break;

		// Motor configuration (COMM_GET_MCCONF, COMM_GET_MCCONF_DEFAULT)
		// Currently not fully implemented
	case COMM_GET_MCCONF:
	case COMM_GET_MCCONF_DEFAULT:
		// TODO: Parse and store motor configuration
		// Commented out: Callback for motor configuration
//		if (rx_mcconf_func) {
//			rx_mcconf_func(&mcconf);
//		}
		break;

		// Application configuration (COMM_GET_APPCONF, COMM_GET_APPCONF_DEFAULT)
		// Currently not fully implemented
	case COMM_GET_APPCONF:
	case COMM_GET_APPCONF_DEFAULT:
		// TODO: Parse and store application configuration
		// Commented out: Callback for application configuration
//		if (rx_appconf_func) {
//			rx_appconf_func(&appconf);
//		}
		break;

		// Motor parameter detection result (COMM_DETECT_MOTOR_PARAM)
		// Data format: [cycle_int_limit (float32), coupling_k (float32), hall_table[8] (bytes), hall_res (byte)]
	case COMM_DETECT_MOTOR_PARAM:
		ind = 0;
		// Parse motor detection parameters
		detect_cycle_int_limit = buffer_get_float32(data, 1000.0, &ind); // Cycle integral limit
		detect_coupling_k = buffer_get_float32(data, 1000.0, &ind); // Coupling constant
		
		// Parse Hall sensor table (8 entries)
		for (i = 0;i < 8;i++) {
			detect_hall_table[i] = (const signed char)(data[ind++]);
		}
		detect_hall_res = (const signed char)(data[ind++]); // Hall sensor resolution

		// Invoke detection callback if registered
		if (rx_detect_func) {
			rx_detect_func(detect_cycle_int_limit, detect_coupling_k,
					detect_hall_table, detect_hall_res);
		}
		break;

		// Motor detection: R/L (COMM_DETECT_MOTOR_R_L)
		// Currently not implemented
	case COMM_DETECT_MOTOR_R_L: {
		// TODO: Implement motor resistance/inductance detection handling
	} break;

		// Motor detection: Flux linkage (COMM_DETECT_MOTOR_FLUX_LINKAGE)
		// Currently not implemented
	case COMM_DETECT_MOTOR_FLUX_LINKAGE: {
		// TODO: Implement motor flux linkage detection handling
	} break;

		// Encoder detection (COMM_DETECT_ENCODER)
		// Currently not implemented
	case COMM_DETECT_ENCODER: {
		// TODO: Implement encoder detection handling
	} break;

		// Hall sensor FOC detection (COMM_DETECT_HALL_FOC)
		// Currently not implemented
	case COMM_DETECT_HALL_FOC: {
		// TODO: Implement Hall sensor FOC detection handling
	} break;

		// Decoded PPM signal (COMM_GET_DECODED_PPM)
		// Data format: [value (float32), length (float32)]
	case COMM_GET_DECODED_PPM:
		ind = 0;
		dec_ppm = buffer_get_float32(data, 1000000.0, &ind); // PPM value (scaled by 1e6)
		dec_ppm_len = buffer_get_float32(data, 1000000.0, &ind); // PPM pulse length (microseconds)

		if (rx_dec_ppm_func) {
			rx_dec_ppm_func(dec_ppm, dec_ppm_len); // Invoke PPM callback
		}
		break;

		// Decoded ADC value (COMM_GET_DECODED_ADC)
		// Data format: [value (float32), voltage (float32)]
	case COMM_GET_DECODED_ADC:
		ind = 0;
		dec_adc = buffer_get_float32(data, 1000000.0, &ind); // ADC value (scaled by 1e6)
		dec_adc_voltage = buffer_get_float32(data, 1000000.0, &ind); // ADC voltage (scaled by 1e6)
		// Note: TODO for adc2 (second ADC channel)

		if (rx_dec_adc_func) {
			rx_dec_adc_func(dec_adc, dec_adc_voltage); // Invoke ADC callback
		}
		break;

		// Decoded Chuck/encoder value (COMM_GET_DECODED_CHUK)
		// Data format: [value (float32)]
	case COMM_GET_DECODED_CHUK:
		ind = 0;
		dec_chuk = buffer_get_float32(data, 1000000.0, &ind); // Chuck value (scaled by 1e6)

		if (rx_dec_chuk_func) {
			rx_dec_chuk_func(dec_chuk); // Invoke Chuck callback
		}
		break;

		// Motor configuration set confirmation (COMM_SET_MCCONF)
		// This is a confirmation that the new motor configuration was received by the controller
	case COMM_SET_MCCONF:
		// This is a confirmation that the new mcconf is received.
		if (rx_mcconf_received_func) {
			rx_mcconf_received_func(); // Invoke configuration received callback
		}
		break;

		// Application configuration set confirmation (COMM_SET_APPCONF)
		// This is a confirmation that the new application configuration was received by the controller
	case COMM_SET_APPCONF:
		// This is a confirmation that the new appconf is received.
		if (rx_appconf_received_func) {
			rx_appconf_received_func(); // Invoke app configuration received callback
		}
		break;

		// Unknown/unhandled command
		default:
		// Ignore unknown commands
		break;
	}
}

/**
 * Function pointer setters. When data that is requested with the get functions
 * is received, the corresponding function pointer will be called with the
 * received data.
 *
 * These functions register callbacks that will be invoked when specific data
 * is received from the motor controller.
 *
 * @param func
 * A function to be called when the corresponding data is received.
 */

// Set callback for motor values (temperatures, currents, RPM, etc.)
// @param func Callback function with signature: void func(mc_values *values)
void bldc_interface_set_rx_value_func(void(*func)(mc_values *values)) {
	rx_value_func = func;
}

// Set callback for print/debug messages from controller
// @param func Callback function with signature: void func(char *str)
void bldc_interface_set_rx_printf_func(void(*func)(char *str)) {
	rx_printf_func = func;
}

// Set callback for firmware version information
// @param func Callback function with signature: void func(int major, int minor)
void bldc_interface_set_rx_fw_func(void(*func)(int major, int minor)) {
	rx_fw_func = func;
}

// Set callback for rotor position updates
// @param func Callback function with signature: void func(float pos)
void bldc_interface_set_rx_rotor_pos_func(void(*func)(float pos)) {
	rx_rotor_pos_func = func;
}

// Set callback for motor detection results
// @param func Callback function with signature: void func(float cycle_int_limit, float coupling_k,
//                                                        const signed char *hall_table, signed char hall_res)
void bldc_interface_set_rx_detect_func(void(*func)(float cycle_int_limit, float coupling_k,
		const signed char *hall_table, signed char hall_res)) {
	rx_detect_func = func;
}

// Set callback for decoded PPM signal
// @param func Callback function with signature: void func(float val, float ms)
void bldc_interface_set_rx_dec_ppm_func(void(*func)(float val, float ms)) {
	rx_dec_ppm_func = func;
}

// Set callback for decoded ADC values
// @param func Callback function with signature: void func(float val, float voltage)
void bldc_interface_set_rx_dec_adc_func(void(*func)(float val, float voltage)) {
	rx_dec_adc_func = func;
}

// Set callback for decoded Chuck/encoder values
// @param func Callback function with signature: void func(float val)
void bldc_interface_set_rx_dec_chuk_func(void(*func)(float val)) {
	rx_dec_chuk_func = func;
}

// Set callback for motor configuration received confirmation
// @param func Callback function with signature: void func(void)
void bldc_interface_set_rx_mcconf_received_func(void(*func)(void)) {
	rx_mcconf_received_func = func;
}

// Set callback for application configuration received confirmation
// @param func Callback function with signature: void func(void)
void bldc_interface_set_rx_appconf_received_func(void(*func)(void)) {
	rx_appconf_received_func = func;
}

// Set callback for motor control in simulation mode
// @param func Callback function with signature: void func(motor_control_mode mode, float value)
void bldc_interface_set_sim_control_function(void(*func)(motor_control_mode mode, float value)) {
	motor_control_set_func = func;
}

// Set callback for values requested in simulation mode
// @param func Callback function with signature: void func(void)
void bldc_interface_set_sim_values_func(void(*func)(void)) {
	values_requested_func = func;
}

// Setters
// These functions send commands to the motor controller to set various parameters.
// They construct the appropriate command packet and send it via the registered send function.

// Send a terminal command to the motor controller
// This allows sending arbitrary text commands to the VESC controller's terminal interface.
//
// @param cmd Null-terminated command string to send
void bldc_interface_terminal_cmd(char* cmd) {
	int len = strlen(cmd);
	send_buffer[0] = COMM_TERMINAL_CMD; // Command ID for terminal command
	memcpy(send_buffer + 1, cmd, len); // Copy command string to buffer
	send_packet_no_fwd(send_buffer, len + 1); // Send packet (command ID + string)
}

// Set the motor duty cycle
// Duty cycle controls the PWM output to the motor (0.0 = off, 1.0 = full power).
//
// @param dutyCycle Duty cycle value (-1.0 to 1.0, where negative = reverse direction)
void bldc_interface_set_duty_cycle(float dutyCycle) {
	// Debug output for duty cycle commands
	if (iDebug==43)
	{
		commands_printf("bldc action: %i",MOTOR_CONTROL_DUTY);
		commands_printf("bldc DUTY: %f",dutyCycle);
	}
	
	// If simulation mode is enabled, use the simulation callback
	if (motor_control_set_func) {
		motor_control_set_func(MOTOR_CONTROL_DUTY, dutyCycle);
		return; // Skip actual packet sending in simulation mode
	}
	
	// Construct packet for COMM_SET_DUTY command
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_SET_DUTY; // Command ID
	buffer_append_float32(send_buffer, dutyCycle, 100000.0, &send_index); // Append duty cycle (scaled by 1e5)
	send_packet_no_fwd(send_buffer, send_index); // Send the packet
}

// Set the motor current
// Current control mode regulates the motor current directly.
//
// @param current Current in Amps (positive or negative for direction)
void bldc_interface_set_current(float current) {
	// Debug output for current commands
	if (iDebug==43)
	{
		commands_printf("bldc action: %i",MOTOR_CONTROL_CURRENT);
		commands_printf("bldc CURRENT: %f",current);
	}
	
	// If simulation mode is enabled, use the simulation callback
	if (motor_control_set_func) {
		motor_control_set_func(MOTOR_CONTROL_CURRENT, current);
		return; // Skip actual packet sending in simulation mode
	}
	
	// Construct packet for COMM_SET_CURRENT command
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_SET_CURRENT; // Command ID
	buffer_append_float32(send_buffer, current, 1000.0, &send_index); // Append current (scaled by 1e3)
	send_packet_no_fwd(send_buffer, send_index); // Send the packet
}

// Set the motor current brake
// Current brake mode uses current to brake/hold the motor.
//
// @param current Braking current in Amps
void bldc_interface_set_current_brake(float current) {
	// Debug output for current brake commands
	if (iDebug==43)
	{
		commands_printf("bldc action: %i",MOTOR_CONTROL_CURRENT);
		commands_printf("bldc CURRENT BRAKE: %f",current);
	}
	
	// If simulation mode is enabled, use the simulation callback
	if (motor_control_set_func) {
		motor_control_set_func(MOTOR_CONTROL_CURRENT_BRAKE, current);
		return; // Skip actual packet sending in simulation mode
	}
	
	// Construct packet for COMM_SET_CURRENT_BRAKE command
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_SET_CURRENT_BRAKE; // Command ID
	buffer_append_float32(send_buffer, current, 1000.0, &send_index); // Append current (scaled by 1e3)
	send_packet_no_fwd(send_buffer, send_index); // Send the packet
}

// Set the motor RPM
// RPM control mode regulates the motor speed in rotations per minute.
//
// @param rpm Target RPM (positive or negative for direction)
void bldc_interface_set_rpm(int rpm) {
	// Debug output for RPM commands
	if (iDebug==43)
	{
		commands_printf("bldc action: %i",MOTOR_CONTROL_CURRENT);
		commands_printf("bldc RPM: %f",rpm);
	}
	
	// If simulation mode is enabled, use the simulation callback
	if (motor_control_set_func) {
		motor_control_set_func(MOTOR_CONTROL_RPM, rpm);
		return; // Skip actual packet sending in simulation mode
	}
	
	// Construct packet for COMM_SET_RPM command
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_SET_RPM; // Command ID
	buffer_append_int32(send_buffer, rpm, &send_index); // Append RPM (as integer)
	send_packet_no_fwd(send_buffer, send_index); // Send the packet
}

// Set the motor position
// Position control mode moves the motor to a specific angular position.
//
// @param pos Target position in degrees (0-360 or relative)
void bldc_interface_set_pos(float pos) {
	// If simulation mode is enabled, use the simulation callback
	if (motor_control_set_func) {
		motor_control_set_func(MOTOR_CONTROL_POS, pos);
		return; // Skip actual packet sending in simulation mode
	}
	
	// Construct packet for COMM_SET_POS command
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_SET_POS; // Command ID
	buffer_append_float32(send_buffer, pos, 1000000.0, &send_index); // Append position (scaled by 1e6)
	send_packet_no_fwd(send_buffer, send_index); // Send the packet
}

// Set the handbrake current
// Handbrake mode uses current to hold the motor stationary (braking).
//
// @param current Handbrake current in Amps
void bldc_interface_set_handbrake(float current) {
	// Construct packet for COMM_SET_HANDBRAKE command
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_SET_HANDBRAKE; // Command ID
	buffer_append_float32(send_buffer, current, 1e3, &send_index); // Append current (scaled by 1e3)
	send_packet_no_fwd(send_buffer, send_index); // Send the packet
}

// Set the servo position
// Controls a servo motor connected to the controller.
//
// @param pos Servo position (0.0 to 1.0, representing the pulse width range)
void bldc_interface_set_servo_pos(float pos) {
	// Construct packet for COMM_SET_SERVO_POS command
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_SET_SERVO_POS; // Command ID
	buffer_append_float16(send_buffer, pos, 1000.0, &send_index); // Append position (scaled by 1e3)
	send_packet_no_fwd(send_buffer, send_index); // Send the packet
}

// Getters
// These functions request data from the motor controller.
// They send the appropriate request packet and the response will be processed
// by the packet processing function, which invokes the registered callbacks.

// Request firmware version from the motor controller
// Response will be processed and fw_major/fw_minor will be updated
void bldc_interface_get_fw_version(void) {
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_FW_VERSION; // Command ID
	send_packet_no_fwd(send_buffer, send_index); // Send request
}

// Request motor values from the controller
// Response contains temperatures, currents, RPM, voltage, etc.
void bldc_interface_get_values(void) {
	// If simulation mode is enabled, use the simulation callback
	if (values_requested_func) {
		values_requested_func();
		return; // Skip actual packet sending in simulation mode
	}
	
	// Construct and send request packet
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_GET_VALUES; // Command ID
	send_packet_no_fwd(send_buffer, send_index); // Send request
}

// Request motor configuration from the controller
void bldc_interface_get_mcconf(void) {
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_GET_MCCONF; // Command ID
	send_packet_no_fwd(send_buffer, send_index); // Send request
}

// Request application configuration from the controller
void bldc_interface_get_appconf(void) {
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_GET_APPCONF; // Command ID
	send_packet_no_fwd(send_buffer, send_index); // Send request
}

// Request decoded PPM signal from the controller
void bldc_interface_get_decoded_ppm(void) {
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_GET_DECODED_PPM; // Command ID
	send_packet_no_fwd(send_buffer, send_index); // Send request
}

// Request decoded ADC values from the controller
void bldc_interface_get_decoded_adc(void) {
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_GET_DECODED_ADC; // Command ID
	send_packet_no_fwd(send_buffer, send_index); // Send request
}

// Request decoded Chuck/encoder values from the controller
void bldc_interface_get_decoded_chuk(void) {
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_GET_DECODED_CHUK; // Command ID
	send_packet_no_fwd(send_buffer, send_index); // Send request
}

// Other functions

// Start motor parameter detection
// This command initiates the motor parameter detection process on the controller.
// The controller will measure motor resistance, inductance, and other parameters.
//
// @param current Detection current in Amps
// @param min_rpm Minimum RPM for detection
// @param low_duty Low duty cycle for detection
void bldc_interface_detect_motor_param(float current, float min_rpm, float low_duty) {
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_DETECT_MOTOR_PARAM; // Command ID
	buffer_append_float32(send_buffer, current, 1000.0, &send_index); // Detection current (scaled by 1e3)
	buffer_append_float32(send_buffer, min_rpm, 1000.0, &send_index); // Minimum RPM (scaled by 1e3)
	buffer_append_float32(send_buffer, low_duty, 1000.0, &send_index); // Low duty cycle (scaled by 1e3)
	send_packet_no_fwd(send_buffer, send_index); // Send the detection command
}

// Reboot the motor controller
// This command causes the VESC controller to reboot.
void bldc_interface_reboot(void) {
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_REBOOT; // Command ID
	send_packet_no_fwd(send_buffer, send_index); // Send reboot command
}

// Send alive/heartbeat signal to the motor controller
// This command is used to check if the controller is responsive.
void bldc_interface_send_alive(void) {
	int32_t send_index = 0;
	send_buffer[send_index++] = COMM_ALIVE; // Command ID
	send_packet_no_fwd(send_buffer, send_index); // Send alive/heartbeat
}

// Send motor values to a receiver
// This function passes motor values to the registered callback, if any.
//
// @param values Pointer to mc_values structure containing motor data
void send_values_to_receiver(mc_values *values) {
	// Invoke the registered callback if present
	if (rx_value_func) {
		rx_value_func(values); // Pass values to callback
	}
}

// Helpers

// Convert a fault code to a human-readable string
// This function is useful for debugging and error reporting.
//
// @param fault The fault code to convert (mc_fault_code enum)
// @return A string describing the fault, or "Unknown fault" if the code is not recognized
const char* bldc_interface_fault_to_string(mc_fault_code fault) {
	switch (fault) {
	case FAULT_CODE_NONE: return "FAULT_CODE_NONE"; // No fault
	case FAULT_CODE_OVER_VOLTAGE: return "FAULT_CODE_OVER_VOLTAGE"; // Input voltage too high
	case FAULT_CODE_UNDER_VOLTAGE: return "FAULT_CODE_UNDER_VOLTAGE"; // Input voltage too low
	case FAULT_CODE_DRV: return "FAULT_CODE_DRV"; // DRV8302 driver fault
	case FAULT_CODE_ABS_OVER_CURRENT: return "FAULT_CODE_ABS_OVER_CURRENT"; // Absolute over-current
	case FAULT_CODE_OVER_TEMP_FET: return "FAULT_CODE_OVER_TEMP_FET"; // MOSFET over-temperature
	case FAULT_CODE_OVER_TEMP_MOTOR: return "FAULT_CODE_OVER_TEMP_MOTOR"; // Motor over-temperature
	default: return "Unknown fault"; // Unknown fault code
	}
}

// Private functions

// Send a packet without forwarding (internal function)
// This function sends a packet using the registered send function, but only if
// forwarding is not enabled. If forwarding is enabled, packets are only forwarded.
//
// @param data The packet data to send
// @param len The length of the packet
void send_packet_no_fwd(unsigned char *data, unsigned int len) {
	// Only send if forwarding is not enabled
	if (!forward_func) {
		bldc_interface_send_packet(data, len); // Use the registered send function
	}
	// If forward_func is set, the packet will be forwarded by bldc_interface_send_packet
}
