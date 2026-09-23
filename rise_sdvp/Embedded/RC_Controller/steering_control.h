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

#ifndef STEERING_CONTROL_H
#define STEERING_CONTROL_H

#include "datatypes.h"

// Steering control configuration
typedef struct {
    float p_gain;        // Proportional gain
    float i_gain;        // Integral gain
    float d_gain;        // Derivative gain
    float d_filter;      // Derivative filter constant
    float max_output;    // Maximum output value
    float deadband;      // Deadband compensation
    float max_i_term;     // Maximum integral term
} STEERING_CONTROL_CONFIG;

// Initialize steering control
void steering_control_init(STEERING_CONTROL_CONFIG *config);

// Set desired steering angle
void steering_control_set_target(float target_angle);

// Get current steering angle (for monitoring)
float steering_control_get_current(void);

// Update control loop with current angle feedback
// Returns the control output to apply to the actuator
float steering_control_update(float current_angle);

// Reset integral term (for fault conditions)
void steering_control_reset(void);

#endif /* STEERING_CONTROL_H */