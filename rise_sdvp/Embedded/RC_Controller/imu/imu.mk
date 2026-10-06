IMUSRC = 	imu/mpu9150.c \
			imu/ahrs.c \
			imu/BMI160_driver/bmi160.c \
			imu/bmi160_wrapper.c

ifeq ($(BOARD),mp101)
IMUSRC += imu/bmi270_wrapper.c imu/bmi270_config.c
endif

IMUINC = 	imu \
			imu/BMI160_driver
