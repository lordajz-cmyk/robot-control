#ifndef MP101_323_BOARD_H_
#define MP101_323_BOARD_H_

#define BOARD_MP101_323
#define BOARD_NAME "MP101_323 (STM32F415VGT6)"

#if !defined(STM32F415xx) || defined(STM32F405xx) || defined(STM32F407xx)
#error "MP101_323 requires a consistent STM32F415xx build"
#endif

#define STM32_LSECLK 0U
#define STM32_HSECLK 8000000U
#define STM32_VDD 330U

// ChibiOS 3.0.5 names voltage scaling as a two-bit field. F415 has one
// VOS bit: scale 1 = bit 14 set (168 MHz), scale 2 = zero (<=144 MHz).
// Adapt the old HAL names without modifying the ST CMSIS device header.
#define PWR_CR_VOS_0 (1U << 14U)
#define PWR_CR_VOS_1 0U

#ifdef __cplusplus
extern "C" {
#endif
void boardInit(void);
#ifdef __cplusplus
}
#endif

#endif
