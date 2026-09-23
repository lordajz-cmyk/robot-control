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

#include "steering_control.h"
#include "utils.h"
// #include "watchdog.h"
#include <math.h>

// Control state
static float m_target_angle = 0.0;
static float m_current_angle = 0.0;
static float m_prev_error = 0.0;
static float m_i_term = 0.0;
static float m_d_filter = 0.0;
static float m_dt_int = 0.0;
static STEERING_CONTROL_CONFIG m_config;

void steering_control_init(STEERING_CONTROL_CONFIG *config) {
    m_config = *config;
    m_i_term = 0.0;
    m_prev_error = 0.0;
    m_d_filter = 0.0;
    m_dt_int = 0.0;
}

void steering_control_set_target(float target_angle) {
    m_target_angle = target_angle;
}

float steering_control_get_current(void) {
    return m_current_angle;
}

float steering_control_update(float current_angle) {
	/*
    // Check watchdog state - only allow steering operation if system is operational
    if (system_state != SYSTEM_STATE_OPERATIONAL) {
        // Reset control terms and return 0 output if watchdog indicates unsafe state
        m_i_term = 0.0;
        m_prev_error = 0.0;
        return 0.0;
    }
    */
    
    // Update current angle
    m_current_angle = current_angle;

    // Calculate error
    float error = m_target_angle - m_current_angle;

    // PID control
    float dt = 0.01; // Control loop period (adjust as needed)

    // Proportional term
    float p_term = error * m_config.p_gain;

    // Integral term with wind-up protection
    m_i_term += error * (m_config.i_gain * dt);
    utils_truncate_number_abs(&m_i_term, m_config.max_i_term);

    // Derivative term with filtering
    float d_term = 0.0;
    m_dt_int += dt;

    if (error == m_prev_error) {
        d_term = 0.0;
    } else {
        d_term = (error - m_prev_error) * (m_config.d_gain / m_dt_int);
        m_dt_int = 0.0;
    }

    // Filter D term
    UTILS_LP_FAST(m_d_filter, d_term, m_config.d_filter);
    d_term = m_d_filter;

    // I-term wind-up protection considering P term
    utils_truncate_number_abs(&m_i_term, m_config.max_output - fabsf(p_term));

    // Store previous error
    m_prev_error = error;

    // Calculate output with deadband compensation
    float output = p_term + m_i_term + d_term;
    output += SIGN(output) * m_config.deadband;

    // Limit output
    utils_truncate_number(&output, -m_config.max_output, m_config.max_output);

    return output;
}

void steering_control_reset(void) {
    m_i_term = 0.0;
    m_prev_error = 0.0;
    m_d_filter = 0.0;
    m_dt_int = 0.0;
}
