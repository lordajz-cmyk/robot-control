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

#include "sensor_control.h"
#include "conf_general.h"
#include "commands.h"
#include <stdlib.h>
#include <string.h>

// Current sensor values storage
static float sensor_values[16] = {0}; // Support up to 16 sensors

extern float last_sensorvalue;
extern float frontangle;
extern float io_board_as5047_angle;

void sensor_control_init(void) {
    // Initialize sensor values to 0
    for (int i = 0; i < 16; i++) {
        sensor_values[i] = 0.0f;
    }
    
    // Initialize known sensor values
    // Sensor ID 0: Front angle sensor (existing)
    sensor_values[0] = frontangle;
    
    // Sensor ID 1: IO board AS5047 angle sensor (existing)
    sensor_values[1] = io_board_as5047_angle;
}

SENSOR* sensor_get_sensors_by_activity(uint16_t activity, int* count) {
    MAIN_CONFIG conf;
    conf_general_read_main_conf(&conf);
    
    // Initialize count to 0
    *count = 0;
    
    // Loop through all sensors in the configuration
    for (int i = 0; i < conf.vehicle.sensors; i++) {
        if (conf.vehicle.sensor[i].activity == activity) {
            (*count)++;
        }
    }
    
    // If no sensors found, return NULL
    if (*count == 0) {
        return NULL;
    }
    
    // Allocate memory for the result (this should be freed by the caller)
    SENSOR* result = (SENSOR*)malloc(*count * sizeof(SENSOR));
    if (result == NULL) {
        *count = 0;
        return NULL;
    }
    
    // Copy matching sensors to the result array
    int result_index = 0;
    for (int i = 0; i < conf.vehicle.sensors; i++) {
        if (conf.vehicle.sensor[i].activity == activity) {
            result[result_index] = conf.vehicle.sensor[i];
            result_index++;
        }
    }
    
    return result;
}

float sensor_read_value(uint16_t sensorid, SENSOR_TYPE type) {
    // For now, handle known sensors
    switch (sensorid) {
        case 0: // Front angle sensor
            return frontangle;
        case 1: // IO board AS5047 angle sensor
            return io_board_as5047_angle;
        case 2: // Last sensor value (voltage)
            return last_sensorvalue / 1000.0f; // Convert to volts
        default:
            // Return stored value if available
            if (sensorid < 16) {
                return sensor_values[sensorid];
            }
            return 0.0f;
    }
}

float sensor_get_activity_value(uint16_t activity) {
    MAIN_CONFIG conf;
    conf_general_read_main_conf(&conf);
    
    float total_value = 0.0f;
    int sensor_count = 0;
    
    // Find all sensors with this activity and average their values
    for (int i = 0; i < conf.vehicle.sensors; i++) {
        if (conf.vehicle.sensor[i].activity == activity) {
            float value = sensor_read_value(conf.vehicle.sensor[i].sensorid, conf.vehicle.sensor[i].type);
            total_value += value;
            sensor_count++;
        }
    }
    
    // Return average if sensors found, otherwise 0
    if (sensor_count > 0) {
        return total_value / sensor_count;
    }
    
    return 0.0f;
}

// Function to update a sensor value (called from command processing)
void sensor_update_value(uint16_t sensorid, float value) {
    if (sensorid < 16) {
        sensor_values[sensorid] = value;
    }
}