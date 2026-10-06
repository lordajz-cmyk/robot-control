#include "hal.h"

#define OUTPUT(pin) (1U << ((pin) * 2U))
#define ALTERNATE(pin) (2U << ((pin) * 2U))
#define HIGH_SPEED(pin) (3U << ((pin) * 2U))
#define AF_HIGH(pin, af) ((af) << (((pin) - 8U) * 4U))
#define INPUT_PORT {0U, 0U, 0U, 0U, 0U, 0U, 0U}

#if HAL_USE_PAL
/* Start outputs low, keep both CAN transceivers silent until board init,
 * and preserve USB/SWD. PD0/PD1 remain inputs reserved for GPS wheel outputs.
 * PA3 stays input: its translator direction PD7 is low throughout startup.
 */
const PALConfig pal_default_config = {
  {OUTPUT(2U) | OUTPUT(15U) | ALTERNATE(11U) | ALTERNATE(12U) |
       ALTERNATE(13U) | ALTERNATE(14U),
   0U, HIGH_SPEED(11U) | HIGH_SPEED(12U), 0U, 1U << 15U, 0U,
   AF_HIGH(11U, 10U) | AF_HIGH(12U, 10U)}, /* GPIOA */
  {OUTPUT(0U) | OUTPUT(1U) | OUTPUT(3U) | OUTPUT(4U) |
       OUTPUT(14U) | OUTPUT(15U),
   0U, 0U, 0U, (1U << 3U) | (1U << 4U) | (1U << 14U) | (1U << 15U),
   0U, 0U}, /* GPIOB: PWM, DIR_U38/U39, CAN silent */
  {OUTPUT(9U) | OUTPUT(10U) | OUTPUT(11U),
   0U, 0U, 0U, 1U << 9U, 0U, 0U}, /* GPIOC: GPS1 reset high, LEDs off */
  {OUTPUT(7U), 0U, 0U, 0U, 0U, 0U, 0U}, /* GPIOD: DIR_U40 input */
  {OUTPUT(4U), 0U, 0U, 0U, 1U << 4U, 0U, 0U}, /* GPIOE: DIR_U34 output */
  INPUT_PORT, /* GPIOF */
  INPUT_PORT, /* GPIOG */
  INPUT_PORT, /* GPIOH: oscillator pins controlled by RCC */
  INPUT_PORT  /* GPIOI */
};
#endif

void __early_init(void) {
  stm32_clock_init();
}

void boardInit(void) {
}
