//! FIDO2 over USB HID. On Windows this needs administrator rights.

use anyhow::{anyhow, bail, Result};
use ctap_hid_fido2::{
    fidokey::FidoKeyHid, public_key_credential_descriptor::PublicKeyCredentialDescriptor,
    FidoKeyHidFactory, LibCfg,
};
use serde::Serialize;

const PICO_VID: u16 = 0x2E8A;

pub fn is_admin() -> bool {
    #[cfg(windows)]
    {
        is_elevated::is_elevated()
    }
    #[cfg(not(windows))]
    {
        true
    }
}

fn open() -> Result<FidoKeyHid> {
    if !is_admin() {
        bail!("Для операций FIDO нужны права администратора. Нажмите «Перезапустить от администратора».");
    }
    let cfg = LibCfg::init();
    let devs: Vec<_> = ctap_hid_fido2::get_fidokey_devices()
        .into_iter()
        .filter(|d| d.vid == PICO_VID)
        .collect();
    match devs.len() {
        0 => FidoKeyHidFactory::create(&cfg).map_err(|e| anyhow!("Ключ FIDO не найден: {e}")),
        _ => FidoKeyHidFactory::create_by_params(&[devs[0].param.clone()], &cfg)
            .map_err(|e| anyhow!("Нет соединения с ключом FIDO: {e}")),
    }
}

#[derive(Debug, Serialize)]
pub struct FidoInfo {
    pub versions: Vec<String>,
    pub extensions: Vec<String>,
    pub options: Vec<(String, bool)>,
    pub algorithms: Vec<String>,
    pub min_pin_length: u32,
    pub firmware_version: u32,
    pub remaining_discoverable: u32,
    pub aaguid: String,
    pub pin_set: bool,
    pub pin_retries: Option<i32>,
}

pub fn info() -> Result<FidoInfo> {
    let key = open()?;
    let i = key.get_info()?;
    let pin_set = i.options.iter().any(|(k, v)| k == "clientPin" && *v);
    let pin_retries = if pin_set { key.get_pin_retries().ok() } else { None };
    Ok(FidoInfo {
        versions: i.versions,
        extensions: i.extensions,
        options: i.options,
        algorithms: i.algorithms.into_iter().map(|(t, a)| format!("{a} ({t})")).collect(),
        min_pin_length: i.min_pin_length,
        firmware_version: i.firmware_version,
        remaining_discoverable: i.remaining_discoverable_credentials,
        aaguid: hex::encode(i.aaguid),
        pin_set,
        pin_retries,
    })
}

pub fn set_pin(new_pin: &str) -> Result<()> {
    open()?.set_new_pin(new_pin)
}

pub fn change_pin(current: &str, new_pin: &str) -> Result<()> {
    open()?.change_pin(current, new_pin)
}

#[derive(Debug, Serialize)]
pub struct Passkey {
    pub rp_id: String,
    pub rp_name: String,
    pub user_name: String,
    pub user_display_name: String,
    pub user_id: String,
    pub credential_id: String,
}

pub fn list_passkeys(pin: &str) -> Result<Vec<Passkey>> {
    let key = open()?;
    let rps = key.credential_management_enumerate_rps(Some(pin))?;
    let mut out = Vec::new();
    for rp in rps {
        let creds = key.credential_management_enumerate_credentials(Some(pin), &rp.rpid_hash)?;
        for c in creds {
            let u = &c.public_key_credential_user_entity;
            out.push(Passkey {
                rp_id: rp.public_key_credential_rp_entity.id.clone(),
                rp_name: rp.public_key_credential_rp_entity.name.clone(),
                user_name: u.name.clone(),
                user_display_name: u.display_name.clone(),
                user_id: hex::encode(&u.id),
                credential_id: hex::encode(&c.public_key_credential_descriptor.id),
            });
        }
    }
    Ok(out)
}

pub fn delete_passkey(pin: &str, credential_id_hex: &str) -> Result<()> {
    let id = hex::decode(credential_id_hex)?;
    let pkcd = PublicKeyCredentialDescriptor { id, ctype: "public-key".into() };
    open()?.credential_management_delete_credential(Some(pin), pkcd)
}
