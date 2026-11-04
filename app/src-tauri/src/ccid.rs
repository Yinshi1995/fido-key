//! PC/SC (CCID) transport to the Pico Key. Works without administrator rights.

use anyhow::{anyhow, bail, Result};
use pcsc::{Card, Context, Protocols, Scope, ShareMode, MAX_BUFFER_SIZE};

pub const AID_RESCUE: &[u8] = &[0xA0, 0x58, 0x3F, 0xC1, 0x9B, 0x7E, 0x4F, 0x21];
pub const AID_MANAGEMENT: &[u8] = &[0xA0, 0x00, 0x00, 0x05, 0x27, 0x47, 0x11, 0x17];

const READER_MATCH: &str = "Pico Key";

pub struct PicoCard {
    card: Card,
    pub reader: String,
}

impl PicoCard {
    pub fn connect() -> Result<Self> {
        let ctx = Context::establish(Scope::User)
            .map_err(|e| anyhow!("Служба смарт-карт недоступна: {e}"))?;
        let readers = ctx.list_readers_owned().unwrap_or_default();
        let reader = readers
            .into_iter()
            .find(|r| r.to_string_lossy().contains(READER_MATCH))
            .ok_or_else(|| anyhow!("Ключ не найден. Подключите ключ."))?;
        let name = reader.to_string_lossy().into_owned();
        let card = ctx
            .connect(&reader, ShareMode::Shared, Protocols::ANY)
            .map_err(|e| anyhow!("Нет соединения с ключом: {e}"))?;
        Ok(Self { card, reader: name })
    }

    /// Send an APDU. Returns (data, sw).
    pub fn transmit(&self, apdu: &[u8]) -> Result<(Vec<u8>, u16)> {
        let mut buf = [0u8; MAX_BUFFER_SIZE];
        let resp = self
            .card
            .transmit(apdu, &mut buf)
            .map_err(|e| anyhow!("Ошибка обмена с ключом: {e}"))?;
        if resp.len() < 2 {
            bail!("Короткий ответ ключа");
        }
        let (data, sw) = resp.split_at(resp.len() - 2);
        Ok((data.to_vec(), u16::from_be_bytes([sw[0], sw[1]])))
    }

    /// Send an APDU and require SW 9000.
    pub fn transmit_ok(&self, apdu: &[u8]) -> Result<Vec<u8>> {
        let (data, sw) = self.transmit(apdu)?;
        if sw != 0x9000 {
            bail!("{}", sw_text(sw));
        }
        Ok(data)
    }

    pub fn select(&self, aid: &[u8]) -> Result<Vec<u8>> {
        let mut apdu = vec![0x00, 0xA4, 0x04, 0x04, aid.len() as u8];
        apdu.extend_from_slice(aid);
        apdu.push(0x00);
        self.transmit_ok(&apdu)
    }
}

pub fn build_apdu(cla: u8, ins: u8, p1: u8, p2: u8, data: &[u8], le: bool) -> Vec<u8> {
    let mut apdu = vec![cla, ins, p1, p2];
    if !data.is_empty() {
        apdu.push(data.len() as u8);
        apdu.extend_from_slice(data);
    }
    if le {
        apdu.push(0x00);
    }
    apdu
}

pub fn sw_text(sw: u16) -> String {
    let msg = match sw {
        0x6985 => "Условие не выполнено. Нажмите кнопку BOOT, когда индикатор мигает.",
        0x6982 => "Нет доступа.",
        0x6A82 => "Приложение не найдено на ключе.",
        0x6A86 => "Неправильные параметры P1/P2.",
        0x6700 => "Неправильная длина данных.",
        0x6D00 => "Команда не поддерживается.",
        0x6E00 => "Класс команды не поддерживается.",
        0x6F00 => "Внутренняя ошибка ключа.",
        _ => "Ключ вернул ошибку.",
    };
    format!("{msg} (SW={sw:04X})")
}
