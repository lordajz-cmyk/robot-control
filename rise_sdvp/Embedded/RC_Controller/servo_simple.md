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
| servo_simple_init |  | | |
| servo_simple_set_pos | (float) (pos) | | |
| servo_simple_set_pos_ramp | (float) (pos) | | |
| servo_simple_get_pos_now |  | (float) | |
| servo_simple_get_pos_set |  | (float) | |


## Important variables

- main_config.vehicle.steering_ramp_time
- TIM_CLOCK
- RAMP_LOOP_HZ

## Registered terminal commands

None

