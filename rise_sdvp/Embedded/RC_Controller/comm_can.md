Functions related to calculating the vehicles position.

Functions for controling and reading servo information. The servos are controlled by the controller card.

The summary is based on that the unit has a [BMI160](https://www.bosch-sensortec.com/products/motion-sensors/imus/bmi160/) (which the F9-card used by Macbot & Drängen has). Though almost all will be true no matter if it hasn't.

Activity during initiation:

Positions are set to 0.5 (in the middle / neutral). This is to avoid unintentional movement during start up. Various other settings related to the signal transmission are also set.

## Thread

Servo ramp

## Important functions

| Function | Description | Input (description) (type) | Output (description) (type) |
| --- | --- | --- | --- |
| comm_can_init | Initiates CAN | | |
| comm_can_set_vesc_id | Sets unit (VESC) to read from/write to | (int) (id) | |
| comm_can_lock_vesc | ? | | |
| comm_can_unlock_vesc | ? | | |
| comm_can_transmit_eid |  | (uint32_t, uint8_t*, uint8_t) (id, data, len) | |
| comm_can_transmit_sid |  | (uint32_t, uint8_t*, uint8_t) (id, data, len) | |
| comm_can_send_buffer |  | (uint8_t, uint8_t*, unsigned int, bool) (controller_id, data, len, send) | |

void comm_can_dw_range(uint8_t id, uint8_t dest, int samples);
void comm_can_dw_ping(uint8_t id);
void comm_can_dw_reboot(uint8_t id);
void comm_can_dw_get_uptime(uint8_t id);
void comm_can_set_range_func(void(*func)(uint8_t id, uint8_t dest, float range));
void comm_can_set_dw_ping_func(void(*func)(uint8_t id));
void comm_can_set_dw_uptime_func(void(*func)(uint8_t id, uint32_t uptime));
float comm_can_io_board_adc_voltage(int ch);
float comm_can_io_board_as5047_angle(void);
bool comm_can_io_board_lim_sw(int sw);
ADC_CNT_t* comm_can_io_board_adc0_cnt(void);
void comm_can_io_board_set_valve(int board, int valve, bool set);
void comm_can_io_board_set_pwm_duty(int board, float duty);


## Important variables

- main_config.vehicle.steering_ramp_time
- TIM_CLOCK
- RAMP_LOOP_HZ

## Registered terminal commands

None


