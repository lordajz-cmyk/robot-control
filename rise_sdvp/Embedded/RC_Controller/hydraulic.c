/*
	Copyright 2019 Benjamin Vedder	benjamin@vedder.se

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

#include "hydraulic.h"
#include "ch.h"
#include "hal.h"
#include "conf_general.h"
#include "pwm_esc.h"
#include "utils.h"
#include "comm_can.h"
//#include "watchdog.h"
#include <math.h>
#ifdef SERVO_READ
#define READING_TIMEOUT 1000
#endif

#include "pos.h"
#include "commands.h"

//thread_t *hydro_thread = NULL;

// Settings
#ifdef IS_MACTRAC
#define SERVO_LEFT				10 // Only one servo
#define SERVO_RIGHT				1
#define SPEED_M_S				1.1
#else
#define SERVO_LEFT				1
#define SERVO_RIGHT				2
#define SPEED_M_S				0.6
#endif

//float time_last =0;


#define TIMEOUT_SECONDS			2.0
#define TIMEOUT_SECONDS_MOVE	10.0

// Private variables
static volatile float m_distance_now = 0.0;
static volatile float m_timeout_cnt = 0.0;
static volatile HYDRAULIC_MOVE m_move_front = HYDRAULIC_MOVE_STOP;
static volatile HYDRAULIC_MOVE m_move_rear = HYDRAULIC_MOVE_STOP;
static volatile HYDRAULIC_MOVE m_move_extra = HYDRAULIC_MOVE_STOP;
static volatile float m_move_timeout_cnt = 0.0;
volatile float m_throttle_set = 0.0;
volatile float m_speed_now = 1.0;

//extern event_source_t emergency_event;
extern int iDebug;

//#ifdef SERVO_READ
extern ADC_CNT_t io_board_adc0_cnt;
//extern time_t last_reading_time = 0;
//static const time_t READING_TIMEOUT = 1.0; // Timeout in seconds


//#endif


// Threads
static THD_WORKING_AREA(hydro_thread_wa, 1024);
static THD_FUNCTION(hydro_thread, arg);

void hydraulic_init(void) {
#ifdef IS_MACTRAC
	main_config.mr.motor_pwm_min_us = 1000;
	main_config.mr.motor_pwm_max_us = 2000;
#else
	main_config.mr.motor_pwm_min_us = 1000;
	main_config.mr.motor_pwm_max_us = 2100;
#endif

	// prop_valve_init();

	chThdCreateStatic(hydro_thread_wa, sizeof(hydro_thread_wa), NORMALPRIO, hydro_thread, NULL);
}

// static void prop_valve_init(){
// 	comm_can_prop_valve_nmt_init();
// 	comm_can_prop_valve_activate();
// }

/**
 * Get current speed
 *
 * @return
 * Speed in m/s
 */
float hydraulic_get_speed(void) {
	return m_speed_now;
}

/**
 * Get travel distance
 *
 * @param reset
 * Reset distance counter
 *
 * @return
 * Travel distance in meters
 */
float hydraulic_get_distance(bool reset) {
	float ret = m_distance_now;

	if (reset) {
		m_distance_now = 0.0;
	}

	return ret;
}

/**
 * Set speed in m/s
 *
 * @param speed
 * Speed in m/s
 */
void hydraulic_set_speed(float speed) {
	/*
    // Exit if no heartbeat
    if (system_state != SYSTEM_STATE_OPERATIONAL) {
        // Force speed to 0 if watchdog indicates unsafe state
        speed = 0.0f;
    }

    */
	m_timeout_cnt = 0.0;

	if (fabsf(speed) < 0.01) {
		pwm_esc_set(SERVO_LEFT, 0.5);
		pwm_esc_set(SERVO_RIGHT, 0.5);
		m_throttle_set = 0.0;
#ifndef WHEEL_SENSOR
		m_speed_now = 0.0;
#endif
	} else {
#ifdef IS_MACTRAC
		float throttle_val = speed * 0.5;
		float pos = utils_map(throttle_val, -1.0, 1.0, 0.0, 1.0);
		pwm_esc_set(SERVO_RIGHT, 1.0 - pos);
#else
		float throttle_val = 1.0;
		pwm_esc_set(SERVO_LEFT, speed > 0.0 ? throttle_val : 0.0);
		pwm_esc_set(SERVO_RIGHT, speed > 0.0 ? 0.0 : throttle_val);
#endif

		// TODO: Update this
		m_throttle_set = SIGN(speed) * throttle_val;
#ifndef HYDRAULIC_HAS_SPEED_SENSOR
		m_speed_now = SIGN(speed) * SPEED_M_S;
#endif
	}
}

