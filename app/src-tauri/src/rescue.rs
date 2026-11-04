//! Rescue applet of pico-keys-sdk (CLA 0x80): device info, PHY, flash info, reboot.

use crate::ccid::{build_apdu, PicoCard, AID_MANAGEMENT, AID_RESCUE};
use crate::phy::Phy;
use anyhow::Result;
use serde::Serialize;

const INS_WRITE: u8 = 0x1C;
const INS_READ: u8 = 0x1E;
const INS_REBOOT: u8 = 0x1F;

#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub reader: String,
    pub mcu: String,
    pub product: String,
    pub version: String,
    pub serial: String,
    pub flash: Option<FlashInfo>,
}

#[derive(Debug, Serialize)]
pub struct FlashInfo {
    pub free: u32,
    pub used: u32,
    pub total: u32,
    pub files: u32,
    pub size: u32,
    pub firmware: Option<u32>,
}

fn mcu_name(v: u8) -> &'static str {
    match v {
        0 => "RP2040",
        1 => "RP2350",
        2 => "ESP32-S3",
        3 => "Эмуляция",
        4 => "ESP32-S2",
        _ => "неизвестно",
    }
}

fn product_name(v: u8) -> &'static str {
    match v {
        1 => "Pico HSM",
        2 => "Pico FIDO",
        3 => "Pico OpenPGP",
        _ => "неизвестно",
    }
}

fn be32(b: &[u8], i: usize) -> Option<u32> {
    b.get(i..i + 4).map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

pub fn device_info() -> Result<DeviceInfo> {
    let card = PicoCard::connect()?;
    let sel = card.select(AID_RESCUE)?;
    let flash = card
        .transmit_ok(&build_apdu(0x80, INS_READ, 0x02, 0x00, &[], true))
        .ok()
        .and_then(|d| {
            Some(FlashInfo {
                free: be32(&d, 0)?,
                used: be32(&d, 4)?,
                total: be32(&d, 8)?,
                files: be32(&d, 12)?,
                size: be32(&d, 16)?,
                firmware: be32(&d, 20),
            })
        });
    Ok(DeviceInfo {
        reader: card.reader.clone(),
        mcu: mcu_name(*sel.first().unwrap_or(&255)).into(),
        product: product_name(*sel.get(1).unwrap_or(&255)).into(),
        version: format!("{}.{}", sel.get(2).unwrap_or(&0), sel.get(3).unwrap_or(&0)),
        serial: hex::encode(sel.get(4..).unwrap_or(&[])),
        flash,
    })
}

pub fn read_phy() -> Result<Phy> {
    let card = PicoCard::connect()?;
    card.select(AID_RESCUE)?;
    let d = card.transmit_ok(&build_apdu(0x80, INS_READ, 0x01, 0x00, &[], true))?;
    Ok(Phy::parse(&d))
}

/// Writes the full PHY block. The key may wait for a BOOT button press.
pub fn write_phy(phy: &Phy) -> Result<()> {
    let card = PicoCard::connect()?;
    card.select(AID_RESCUE)?;
    card.transmit_ok(&build_apdu(0x80, INS_WRITE, 0x01, 0x00, &phy.serialize(), false))?;
    Ok(())
}

/// to_bootsel = true: reboot to USB bootloader (RPI-RP2 drive). false: normal reboot.
pub fn reboot(to_bootsel: bool) -> Result<()> {
    let card = PicoCard::connect()?;
    card.select(AID_RESCUE)?;
    // The key resets during the command. A transport error here is expected.
    let _ = card.transmit(&build_apdu(0x80, INS_REBOOT, to_bootsel as u8, 0x00, &[], false));
    Ok(())
}

/// Firmware version string from the management applet, for example "8.0.0".
pub fn management_version() -> Result<String> {
    let card = PicoCard::connect()?;
    let d = card.select(AID_MANAGEMENT)?;
    Ok(String::from_utf8_lossy(&d).into_owned())
}
