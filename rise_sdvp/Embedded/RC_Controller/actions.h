/*
 * actions.h
 *
 *  Created on: Jun 4, 2026
 *      Author: gunnar
 */

#ifndef ACTIONS_H_
#define ACTIONS_H_

#include "datatypes.h"


void actuators_init();
void actuator_add(ACTUATOR actuator);
ACTUATOR actuator_read(int id);
void actuators_read(ACTUATOR *conf);
void actuator_act(float magnitude);

#endif /* ACTIONS_H_ */
