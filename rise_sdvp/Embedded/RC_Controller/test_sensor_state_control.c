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

#include "test_sensor_state_control.h"
#include "sensor_control.h"
#include "state_control.h"
#include "motor_control.h"
#include "commands.h"
#include "conf_general.h"

void test_sensor_state_control_init(void) {
    commands_printf("Starting sensor and state control test...");
    
    // Initialize systems
    sensor_control_init();
    state_control_init();
    
    commands_printf("Systems initialized");
    
    // Test sensor reading
    float sensor_value = sensor_get_activity_value(SENS_STEERING);
    commands_printf("Steering sensor value: %f", sensor_value);
    
    // Test state control configuration
    STATE_CONTROL* control = state_control_get_config(0);
    if (control) {
        commands_printf("State control 0: act=%d, sens=%d, type=%d, target=%f",
                        control->actuator_activity, control->sensor_activity,
                        control->control_type, control->target_value);
    }
    
    // Enable state control
    state_control_set_enabled(0, true);
    state_control_set_target(0, 0.5f);
    
    commands_printf("State control test completed");
}

void test_sensor_state_control_update(void) {
    // This would be called periodically to test the update functionality
    state_control_update();
}