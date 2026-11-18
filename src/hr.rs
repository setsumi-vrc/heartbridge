//! Decoding of the Bluetooth "Heart Rate Measurement" characteristic (0x2A37)
//! and heart-rate zones.

#[derive(Debug, Clone, PartialEq)]
pub struct HeartRate {
    pub bpm: u16,
    /// None = the sensor does not report skin contact.
    pub contact: Option<bool>,
    pub energy_kj: Option<u16>,
    /// Intervals between beats (RR) in milliseconds.
    pub rr_ms: Vec<f32>,
}

impl HeartRate {
    /// The strap sends 0 bpm (or reports no contact) when not worn.
    pub fn is_reading(&self) -> bool {
        self.bpm > 0 && self.contact != Some(false)
    }
}

/// Decodes 0x2A37 per the Bluetooth specification.
pub fn parse_heart_rate(data: &[u8]) -> Option<HeartRate> {
    let flags = *data.first()?;
    let mut i = 1;
    let read_u16 =
        |i: usize| -> Option<u16> { Some(u16::from_le_bytes([*data.get(i)?, *data.get(i + 1)?])) };

    let bpm = if flags & 0x01 != 0 {
        let v = read_u16(i)?;
        i += 2;
        v
    } else {
        let v = *data.get(i)? as u16;
        i += 1;
        v
    };

    let contact = match (flags >> 1) & 0b11 {
        0b10 => Some(false),
        0b11 => Some(true),
        _ => None,
    };

    let energy_kj = if flags & 0x08 != 0 {
        let v = read_u16(i)?;
        i += 2;
        Some(v)
    } else {
        None
    };

    let mut rr_ms = Vec::new();
    if flags & 0x10 != 0 {
        while let Some(raw) = read_u16(i) {
            if raw != 0 {
                rr_ms.push(raw as f32 * 1000.0 / 1024.0);
            }
            i += 2;
        }
    }

    Some(HeartRate { bpm, contact, energy_kj, rr_ms })
}

/// Intensity zone as a percentage of max heart rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Rest,
    Light,
    Moderate,
    Hard,
    Max,
}

impl Zone {
    /// Below 60% rest; 60-70 light; 70-80 moderate; 80-90 hard; 90+ max.
    pub fn from_bpm(bpm: u16, max_hr: u16) -> Zone {
        let pct = bpm as f32 / max_hr.max(1) as f32;
        match pct {
            p if p < 0.60 => Zone::Rest,
            p if p < 0.70 => Zone::Light,
            p if p < 0.80 => Zone::Moderate,
            p if p < 0.90 => Zone::Hard,
            _ => Zone::Max,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Zone::Rest => "Rest",
            Zone::Light => "Light",
            Zone::Moderate => "Moderate",
            Zone::Hard => "Hard",
            Zone::Max => "Max",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpm_8_bits_sin_extras() {
        let m = parse_heart_rate(&[0x00, 72]).unwrap();
        assert_eq!(m.bpm, 72);
        assert_eq!(m.contact, None);
        assert!(m.rr_ms.is_empty());
        assert!(m.is_reading());
    }

    #[test]
    fn bpm_16_bits() {
        let m = parse_heart_rate(&[0x01, 0x2C, 0x01]).unwrap();
        assert_eq!(m.bpm, 300);
    }

    #[test]
    fn contacto_y_rr() {
        let m = parse_heart_rate(&[0x16, 65, 0x00, 0x04, 0x00, 0x02]).unwrap();
        assert_eq!(m.bpm, 65);
        assert_eq!(m.contact, Some(true));
        assert_eq!(m.rr_ms, vec![1000.0, 500.0]);
    }

    #[test]
    fn sin_contacto_con_energia() {
        let m = parse_heart_rate(&[0x0C, 80, 0x10, 0x00]).unwrap();
        assert_eq!(m.contact, Some(false));
        assert_eq!(m.energy_kj, Some(16));
        assert!(!m.is_reading());
    }

    #[test]
    fn banda_suelta_manda_cero() {
        // Lo que manda la HS500 sin ponérsela: sin contacto, 0 bpm, RR 0.
        let m = parse_heart_rate(&[0x14, 0, 0x00, 0x00]).unwrap();
        assert_eq!(m.bpm, 0);
        assert!(m.rr_ms.is_empty());
        assert!(!m.is_reading());
    }

    #[test]
    fn paquete_corto() {
        assert!(parse_heart_rate(&[]).is_none());
        assert!(parse_heart_rate(&[0x01, 0x10]).is_none());
    }

    #[test]
    fn zones() {
        assert_eq!(Zone::from_bpm(70, 190), Zone::Rest);
        assert_eq!(Zone::from_bpm(120, 190), Zone::Light);
        assert_eq!(Zone::from_bpm(140, 190), Zone::Moderate);
        assert_eq!(Zone::from_bpm(160, 190), Zone::Hard);
        assert_eq!(Zone::from_bpm(180, 190), Zone::Max);
    }
}
