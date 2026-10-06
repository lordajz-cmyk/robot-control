/*
	BMI270 (IMU på ROV_MCU-kortet, Upwis MP101_323) på I2C2-benen PB10/PB11 via
	hårdvaru-I2C2. Samma gränssnitt och enheter
	som bmi160_wrapper, så pos.c behöver bara välja drivrutin.

	Start enligt Bosch (datablad och BMI270_SensorAPI): chip-ID 0x24, mjukstart,
	stäng av energisparläget, ladda upp konfigurationsfilen (8 kB, bmi270_config.c)
	i bitar med adress i INIT_ADDR_0/1, INIT_CTRL = 1, kontrollera INTERNAL_STATUS,
	och slå sedan på accel/gyro med 200 Hz, ±16 g och ±2000 °/s.
 */

#include "bmi270_wrapper.h"
#include "conf_general.h"
#include "commands.h"
#include "terminal.h"
#include "hal.h"
#include <string.h>

#if HAS_BMI270

#ifndef BMI270_I2C_ADDR
#define BMI270_I2C_ADDR			0x68	// SDO till GND på ROV_MCU
#endif

// Register (bmi2_defs.h)
#define REG_CHIP_ID				0x00
#define REG_ACC_X_LSB			0x0C	// accel x,y,z och sedan gyro x,y,z: 12 byte
#define REG_INTERNAL_STATUS		0x21
#define REG_ACC_CONF			0x40
#define REG_ACC_RANGE			0x41
#define REG_GYR_CONF			0x42
#define REG_GYR_RANGE			0x43
#define REG_INIT_CTRL			0x59
#define REG_INIT_ADDR_0			0x5B
#define REG_INIT_DATA			0x5E
#define REG_PWR_CONF			0x7C
#define REG_PWR_CTRL			0x7D
#define REG_CMD					0x7E

#define BMI270_CHIP_ID			0x24
#define CMD_SOFT_RESET			0xB6
#define CONFIG_CHUNK			32		// jämnt antal byte, som Boschs read_write_len

extern const uint8_t bmi270_config_file[];
extern const uint16_t bmi270_config_file_len;

// Threads
static THD_FUNCTION(bmi270_thread, arg);
static THD_WORKING_AREA(bmi270_thread_wa, 2048);

// Private
static const I2CConfig m_i2c_config = { OPMODE_I2C, 100000, STD_DUTY_CYCLE };
static msg_t m_i2c_result = MSG_OK;
static i2cflags_t m_i2c_errors = 0;
static void(*read_callback)(float *accel, float *gyro, float *mag) = 0;
static int rate_hz;
static volatile bool m_ok = false;
static volatile uint32_t m_init_failures = 0;
static volatile uint32_t m_read_failures = 0;
static uint8_t m_internal_status = 0;

static void terminal_status(int argc, const char **argv) {
	(void)argc;
	(void)argv;
	commands_printf("BMI270 ready: %s, init status: 0x%02X, init failures: %u, read failures: %u, I2C result: %d, errors: 0x%X\n",
			m_ok ? "yes" : "no", m_internal_status,
			(unsigned int)m_init_failures, (unsigned int)m_read_failures,
			(int)m_i2c_result, (unsigned int)m_i2c_errors);
}

static void start_i2c(void) {
	palSetPadMode(GPIOB, 10, PAL_MODE_ALTERNATE(4) |
			PAL_STM32_OTYPE_OPENDRAIN | PAL_STM32_OSPEED_MID2);
	palSetPadMode(GPIOB, 11, PAL_MODE_ALTERNATE(4) |
			PAL_STM32_OTYPE_OPENDRAIN | PAL_STM32_OSPEED_MID2);
	i2cStart(&I2CD2, &m_i2c_config);
}

