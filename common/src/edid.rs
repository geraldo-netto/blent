//! One fixed-porch EDID encoder shared by runtime and development producers.
//! Pure request-in/block-out contract: no filesystem, platform or native state.
use anyhow::{Context, Result};

/// Geometry, physical panel size and current display-instance identity.
/// Pixel/physical dimensions must fit positive 12-bit fields; refresh uses the
/// shared product/timing limits. u32::MAX is rejected as an exhausted instance.
#[derive(Clone, Copy, Debug)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub refresh: u32,
    pub width_mm: u32,
    pub height_mm: u32,
    pub instance: u32,
}

impl Mode {
    /// Encode one owned 128-byte block. Name uses at most 12 Unicode characters,
    /// replacing non-ASCII characters with '?' before LF termination/padding.
    /// The name borrow ends with this call; no output or native state is retained.
    pub fn encode(&self, name: &str) -> Result<[u8; 128]> {
        let Self {
            width,
            height,
            refresh,
            width_mm,
            height_mm,
            instance,
        } = *self;
        let pixel_clock_10khz = crate::display::pixel_clock_10khz(width, height, refresh)?;
        anyhow::ensure!(
            (1..=4095).contains(&width_mm) && (1..=4095).contains(&height_mm),
            "EDID physical dimensions must fit 12 bits and be positive"
        );
        let mut edid = [0u8; 128];

        // Header
        edid[0..8].copy_from_slice(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]);

        // Manufacturer ID (BLN = Blent)
        let (b8, b9) = encode_manufacturer_id(b"BLN");
        edid[8] = b8;
        edid[9] = b9;

        // Product code / serial
        edid[10] = 0x01;
        let serial = instance
            .checked_add(1)
            .context("EDID display instance exhausted")?;
        edid[12..16].copy_from_slice(&serial.to_le_bytes());

        edid[16] = 0; // week
        edid[17] = 34; // year (2024-1990)
        edid[18] = 1; // version
        edid[19] = 4; // revision

        edid[20] = 0xA5; // Digital, 8 bpc, DisplayPort interface code (5)
        edid[21] = (width_mm / 10).clamp(1, 255) as u8; // max image size, cm
        edid[22] = (height_mm / 10).clamp(1, 255) as u8;
        edid[23] = 0x78; // gamma 2.2
        edid[24] = 0xEE; // RGB, DPMS

        // Chromaticity (sRGB). Bytes 25..=34 inclusive — ten of them; the previous
        // nine-byte copy left the last one zeroed.
        edid[25..35].copy_from_slice(&[0xEE, 0x91, 0xA3, 0x54, 0x4C, 0x99, 0x26, 0x0F, 0x50, 0x54]);

        // No established timings; standard timings unused
        for i in 0..8 {
            edid[38 + i * 2] = 0x01;
            edid[39 + i * 2] = 0x01;
        }

        // Custom fixed-porch timings; these do not implement the CVT-RB formulas.
        let h_active = width;
        let v_active = height;
        let (h_front, h_sync, h_back) = crate::display::H_PORCHES;
        let h_blank = h_front + h_sync + h_back;
        let (v_front, v_sync, v_back) = crate::display::V_PORCHES;
        let v_blank = v_front + v_sync + v_back;

        let h_total = h_active + h_blank;
        let v_total = v_active + v_blank;

        let h_image = width_mm;
        let v_image = height_mm;

