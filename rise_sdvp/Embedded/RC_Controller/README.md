Below you find information on important files in the folder and what to find where. (in progress..)

The card runs [ChibioOS](https://en.wikipedia.org/wiki/ChibiOS/RT). You can find more information on the operating system at https://www.chibios.org/dokuwiki/doku.php.

A very short summary of the information flow on the card is:
- Upon start 'main' in 'main.c' is execute in order to initiate various processes
- Among these processes it starts to read data from the usb port on the card. The data is processed by 'commands_process_packet' in 'commands.c'. All communication to the card is processed by this function (not all communication to the vehicle reaches the card, for example if a camera is used that is handled directly by the raspberry pi)
- In many cases the processing of the informtion will lead to 'commands_send_packet' (also in 'commands.c') being called to return data via the usb port.

# Main

The main function initiates all threads running on the card. Logging and timeout settings are set directly in the file


# Autopilot

Functions related to the autopilot. The function autopilot_init() is initiated at startup.

*autopilot_init()

Set values to default value. Register the terminal commands below. Does not start any functions.

## Registered terminal commands
| Command | Description | Calls function |
| --- | --- | --- |
| ap_state | Print the state of the autopilot | terminal_state |
| ap_print_closest | Print and plot distance to closest route point. terminal_print_closest |
| ap_dynamic_rad | Enable or disable dynamic radius. | terminal_dynamic_rad | 
| ap_ang_dist_comp | Enable or disable steering angle distance compensation | terminal_angle_dist_comp |
| ap_set_look_ahead | Set the look-ahead distance along the route in points | terminal_look_ahead |


# BLDC interface

Interface with motor control (https://github.com/vedderb/bldc)

# Conf. general

Configuration of vehicle

# Commands

Functions that process commands sent to the machine.

## Important functions

| Function | Description |
| --- | --- |
| commands_init |                 initiate gps |
| commands_process_packet |       process received command |
| commands_printf |               print information in (program internal) terminal |
| commands_forward_vesc_packet |  forward command to VESC |
| comm_usb_send_packet  | Send command to Raspberry Pi (via USB) |

## Messages

| Message   | Description   |   Input (description) (type) | Output (description) (type) |
|--------|---------|---------|---------|
| CMD_TERMINAL_CMD | Terminal command  |  (command) |  |
| CMD_SET_POS | Set vehicle position | (x,y,angle) (float32,float32,float32) |  |
| CMD_SET_ENU_REF | Set local reference | (lat,long,height) (double64, double64,float32) |
| CMD_GET_ENU_REF | Get local reference |   | (lat,long,height) (double64, double64,float32) |
| CMD_AP_ADD_POINTS | Add autopilot point |   (point) (ROUTE_POINT) |  |
| CMD_AP_REMOVE_LAST_POINT | Remove last point |  |  |
| CMD_AP_CLEAR_POINTS | Remove all points |  |  |
| CMD_AP_GET_ROUTE_PART | Get selected points | (first, last) (int, int) | array ROUTE_POINT |
| CMD_AP_REPLACE_ROUTE | Replace current route with another route(?) | array ROUTE_POINT | |
| CMD_AP_SET_ACTIVE   | Set route as active | (routeid) (int) | |
| CMD_SET_MAIN_CONFIG | Set configuration of vehicle | *See separate item* | |
| CMD_GET_MAIN_CONFIG | Get configuration of vehicle |  | *See separate item* |
| CMD_REBOOT_SYSTEM | Reboot the Raspberry Pi |  |  |
| CMD_SET_SYSTEM_TIME | Set time of the Raspberry Pi | data? |  |
| CMD_RC_CONTROL | Set vehicle throttle and steering directly | (throttle, steering) (float32, float32) | | |
| CMD_HYDRAULIC_MOVE | Change state hydraulic implements | (item, state) (int, int) | |
| CMD_GET_STATE  | Get current vehicle state | | *See separate item* |
| CMD_VESC_FWD   | Forward command to VESC | all data | all data |

## Types

### ROUTE_POINT

| Variable   | Type    |
| ---        | ---     |
| px         | float32 |
| py         | float32 |
| pz         | float32 |
| speed      | float32 |
| time       | int32   |
| attributes | uint32  |

# Hydraulic

Starts thread "Hydraulic". hydraulic.init() starts at startup. It only sets default values.

Currently controls the vehicle powertrain via the controller card whilst the hydraulic implements (front/back) is controlled via a bespoke IO-card.

## Important functions

| Function | Description | Input (description) (type) | Output (description) (type) |
| --- | --- | --- | --- |
| THD_FUNCTION |  .. | .. | .. |
| hydraulic_set_speed | Sets vehicles speed (m/s) | (float) (speed) | |
| hydraulic_move | Change state hydraulic implements | (item, state) (int, int) | |

# Pos ([more information](pos.md))

Functions related to calculating the vehicles position. The functions pos_init() and pos_uwb_init() are initiated at startup.

# Timeout

## Important functions
- timeout_reset      Resets the timeout counter. Can be called from anywhere in the project. Is called in commands.c by various functions, most importantly 
- THD_FUNCTION       Stops autopilot unless timeout_reset has been called within a certain set time interval 

## Settings

- m_timeout_msec = 2000
- m_last_update_time = 0
- m_timeout_brake_current = 0.0
- m_has_timeout = false

# Servo simple ([more information](servo_simple.md))

Functions for controling and reading servo information. The servos are controlled by the controller card.

# Utils

Various useful functions

