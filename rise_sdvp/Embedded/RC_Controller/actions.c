/*
 * actions.c
 *
 *  Created on: Jun 4, 2026
 *      Author: gunnar
 */

#include "datatypes.h"

static ACTUATOR* actuators;
static int actuator_count=0;


void actuators_init()
{

}

void actuator_add(ACTUATOR actuator)
{
	actuator_count++;
	return;
}

ACTUATOR actuator_read(int id)
{
	return actuators[id];
}

void actuators_read(ACTUATOR *conf) {
}


void actuator_act(float magnitude)
{

}
