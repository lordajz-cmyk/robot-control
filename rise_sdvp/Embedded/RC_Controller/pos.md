Functions related to calculating the vehicles position.

The summary is based on that the unit has a [BMI160](https://www.bosch-sensortec.com/products/motion-sensors/imus/bmi160/) (which the F9-card used by Macbot & Drängen has). Though almost all will be true no matter if it hasn't.

Functions called during initiation:
- ahrs_init_attitude_info
- bmi160_wrapper_init (with a sample rate of 500 hz)
- bmi160_wrapper_set_read_callback (with function mpu9150_read)
- bldc_interface_set_rx_value_func (with mc_values_received)
- ublox_set_rx_callback_relposned(with ublox_relposned_rx)
- ublox_set_rx_callback_rawx (with ubx_rx_rawx)
- save_pos_history()

During initiation it also sets a large number of variable to zero and terminal commands list below are initiated.



## Typical workflow

When operating the vehicle at different locations, each with its own base station, the system's functions in pos.c handle this scenario through reinitialization and recalibration processes. Here's how the key functions manage this:

- pos_init:     Initialise:
  -This function initializes the positioning system, setting up necessary parameters and states.
  -Every time the function is called the system (positioning-wise) is reset. Data from prior executions should hence not interfer with the current one

- pos_set_enu_ref:        Setting ENU Reference
  - Sets a new East-North-Up (ENU) coordinate reference based on the current location. At each new location, this function is invoked to define the local coordinate system relative to the new base station. This recalibration aligns the system's positional data with the new environment.

- pos_get_pos:    Retrieving Current position:
  -  Provid provides accurate positional data relative to the new base station.

Operational Workflow:

    At Each New Location:
        System Start: Power on the vehicle, initiating the positioning system.
        Initialization: pos_init is called to reset and prepare the system.
        Set ENU Reference: pos_set_enu_ref establishes the local coordinate system based on the current base station.
        Position Retrieval: pos_get_pos can now be used to obtain accurate positional data in the context of the new location.

By following this sequence, the system effectively manages operations across different locations and base stations, ensuring accurate and reliable positioning data for each session.

## Important functions

| Function | Description | Input (description) (type) | Output (description) (type) |
| --- | --- | --- | --- |
| pos_init |  | | |
| pos_pps_cb | | (EXTDriver*, expchannel_t) (extp, channel) | |
| pos_get_imu | | (float *, float *, float *) (accel, gyro, mag) | |
| pos_get_quaternions | (float *) (q) | |
| pos_get_pos | Get position relative to ENU (POS_STATE *) (p) | |
| pos_get_gps | (GPS_STATE *) (p) | |
| pos_get_speed | | | (float) |
| pos_set_xya | | (float, float, float) (x, y, angle) | |
| pos_set_yaw_offset | | (float) (angle) | |
| pos_set_enu_ref | Setting ENU reference to current position| (double, double, double) (lat, lon, height) | |
| pos_get_enu_ref | Get ENU reference | | (array double) (llh);
| pos_reset_enu_ref | | | |
| pos_get_mc_val | (mc_values *) (v) | |
| pos_get_ms_today | | | int32_t |
| pos_set_ms_today | | (int32_t) (ms) | |
| pos_input_nmea(const char *data) | | (bool) |
| pos_reset_attitude | | | |
| pos_time_since_gps_corr | | | (int) | 
| pos_base_rtcm_obs| | (rtcm_obs_header_t *, rtcm_obs_t*, int) (header, obs, obs_num) | |

### mpu9150_read

This function does quite a lot of processing to handle both imu data


## Registered terminal commands
| Command | Description | Calls function |
| --- | --- | --- |
| pos_delay_info | Print and plot delay information when doing GNSS position correction | cmd_terminal_delay_info |
| pos_gnss_corr_info | Print and plot correction information when doing GNSS position correction | cmd_terminal_gps_corr_info |
| pos_delay_comp | Enable or disable delay compensation | cmd_terminal_delay_comp |
| pos_print_sat_info | Print all satellite information | cmd_terminal_print_sat_info |
| pos_sat_info | Print and plot information about a satellite | cmd_terminal_sat_info |

