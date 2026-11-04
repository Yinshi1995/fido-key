//! PHY configuration of pico-keys-sdk (TLV), see pico-keys-sdk/src/fs/phy.{c,h}.

use serde::{Deserialize, Serialize};

const TAG_VIDPID: u8 = 0x00;
const TAG_LED_GPIO: u8 = 0x04;
const TAG_LED_BTNESS: u8 = 0x05;
const TAG_OPTS: u8 = 0x06;
const TAG_UP_BTN: u8 = 0x08;
const TAG_USB_PRODUCT: u8 = 0x09;
const TAG_CURVES: u8 = 0x0A;
const TAG_USB_ITF: u8 = 0x0B;
const TAG_LED_DRIVER: u8 = 0x0C;


#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Phy {
    /// Raw 4 bytes as stored on the key: VID (BE) + PID (BE).
    pub vidpid: Option<[u8; 4]>,
    pub led_gpio: Option<u8>,
    /// 0..=15
    pub led_brightness: Option<u8>,
    pub opts: u16,
    /// User presence button timeout, seconds. None or 0 = button disabled.
    pub up_btn: Option<u8>,
    pub usb_product: Option<String>,
    pub enabled_curves: Option<u32>,
    pub enabled_usb_itf: Option<u8>,
    pub led_driver: Option<u8>,
    pub led_order: Option<u8>,
    /// Unknown tags, kept as is.
    #[serde(default)]
    pub extra: Vec<(u8, Vec<u8>)>,
}

impl Phy {
    pub fn parse(b: &[u8]) -> Phy {
        let mut p = Phy::default();
        let mut i = 0;
        while i + 2 <= b.len() {
            let tag = b[i];
            let len = b[i + 1] as usize;
            i += 2;
            if i + len > b.len() {
                break;
            }
            let v = &b[i..i + len];
            match (tag, len) {
                (TAG_VIDPID, 4) => p.vidpid = Some([v[0], v[1], v[2], v[3]]),
                (TAG_LED_GPIO, 1) => p.led_gpio = Some(v[0]),
                (TAG_LED_BTNESS, 1) => p.led_brightness = Some(v[0]),
                (TAG_OPTS, 2) => p.opts = u16::from_be_bytes([v[0], v[1]]),
                (TAG_UP_BTN, 1) => p.up_btn = Some(v[0]),
                (TAG_USB_PRODUCT, _) => {
                    let s = v.split(|c| *c == 0).next().unwrap_or(&[]);
                    p.usb_product = Some(String::from_utf8_lossy(s).into_owned());
                }
                (TAG_CURVES, 4) => p.enabled_curves = Some(u32::from_be_bytes([v[0], v[1], v[2], v[3]])),
                (TAG_USB_ITF, 1) => p.enabled_usb_itf = Some(v[0]),
                (TAG_LED_DRIVER, 1) => p.led_driver = Some(v[0]),
                (TAG_LED_DRIVER, 2) => {
                    p.led_driver = Some(v[0]);
                    p.led_order = Some(v[1]);
                }
                _ => p.extra.push((tag, v.to_vec())),
            }
            i += len;
        }
        p
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut o = Vec::new();
        let mut put = |tag: u8, v: &[u8]| {
            o.push(tag);
            o.push(v.len() as u8);
            o.extend_from_slice(v);
        };
        if let Some(v) = self.vidpid {
            put(TAG_VIDPID, &v);
        }
        if let Some(v) = self.led_gpio {
            put(TAG_LED_GPIO, &[v]);
        }
        if let Some(v) = self.led_brightness {
            put(TAG_LED_BTNESS, &[v.min(15)]);
        }
        put(TAG_OPTS, &self.opts.to_be_bytes());
        if let Some(v) = self.up_btn {
            put(TAG_UP_BTN, &[v]);
        }
        if let Some(s) = &self.usb_product {
            let mut b: Vec<u8> = s.as_bytes().iter().copied().take(30).collect();
            b.push(0);
            put(TAG_USB_PRODUCT, &b);
        }
        if let Some(v) = self.enabled_curves {
            put(TAG_CURVES, &v.to_be_bytes());
        }
        if let Some(v) = self.enabled_usb_itf {
            put(TAG_USB_ITF, &[v]);
        }
        if let Some(d) = self.led_driver {
            match self.led_order {
                Some(ord) => put(TAG_LED_DRIVER, &[d, ord]),
                None => put(TAG_LED_DRIVER, &[d]),
            }
        }
        for (t, v) in &self.extra {
            put(*t, v);
        }
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_from_device() {
        // Bytes read from the real key after flashing the YD-RP2040 build.
        let raw = hex::decode("00042e8a10fe040117060200000c0103").unwrap();
        let p = Phy::parse(&raw);
        assert_eq!(p.led_gpio, Some(23));
        assert_eq!(p.led_driver, Some(3));
        assert_eq!(p.vidpid, Some([0x2e, 0x8a, 0x10, 0xfe]));
        assert_eq!(p.serialize(), raw);
    }

    #[test]
    fn led_order_and_button() {
        let p = Phy {
            led_driver: Some(3),
            led_order: Some(2),
            up_btn: Some(15),
            ..Default::default()
        };
        let q = Phy::parse(&p.serialize());
        assert_eq!(q.led_order, Some(2));
        assert_eq!(q.up_btn, Some(15));
    }
}
