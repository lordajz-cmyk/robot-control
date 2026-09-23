/*
	Copyright 2026 Gunnar Larsson

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

#ifndef STATE_CONTROL_H
#define STATE_CONTROL_H

#include "datatypes.h"

// PID Controller structure
typedef struct {
    float integral;
    float prev_error;
    float kp, ki, kd;
    float min_output;
    float max_output;
} PID_Controller;

// State control functions
void state_control_init(void);
void state_control_update(void);
void state_control_update_single(STATE_CONTROL* control);
void state_control_set_enabled(uint16_t control_index, bool enabled);
void state_control_set_target(uint16_t control_index, float target);
STATE_CONTROL* state_control_get_config(uint16_t control_index);

// PID control functions
void pid_init(PID_Controller* pid, float kp, float ki, float kd, float min, float max);
float pid_update(PID_Controller* pid, float error, CONTROL_TYPE control_type);

#endif /* STATE_CONTROL_H */