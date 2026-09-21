//! Hittar och öppnar CAN-bussen mot CarController-kortet.
//!
//! Status just nu (se PROJECT_SPEC.md §11): okänt om Pi:n ser bussen som
//! ett standard SocketCAN-gränssnitt (`can0`, via gs_usb/candleLight eller
//! liknande på kortets mikrokontroller), eller om den kräver ett eget
//! seriellt protokoll via `/dev/car` (rise_sdvp-mönster). Den här modulen
//! provar SocketCAN först eftersom det är enklast och mest robust om det
//! finns; annars ger den en tydlig, actionable felutskrift i stället för att
//! krascha kryptiskt.

use socketcan::{CanSocket, Socket};

pub enum CanBackend {
    SocketCan(CanSocket),
    /// Seriellt protokoll via CarController-kortets USB-brygga (t.ex.
    /// rise_sdvp/`Car_Client`-stil). Fylls i när protokollet är bekräftat
    /// mot verklig hårdvara.
    SerialBridge { device_path: String },
}

pub fn open_can(interface: &str) -> Result<CanBackend, String> {
    match CanSocket::open(interface) {
        Ok(sock) => {
            tracing::info!("Hittade SocketCAN-gränssnitt {interface}");
            Ok(CanBackend::SocketCan(sock))
        }
        Err(e) => {
            let dev_car = std::path::Path::new("/dev/car");
            if dev_car.exists() {
                tracing::warn!(
                    "Inget SocketCAN-gränssnitt ({interface}: {e}), men /dev/car finns \
                     — verkar vara rise_sdvp-stil seriell CAN-brygga. Protokollet är \
                     inte inkopplat i den här återskapningen än."
                );
                Ok(CanBackend::SerialBridge {
                    device_path: "/dev/car".to_string(),
                })
            } else {
                Err(format!(
                    "Kunde varken hitta SocketCAN-gränssnittet '{interface}' ({e}) eller \
                     /dev/car. Kör diagnostikkommandona i PROJECT_SPEC.md §11 på Pi:n \
                     för att avgöra vilket protokoll CarController-kortet faktiskt pratar."
                ))
            }
        }
    }
}