        // === DTD 1 (bytes 54-71) ===
        let i = 54;
        edid[i..i + 2].copy_from_slice(&pixel_clock_10khz.to_le_bytes());
        edid[i + 2] = (h_active & 0xFF) as u8;
        edid[i + 3] = (h_blank & 0xFF) as u8;
        edid[i + 4] = ((((h_active >> 8) & 0x0F) << 4) | ((h_blank >> 8) & 0x0F)) as u8;
        edid[i + 5] = (v_active & 0xFF) as u8;
        edid[i + 6] = (v_blank & 0xFF) as u8;
        edid[i + 7] = ((((v_active >> 8) & 0x0F) << 4) | ((v_blank >> 8) & 0x0F)) as u8;
        edid[i + 8] = (h_front & 0xFF) as u8;
        edid[i + 9] = (h_sync & 0xFF) as u8;
        edid[i + 10] = (((v_front & 0x0F) << 4) | (v_sync & 0x0F)) as u8;
        edid[i + 11] = ((((h_front >> 8) & 0x03) << 6)
            | (((h_sync >> 8) & 0x03) << 4)
            | (((v_front >> 4) & 0x03) << 2)
            | ((v_sync >> 4) & 0x03)) as u8;
        edid[i + 12] = (h_image & 0xFF) as u8;
        edid[i + 13] = (v_image & 0xFF) as u8;
        edid[i + 14] = ((((h_image >> 8) & 0x0F) << 4) | ((v_image >> 8) & 0x0F)) as u8;
        edid[i + 17] = 0x1E; // non-interlaced, digital separate sync, +h +v

        // === Unused descriptor (bytes 72-89): EDID dummy tag, zero payload ===
        edid[75] = 0x10;

        // === Monitor name descriptor (bytes 90-107) ===
        let i = 90;
        edid[i + 3] = 0xFC;
        write_name(&mut edid, name);

        // === Range limits descriptor (bytes 108-125) ===
        let i = 108;
        edid[i + 3] = 0xFD;
        let minimum_refresh = crate::display::minimum_refresh(width, height)?;
        edid[i + 5] = minimum_refresh as u8;
        edid[i + 6] = crate::MAX_FPS as u8;
        // Include the actual rounded DTD clock and every configured refresh.
        // EDID 1.4 range offsets represent horizontal rates above 255 kHz.
        let clock_hz = u32::from(pixel_clock_10khz) * 10_000;
        let min_h = (v_total * minimum_refresh / 1000).min(clock_hz / (h_total * 1000));
        let max_h = (v_total * crate::MAX_FPS)
            .div_ceil(1000)
            .max(clock_hz.div_ceil(h_total * 1000));
        edid[i + 4] = (u8::from(min_h > 255) << 2) | (u8::from(max_h > 255) << 3);
        edid[i + 7] = (min_h - if min_h > 255 { 255 } else { 0 }) as u8;
        edid[i + 8] = (max_h - if max_h > 255 { 255 } else { 0 }) as u8;
        let max_pclk_mhz = (pixel_clock_10khz as u32 * 10_000).div_ceil(1_000_000);
        edid[i + 9] = max_pclk_mhz.div_ceil(10).min(255) as u8;
        edid[i + 10] = 0x01; // EDID 1.4: range limits only, no timing formula
        edid[i + 11..i + 18].copy_from_slice(b"\x0A      ");

        // Checksum
        let sum: u32 = edid[..127].iter().map(|&b| b as u32).sum();
        edid[127] = ((256 - (sum % 256)) % 256) as u8;

        Ok(edid)
    }
}

fn encode_manufacturer_id(s: &[u8; 3]) -> (u8, u8) {
    let c1 = (s[0] - b'A' + 1) as u16;
    let c2 = (s[1] - b'A' + 1) as u16;
    let c3 = (s[2] - b'A' + 1) as u16;
    let byte8 = ((c1 & 0x1F) << 2) | ((c2 >> 3) & 0x03);
    let byte9 = ((c2 & 0x07) << 5) | (c3 & 0x1F);
    (byte8 as u8, byte9 as u8)
}

fn write_name(edid: &mut [u8; 128], name: &str) {
    let target = &mut edid[95..108];
    target.fill(b' ');
    let mut end = 0;
    for (index, character) in name.chars().take(12).enumerate() {
        target[index] = if character.is_ascii() {
            character as u8
        } else {
            b'?'
        };
        end = index + 1;
    }
    target[end] = b'\n';
}

#[cfg(test)]
mod tests;
