//! VESC:s öppna CAN-protokoll (samma som används rakt av av bl.a. rise_sdvp).
//! CAN-ID:t kodar VESC:ens ID + vilket kommando det gäller (standard-mönstret:
//! `(command_id << 8) | vesc_id`).

pub const CAN_PACKET_SET_DUTY: u8 = 0;
pub const CAN_PACKET_SET_CURRENT: u8 = 1;
pub const CAN_PACKET_SET_RPM: u8 = 3;
pub const CAN_PACKET_STATUS: u8 = 9; // rpm, current, duty
pub const CAN_PACKET_STATUS_5: u8 = 27; // temp, input_voltage

#[derive(Debug, Clone, Copy)]
pub struct VescStatus {
    pub vesc_id: u8,
    pub rpm: f32,
    pub current: f32,
    pub duty: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct VescStatus5 {
    pub vesc_id: u8,
    pub temp_c: f32,
    pub input_voltage: f32,
}

pub fn make_set_rpm_frame(vesc_id: u8, rpm: f32) -> (u32, [u8; 8]) {
    let can_id = ((CAN_PACKET_SET_RPM as u32) << 8) | vesc_id as u32;
    let mut data = [0u8; 8];
    data[0..4].copy_from_slice(&(rpm as i32).to_be_bytes());
    (can_id | 0x8000_0000, data) // extended frame
}

pub fn make_set_current_frame(vesc_id: u8, current_amps: f32) -> (u32, [u8; 8]) {
    let can_id = ((CAN_PACKET_SET_CURRENT as u32) << 8) | vesc_id as u32;
    let mut data = [0u8; 8];
    data[0..4].copy_from_slice(&((current_amps * 1000.0) as i32).to_be_bytes());
    (can_id | 0x8000_0000, data)
}

pub fn parse_status(can_id: u32, data: &[u8]) -> Option<VescStatus> {
    let cmd = (can_id >> 8) as u8;
    if cmd != CAN_PACKET_STATUS || data.len() < 8 {
        return None;
    }
    let vesc_id = (can_id & 0xff) as u8;
    let rpm = i32::from_be_bytes(data[0..4].try_into().ok()?) as f32;
    let current = i16::from_be_bytes(data[4..6].try_into().ok()?) as f32 / 10.0;
    let duty = i16::from_be_bytes(data[6..8].try_into().ok()?) as f32 / 1000.0;
    Some(VescStatus {
        vesc_id,
        rpm,
        current,
        duty,
    })
}

pub fn parse_status5(can_id: u32, data: &[u8]) -> Option<VescStatus5> {
    let cmd = (can_id >> 8) as u8;
    if cmd != CAN_PACKET_STATUS_5 || data.len() < 8 {
        return None;
    }
    let vesc_id = (can_id & 0xff) as u8;
    let temp_c = i16::from_be_bytes(data[0..2].try_into().ok()?) as f32 / 10.0;
    let input_voltage = i16::from_be_bytes(data[4..6].try_into().ok()?) as f32 / 10.0;
    Some(VescStatus5 {
        vesc_id,
        temp_c,
        input_voltage,
    })
}

/// Uppskatta batteriprocent från spänning över 4x 12V litiumblock i serie
/// (~48V nominellt). Grov, linjär uppskattning tills något bättre finns —
/// litiumcellers urladdningskurva är inte linjär, men det räcker för en
/// ungefärlig indikator i hörnet av skärmen.
pub fn estimate_battery_percent(input_voltage: f32) -> f32 {
    const EMPTY_V: f32 = 42.0; // ungefärlig tom-spänning för 4x12V litium i serie
    const FULL_V: f32 = 54.4; // ungefärlig fulladdad spänning
    ((input_voltage - EMPTY_V) / (FULL_V - EMPTY_V) * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_estimate_clampar() {
        assert_eq!(estimate_battery_percent(30.0), 0.0);
        assert_eq!(estimate_battery_percent(60.0), 100.0);
    }
}
