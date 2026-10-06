/*
	BMI270 (IMU på ROV_MCU-kortet) med samma gränssnitt som bmi160_wrapper:
	accel i g (±16 g) och gyro i grader/s (±2000), levererat via read_callback.
 */

#ifndef IMU_BMI270_WRAPPER_H_
#define IMU_BMI270_WRAPPER_H_

#include "ch.h"
#include "hal.h"
#include <stdint.h>
#include <stdbool.h>

void bmi270_wrapper_init(int samp_rate_hz);
void bmi270_wrapper_set_read_callback(void(*func)(float *accel, float *gyro, float *mag));
bool bmi270_wrapper_is_ok(void);

#endif /* IMU_BMI270_WRAPPER_H_ */