static void restore_i2c(void) {
	i2cAcquireBus(&I2CD2);
	i2cStop(&I2CD2);
	palSetPad(GPIOB, 10);
	palSetPad(GPIOB, 11);
	palSetPadMode(GPIOB, 10, PAL_MODE_OUTPUT_OPENDRAIN);
	palSetPadMode(GPIOB, 11, PAL_MODE_OUTPUT_OPENDRAIN);
	// Clear an interrupted byte, then generate STOP before restarting I2C2.
	for (unsigned int i = 0; i < 9; i++) {
		palClearPad(GPIOB, 10);
		chThdSleepMicroseconds(100);
		palSetPad(GPIOB, 10);
		chThdSleepMicroseconds(100);
	}
	palClearPad(GPIOB, 10);
	palClearPad(GPIOB, 11);
	chThdSleepMicroseconds(100);
	palSetPad(GPIOB, 10);
	chThdSleepMicroseconds(100);
	palSetPad(GPIOB, 11);
	chThdSleepMicroseconds(100);
	start_i2c();
	i2cReleaseBus(&I2CD2);
}

static bool transfer(const uint8_t *tx, size_t ntx, uint8_t *rx, size_t nrx) {
	i2cAcquireBus(&I2CD2);
	// Leave the stopped driver alone until recovery restarts it.
	if (I2CD2.state != I2C_READY) {
		i2cReleaseBus(&I2CD2);
		return false;
	}
	m_i2c_result = i2cMasterTransmitTimeout(&I2CD2, BMI270_I2C_ADDR,
			tx, ntx, rx, nrx, MS2ST(100));
	m_i2c_errors = i2cGetErrors(&I2CD2);
	if (m_i2c_result == MSG_TIMEOUT) {
		// ChibiOS leaves DMA armed on timeout. Stop it while the bus is held,
		// before reg_read/reg_write return their stack-backed buffers.
		i2cStop(&I2CD2);
	}
	i2cReleaseBus(&I2CD2);
	return m_i2c_result == MSG_OK;
}

static bool reg_write(uint8_t reg, const uint8_t *data, uint16_t len) {
	uint8_t txbuf[CONFIG_CHUNK + 1];
	if (len > CONFIG_CHUNK) {
		return false;
	}
	txbuf[0] = reg;
	memcpy(txbuf + 1, data, len);
	return transfer(txbuf, len + 1, 0, 0);
}

static bool reg_write1(uint8_t reg, uint8_t val) {
	return reg_write(reg, &val, 1);
}

static bool reg_read(uint8_t reg, uint8_t *data, uint16_t len) {
	return transfer(&reg, 1, data, len);
}

static bool init_bmi270(void) {
	uint8_t id = 0;
	m_internal_status = 0;
	if (bmi270_config_file_len != 8192U) {
		return false;
	}
	// Första läsningen efter spänningspåslag kan misslyckas; läs två gånger.
	reg_read(REG_CHIP_ID, &id, 1);
	if (!reg_read(REG_CHIP_ID, &id, 1) || id != BMI270_CHIP_ID) {
		return false;
	}

	if (!reg_write1(REG_CMD, CMD_SOFT_RESET)) {
		return false;
	}
	chThdSleepMilliseconds(3);
	if (!reg_read(REG_CHIP_ID, &id, 1) || id != BMI270_CHIP_ID) {
		return false;
	}

	// Energisparläget av, konfigurationsladdning av, sedan uppladdning.
	if (!reg_write1(REG_PWR_CONF, 0x00)) {
		return false;
	}
	chThdSleepMilliseconds(1);	// ≥ 450 µs
	if (!reg_write1(REG_INIT_CTRL, 0x00)) {
		return false;
	}

	for (uint16_t index = 0; index < bmi270_config_file_len; index += CONFIG_CHUNK) {
		uint16_t n = bmi270_config_file_len - index;
		if (n > CONFIG_CHUNK) {
			n = CONFIG_CHUNK;
		}
		uint8_t addr[2];
		addr[0] = (uint8_t)((index / 2) & 0x0F);
		addr[1] = (uint8_t)((index / 2) >> 4);
		if (!reg_write(REG_INIT_ADDR_0, addr, 2) ||
				!reg_write(REG_INIT_DATA, bmi270_config_file + index, n)) {
			return false;
		}
	}

	if (!reg_write1(REG_INIT_CTRL, 0x01)) {
		return false;
	}
	chThdSleepMilliseconds(25);	// databladet: ≤ 20 ms

	if (!reg_read(REG_INTERNAL_STATUS, &m_internal_status, 1) ||
			(m_internal_status & 0x0F) != 0x01) {
		return false;
	}

	// Accel, gyro och temperatur på. 200 Hz, normal bandbredd, prestandaläge.
	bool ok = reg_write1(REG_PWR_CTRL, 0x0E);
	ok = ok && reg_write1(REG_ACC_CONF, 0xA9);	// filter_perf | bwp normal | 200 Hz
	ok = ok && reg_write1(REG_ACC_RANGE, 0x03);	// ±16 g
	ok = ok && reg_write1(REG_GYR_CONF, 0xE9);	// filter_perf | noise_perf | bwp normal | 200 Hz
	ok = ok && reg_write1(REG_GYR_RANGE, 0x00);	// ±2000 °/s
	chThdSleepMilliseconds(50);	// gyro startar

	return ok;
}

