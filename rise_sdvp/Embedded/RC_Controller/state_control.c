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

#include "state_control.h"
#include "sensor_control.h"
#include "motor_control.h"
#include "conf_general.h"
#include "commands.h"
#include <stdlib.h>
#include <math.h>

// State control system
static STATE_CONTROL active_controls[4];
static PID_Controller pid_controllers[4];
static int active_control_count = 0;

void state_control_init(void) {
    MAIN_CONFIG conf;
    conf_general_read_main_conf(&conf);
    
    active_control_count = 0;
    
    // Load state control configurations from main_config
    for (int i = 0; i < 4; i++) {
        if (i < conf.vehicle.state_controls) {
            active_controls[i] = conf.vehicle.control[i];
            
            // Initialize PID controller for this control loop
            pid_init(&pid_controllers[i], 
                    active_controls[i].kp, 
                    active_controls[i].ki, 
                    active_controls[i].kd,
                    active_controls[i].min_output,
                    active_controls[i].max_output);
            
            active_control_count++;
        } else {
            // Initialize unused controls to safe defaults
            active_controls[i].enabled = false;
            active_controls[i].actuator_activity = ACT_FORWARD;
            active_controls[i].sensor_activity = SENS_FORWARD;
            active_controls[i].control_type = CT_OPEN_LOOP;
            active_controls[i].target_value = 0.0f;
            active_controls[i].kp = active_controls[i].ki = active_controls[i].kd = 0.0f;
            active_controls[i].min_output = -1.0f;
            active_controls[i].max_output = 1.0f;
            
            pid_init(&pid_controllers[i], 0.0f, 0.0f, 0.0f, -1.0f, 1.0f);
        }
    }
}

void state_control_update(void) {
    for (int i = 0; i < active_control_count; i++) {
        if (active_controls[i].enabled) {
            state_control_update_single(&active_controls[i]);
        }
    }
}

void state_control_update_single(STATE_CONTROL* control) {
    // 1. Get current sensor value
    float current_value = sensor_get_activity_value(control->sensor_activity);
    
    // 2. Calculate error
    float error = control->target_value - current_value;
    
    // 3. Get the PID controller for this control loop
    PID_Controller* pid = &pid_controllers[control->control_type]; // Simple mapping for now
    
    // 4. Apply PID control
    float output = pid_update(pid, error, control->control_type);
    
    // For now, apply the control->min_output and control->max_output limits
    if (output < control->min_output) output = control->min_output;
    if (output > control->max_output) output = control->max_output;
    
    // 5. Send to actuators
    int actuator_count = 0;
    ACTUATOR* actuators = motor_get_actuators_by_activity(control->actuator_activity, &actuator_count);
    
    if (actuators && actuator_count > 0) {
        for (int j = 0; j < actuator_count; j++) {
            motor_set_vesc_value(actuators[j].motorid, output, actuators[j].mode);
        }
        free(actuators);
    }
}

void state_control_set_enabled(uint16_t control_index, bool enabled) {
    if (control_index < 4) {
        active_controls[control_index].enabled = enabled;
    }
}

void state_control_set_target(uint16_t control_index, float target) {
    if (control_index < 4) {
        active_controls[control_index].target_value = target;
    }
}

STATE_CONTROL* state_control_get_config(uint16_t control_index) {
    if (control_index < 4) {
        return &active_controls[control_index];
    }
    return NULL;
}

// PID Controller Implementation
void pid_init(PID_Controller* pid, float kp, float ki, float kd, float min, float max) {
    pid->kp = kp;
    pid->ki = ki;
    pid->kd = kd;
    pid->integral = 0.0f;
    pid->prev_error = 0.0f;
    pid->min_output = min;
    pid->max_output = max;
}

float pid_update(PID_Controller* pid, float error, CONTROL_TYPE control_type) {
    float output;
    float derivative;
    
    // Proportional term
    float proportional = pid->kp * error;
    
    // Integral term with anti-windup
    pid->integral += pid->ki * error;
    
    // Derivative term (on error for simplicity, could be on measurement)
    derivative = pid->kd * (error - pid->prev_error);
    
    // Calculate raw output
    output = proportional + pid->integral + derivative;
    
    // Apply output limits and anti-windup
    if (output > pid->max_output) {
        output = pid->max_output;
        // Anti-windup: adjust integral term
        pid->integral = output - proportional - derivative;
    } else if (output < pid->min_output) {
        output = pid->min_output;
        // Anti-windup: adjust integral term
        pid->integral = output - proportional - derivative;
    }
    
    // Store error for next derivative calculation
    pid->prev_error = error;
    
    return output;
}