//! Styr belysningsreläet via Raspberry Pi GPIO. Beslut (spec §7): GPIO17 ->
//! optoisolerat 1-kanals reläkort -> belysning, på/av via on-screen-knapp i
//! klienten (inte dosan). Hållet enkelt: en digital utgång, hög = på.
//!
//! Otestat mot riktig I2C/GPIO-hårdvara ännu (ingen Pi tillgänglig i den här
//! miljön) — strukturen bör stämma, men verifiera pin-numret mot ert
//! faktiska reläkort första gången (aktiv-hög vs aktiv-låg beror på
//! reläkortets utformning; vissa optoisolerade kort är aktiv-låg).

use rppal::gpio::{Gpio, OutputPin};

pub struct LightingRelay {
    pin: OutputPin,
    /// Om reläkortet är aktiv-låg (vanligt för vissa billiga optoisolerade
    /// kort) sätts detta till true, så on/off vänds internt.
    active_low: bool,
}

impl LightingRelay {
    pub fn new(gpio_pin: u8, active_low: bool) -> Result<Self, String> {
        let gpio = Gpio::new().map_err(|e| format!("kunde inte öppna GPIO: {e}"))?;
        let mut pin = gpio
            .get(gpio_pin)
            .map_err(|e| format!("kunde inte hämta GPIO-pin {gpio_pin}: {e}"))?
            .into_output();
        // Säkert utgångsläge vid start: av.
        if active_low {
            pin.set_high();
        } else {
            pin.set_low();
        }
        Ok(Self { pin, active_low })
    }

    pub fn set(&mut self, on: bool) {
        let physically_high = on != self.active_low;
        if physically_high {
            self.pin.set_high();
        } else {
            self.pin.set_low();
        }
    }

    pub fn toggle(&mut self, currently_on: bool) -> bool {
        let new_state = !currently_on;
        self.set(new_state);
        new_state
    }
}