void bmi270_wrapper_init(int samp_rate_hz) {
	rate_hz = (samp_rate_hz > 0 && samp_rate_hz <= 1000000) ? samp_rate_hz : 500;

	// Release both open-drain lines before changing their GPIO mode.
	palSetPad(GPIOB, 11);
	palSetPad(GPIOB, 10);
	start_i2c();

	m_ok = init_bmi270();
	if (!m_ok) {
		m_init_failures++;
	}
	// A failed initial upload must remain visible and recoverable after boot.
	terminal_register_command_callback("bmi270_status",
			"Read BMI270 initialization and I2C error status.", 0, terminal_status);
	chThdCreateStatic(bmi270_thread_wa, sizeof(bmi270_thread_wa),
			NORMALPRIO, bmi270_thread, NULL);
}

void bmi270_wrapper_set_read_callback(void(*func)(float *accel, float *gyro, float *mag)) {
	read_callback = func;
}

bool bmi270_wrapper_is_ok(void) {
	return m_ok;
}

static THD_FUNCTION(bmi270_thread, arg) {
	(void)arg;
	unsigned int failed_reads = 0;

	chRegSetThreadName("BMI Sampling");

	for(;;) {
		if (!m_ok) {
			chThdSleepMilliseconds(1000);
			restore_i2c();
			m_ok = init_bmi270();
			if (!m_ok) {
				m_init_failures++;
				continue;
			}
			failed_reads = 0;
		}
		uint8_t d[12];

		if (!reg_read(REG_ACC_X_LSB, d, sizeof(d))) {
			m_read_failures++;
			if (++failed_reads >= 5) {
				m_ok = false;
			}
			chThdSleepMilliseconds(5);
			continue;
		}
		failed_reads = 0;

		float tmp_accel[3], tmp_gyro[3], tmp_mag[3];

		for (int i = 0; i < 3; i++) {
			int16_t a = (int16_t)((uint16_t)d[2 * i] | ((uint16_t)d[2 * i + 1] << 8));
			int16_t g = (int16_t)((uint16_t)d[6 + 2 * i] | ((uint16_t)d[6 + 2 * i + 1] << 8));
			tmp_accel[i] = (float)a * 16.0 / 32768.0;
			tmp_gyro[i] = (float)g * 2000.0 / 32768.0;
		}

		memset(tmp_mag, 0, sizeof(tmp_mag));

		if (read_callback) {
			read_callback(tmp_accel, tmp_gyro, tmp_mag);
		}

		chThdSleepMicroseconds(1000000 / rate_hz);
	}
}

#endif /* HAS_BMI270 */
