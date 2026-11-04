import { invoke } from "@tauri-apps/api/core";

export type Phy = {
  vidpid: number[] | null;
  led_gpio: number | null;
  led_brightness: number | null;
  opts: number;
  up_btn: number | null;
  usb_product: string | null;
  enabled_curves: number | null;
  enabled_usb_itf: number | null;
  led_driver: number | null;
  led_order: number | null;
  extra: [number, number[]][];
};

export type DeviceInfo = {
  reader: string;
  mcu: string;
  product: string;
  version: string;
  serial: string;
  flash: {
    free: number;
    used: number;
    total: number;
    files: number;
    size: number;
    firmware: number | null;
  } | null;
};

export type FidoInfo = {
  versions: string[];
  extensions: string[];
  options: [string, boolean][];
  algorithms: string[];
  min_pin_length: number;
  firmware_version: number;
  remaining_discoverable: number;
  aaguid: string;
  pin_set: boolean;
  pin_retries: number | null;
};

export type Passkey = {
  rp_id: string;
  rp_name: string;
  user_name: string;
  user_display_name: string;
  user_id: string;
  credential_id: string;
};

export const OPT = { WCID: 0x1, DIMM: 0x2, DISABLE_POWER_RESET: 0x4, LED_STEADY: 0x8 };
export const ITF = { CCID: 0x1, WCID: 0x2, HID: 0x4, KB: 0x8, ALL_DEFAULT: 0x0f };
export const LED_DRIVERS = [
  { id: 0xff, name: "Выключен" },
  { id: 0x01, name: "Обычный светодиод (GPIO)" },
  { id: 0x03, name: "RGB WS2812" },
];
export const LED_ORDERS = ["RGB", "RBG", "GRB", "GBR", "BRG", "BGR"];

export const api = {
  appStatus: () => invoke<{ admin: boolean; platform: string }>("app_status"),
  deviceInfo: () => invoke<DeviceInfo>("device_info"),
  firmwareVersion: () => invoke<string>("firmware_version"),
  readPhy: () => invoke<Phy>("read_phy"),
  writePhy: (phy: Phy) => invoke<void>("write_phy", { phy }),
  reboot: (toBootsel: boolean) => invoke<void>("reboot", { toBootsel }),
  fidoInfo: () => invoke<FidoInfo>("fido_info"),
  setPin: (newPin: string) => invoke<void>("fido_set_pin", { newPin }),
  changePin: (currentPin: string, newPin: string) =>
    invoke<void>("fido_change_pin", { currentPin, newPin }),
  listPasskeys: (pin: string) => invoke<Passkey[]>("fido_list_passkeys", { pin }),
  deletePasskey: (pin: string, credentialId: string) =>
    invoke<void>("fido_delete_passkey", { pin, credentialId }),
  relaunchAsAdmin: () => invoke<void>("relaunch_as_admin"),
  flashFirmware: (path: string) => invoke<string>("flash_firmware", { path }),
};
