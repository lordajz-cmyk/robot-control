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

#ifndef SENSOR_CONTROL_H
#define SENSOR_CONTROL_H

#include "datatypes.h"

// Sensor control functions
SENSOR* sensor_get_sensors_by_activity(uint16_t activity, int* count);
float sensor_read_value(uint16_t sensorid, SENSOR_TYPE type);
float sensor_get_activity_value(uint16_t activity);
void sensor_control_init(void);

#endif /* SENSOR_CONTROL_H */