void hydraulic_set_throttle_raw(float throttle) {
	m_timeout_cnt = 0.0;
	utils_truncate_number_abs(&throttle, 1.0);
	m_throttle_set = throttle;

#ifndef WHEEL_SENSOR
	if (fabsf(throttle) > 0.9) {
		m_speed_now = SIGN(throttle) * SPEED_M_S;
	} else {
		m_speed_now = 0.0;
	}
#endif

	float pos = utils_map(throttle, -1.0, 1.0, 0.0, 1.0);
	pwm_esc_set(SERVO_LEFT, pos);
	pwm_esc_set(SERVO_RIGHT, 1.0 - pos);
}

void hydraulic_move(HYDRAULIC_POS pos, HYDRAULIC_MOVE move) {
	m_move_timeout_cnt = 0;
	if (iDebug==77)
	{
		commands_printf("in hydraulic move!");
	}
	switch (pos) {
		case HYDRAULIC_POS_FRONT:
			m_move_front = move;
			break;

		case HYDRAULIC_POS_REAR:
			m_move_rear = move;
			break;

		case HYDRAULIC_POS_EXTRA:
			m_move_extra = move;
			break;

		default:
			break;
	}
}


static THD_FUNCTION(hydro_thread, arg) {
	(void)arg;

//	event_listener_t el;
//    eventmask_t events;
	chRegSetThreadName("Hydraulic");
	HYDRAULIC_MOVE move_last_front = HYDRAULIC_MOVE_STOP;
	HYDRAULIC_MOVE move_last_rear = HYDRAULIC_MOVE_STOP;
	HYDRAULIC_MOVE move_last_extra = HYDRAULIC_MOVE_STOP;
	int move_repeat_cnt = 0;


//   chEvtRegister(&emergency_event, &el, EMERGENCY_STOP_EVENT);

#ifdef SERVO_READ
//    const systime_t READING_TIMEOUT = TIME_MS2I(1000);// Timeout in milli seconds
#endif

    while (!chThdShouldTerminateX()) {


//		events = chEvtWaitAnyTimeout(EMERGENCY_STOP_EVENT, MS2ST(100));

//		if (events & EMERGENCY_STOP_EVENT)
//		{// Perform cleanup if necessary
//            // ...
//            break;
//        }
		if (fabsf(m_speed_now) > 0.01) {
			m_distance_now += m_speed_now * 0.01;
		}

		chThdSleepMilliseconds(10);

		m_timeout_cnt += 0.01;

		if (m_timeout_cnt >= TIMEOUT_SECONDS) {
			pwm_esc_set(SERVO_LEFT, 0.5);
			pwm_esc_set(SERVO_RIGHT, 0.5);
			m_throttle_set = 0.0;
#ifndef WHEEL_SENSOR
			m_speed_now = 0.0;
#endif
		}

		m_move_timeout_cnt += 0.01;
		if (m_move_timeout_cnt >= TIMEOUT_SECONDS_MOVE) {
			m_move_timeout_cnt = TIMEOUT_SECONDS_MOVE - 0.5;
			m_move_front = HYDRAULIC_MOVE_STOP;
			m_move_rear = HYDRAULIC_MOVE_STOP;
			move_last_extra = HYDRAULIC_MOVE_STOP;
		}

		move_repeat_cnt++;
		if (move_repeat_cnt > 10) {
			move_last_front = HYDRAULIC_MOVE_UNDEFINED;
			move_last_rear = HYDRAULIC_MOVE_UNDEFINED;
			move_last_extra = HYDRAULIC_MOVE_UNDEFINED;
		}


		ADC_CNT_t cnt;
#ifdef SERVO_READ
		cnt=io_board_adc0_cnt;
#else
		cnt = *comm_can_io_board_adc0_cnt();
#endif




#ifdef IS_MACTRAC
		// Control hydraulic actuators
		#ifdef ADDIO
		if (!comm_can_addio_lim_sw(0) && m_move_front == HYDRAULIC_MOVE_UP) {
			m_move_front = HYDRAULIC_MOVE_STOP;
		} else if (!comm_can_addio_lim_sw(1) && m_move_front == HYDRAULIC_MOVE_DOWN) {
			m_move_front = HYDRAULIC_MOVE_STOP;
		}

		if (!comm_can_addio_lim_sw(2) && m_move_rear == HYDRAULIC_MOVE_UP) {
			m_move_rear = HYDRAULIC_MOVE_STOP;
		} else if (!comm_can_addio_lim_sw(3) && m_move_rear == HYDRAULIC_MOVE_DOWN) {
			m_move_rear = HYDRAULIC_MOVE_STOP;
		}

		#elif IO_BOARD
		if (comm_can_io_board_lim_sw(0) && m_move_front == HYDRAULIC_MOVE_DOWN) {
			m_move_front = HYDRAULIC_MOVE_STOP;
		} else if (comm_can_io_board_lim_sw(1) && m_move_front == HYDRAULIC_MOVE_UP) {
			m_move_front = HYDRAULIC_MOVE_STOP;
		}

		if (comm_can_io_board_lim_sw(2) && m_move_rear == HYDRAULIC_MOVE_DOWN) {
			m_move_rear = HYDRAULIC_MOVE_STOP;
		} else if (comm_can_io_board_lim_sw(3) && m_move_rear == HYDRAULIC_MOVE_UP) {
			m_move_rear = HYDRAULIC_MOVE_STOP;
		}
		#endif

		if (move_last_front != m_move_front) {
			move_last_front = m_move_front;
			#ifdef CAN_ADDIO
			comm_can_addio_set_valve(0, move_last_front == HYDRAULIC_MOVE_UP);
			comm_can_addio_set_valve(1, move_last_front == HYDRAULIC_MOVE_DOWN);
			#elif CAN_IO_BOARD
			comm_can_io_board_set_valve(0, 1, move_last_front == HYDRAULIC_MOVE_UP);
			comm_can_io_board_set_valve(0, 2, move_last_front == HYDRAULIC_MOVE_DOWN);
			#endif
		}

		if (move_last_rear != m_move_rear) {
			move_last_rear = m_move_rear;
			#ifdef CAN_ADDIO
			comm_can_addio_set_valve(2, move_last_rear == HYDRAULIC_MOVE_UP);
			comm_can_addio_set_valve(3, move_last_rear == HYDRAULIC_MOVE_DOWN);
			#elif CAN_IO_BOARD
			comm_can_io_board_set_valve(0, 5, move_last_rear == HYDRAULIC_MOVE_UP);
			comm_can_io_board_set_valve(0, 6, move_last_rear == HYDRAULIC_MOVE_DOWN);
			#endif
		}

		if (move_last_extra != m_move_extra) {
			move_last_extra = m_move_extra;
			#ifdef CAN_ADDIO
			comm_can_addio_set_valve(4, move_last_extra == HYDRAULIC_MOVE_OUT);
			comm_can_addio_set_valve(5, move_last_extra == HYDRAULIC_MOVE_IN);
			#elif CAN_IO_BOARD
			comm_can_io_board_set_valve(0, 4, move_last_extra == HYDRAULIC_MOVE_OUT);
			comm_can_io_board_set_valve(0, 3, move_last_extra == HYDRAULIC_MOVE_IN);
			#endif
		}
#else
		(void)move_last_front;
		(void)move_last_rear;
		(void)move_last_extra;
 #endif
		/*
 */
	}

	//    chEvtUnregister(&emergency_event, &el);
//    chThdExit(MSG_OK);
}
