//! OLED-status-displayen inne i den IP56-klassade lådan (spec §1). Bara
//! synlig när man öppnar locket — inte en stor, ständigt synlig skärm.
//! Visar robotnamn + parkopplingskod vid uppstart, sedan löpande
//! statuskontroller för att kunna felsöka på plats utan dator: internet,
//! om Car_Client-länken (§13) kör, att förväntad USB-hårdvara syns, och
//! batteri (väntar på §13 — se fältkommentar). SSD1306 via I2C.
//!
//! Otestat mot riktig hårdvara. I2C-adress (vanligtvis 0x3C eller 0x3D för
//! SSD1306) och skärmupplösning (vanligtvis 128x64 eller 128x32 för de
//! billiga 0.96"-modulerna) behöver bekräftas mot er faktiska skärm första
//! gången — koden nedan antar 128x64, justera konstanten om er skärm är
//! 128x32.

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use linux_embedded_hal::I2cdev;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

pub struct StatusDisplay {
    display: Ssd1306<
        ssd1306::prelude::I2CInterface<I2cdev>,
        ssd1306::size::DisplaySize128x64,
        ssd1306::mode::BufferedGraphicsMode<ssd1306::size::DisplaySize128x64>,
    >,
}

#[derive(Debug, Clone)]
pub struct DisplayStatus {
    /// Bara `Some` vid uppstart/parkoppling — försvinner ur visningen
    /// efter en stund så den inte tar plats under normal drift.
    pub pairing_code: Option<String>,
    pub internet_ok: bool,
    /// Är Car_Client-länken (§13) uppe — dvs. kör de förväntade skripten
    /// (`car`-tjänsten/`robotd` självt) som de ska.
    pub car_client_ok: bool,
    /// Syns den förväntade USB-hårdvaran (CarController-kortet)?
    pub usb_ok: bool,
    /// `None` tills batterispänning faktiskt går att läsa via VESC
    /// (§13, obekräftat än) — visas som "–", inte som ett falskt 0%.
    pub battery_percent: Option<f32>,
    pub last_error: Option<String>,
}

impl StatusDisplay {
    pub fn new(i2c_bus: &str) -> Result<Self, String> {
        let i2c = I2cdev::new(i2c_bus).map_err(|e| format!("kunde inte öppna I2C-bussen {i2c_bus}: {e}"))?;
        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display
            .init()
            .map_err(|e| format!("kunde inte initiera OLED-displayen: {e:?}"))?;
        Ok(Self { display })
    }

    pub fn show(&mut self, status: &DisplayStatus) -> Result<(), String> {
        self.display.clear(BinaryColor::Off).ok();
        let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

        let mut y: i32 = 10;

        self.draw_line(&mut y, &style, "Status");

        if let Some(code) = &status.pairing_code {
            self.draw_line(&mut y, &style, &format!("Kod: {code}"));
        }

        self.draw_line(&mut y, &style, if status.internet_ok { "Internet: OK" } else { "Internet: NERE" });
        self.draw_line(&mut y, &style, if status.car_client_ok { "Skript: OK" } else { "Skript: NERE" });
        self.draw_line(&mut y, &style, if status.usb_ok { "USB: OK" } else { "USB: SAKNAS" });

        let battery_line = match status.battery_percent {
            Some(pct) => format!("Batteri: {pct:.0}%"),
            None => "Batteri: -".to_string(),
        };
        self.draw_line(&mut y, &style, &battery_line);

        if let Some(err) = &status.last_error {
            // Klipp av långa felmeddelanden så de får plats på raden.
            let truncated: String = err.chars().take(20).collect();
            self.draw_line(&mut y, &style, &format!("Fel: {truncated}"));
        }

        self.display.flush().map_err(|e| format!("kunde inte rita om displayen: {e:?}"))
    }

    fn draw_line(&mut self, y: &mut i32, style: &MonoTextStyle<BinaryColor>, text: &str) {
        Text::new(text, Point::new(0, *y), *style).draw(&mut self.display).ok();
        *y += 11;
    }
}
