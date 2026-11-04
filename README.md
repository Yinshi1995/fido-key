<div align="center">

# 🔑 fido-key

**Швейцарский нож в мире цифровых ключей — на китайской плате за пару долларов.**

FIDO2 / passkeys · SSH · OATH TOTP/HOTP · <sub>скоро:</sub> дисплей · NFC

![board](https://img.shields.io/badge/board-YD--RP2040-black?style=flat-square)
![mcu](https://img.shields.io/badge/MCU-RP2040%20%40125MHz-c51a4a?style=flat-square&logo=raspberrypi&logoColor=white)
![fw](https://img.shields.io/badge/pico--fido-v8.0%20%2B%20patches-2ea44f?style=flat-square)
![fido](https://img.shields.io/badge/FIDO-2.0%E2%80%932.3-blue?style=flat-square&logo=fidoalliance&logoColor=white)
![license](https://img.shields.io/badge/license-AGPL--3.0-lightgrey?style=flat-square)

</div>

---

## Что это

Аппаратный ключ безопасности на базе **YD-RP2040** (VCC-GND Studio, чёрная плата, 4 MB flash, RGB WS2812, кнопки BOOT / RST / USR) с прошивкой [pico-fido](https://github.com/polhenarejos/pico-fido), собранной и исправленной специально под эту плату.

| Функция | Статус | Для чего |
|---|:---:|---|
| **FIDO2 / WebAuthn passkeys** | ✅ | вход на сайты без пароля, SSO (Pocket ID и т.п.) |
| **SSH `ecdsa-sk` resident key** | ✅ | `ssh root@server` — приватный ключ не покидает плату |
| **U2F (CTAP1)** | ✅ | старые сервисы |
| **OATH TOTP / HOTP** (CCID) | ✅ | 2FA-коды через Yubico Authenticator-совместимый протокол |
| **OTP-клавиатура** | ✅ | ключ «печатает» одноразовый пароль сам |
| **RGB-индикация** (WS2812, GPIO23) | ✅ | статус ключа цветом |
| **Дисплей** | 🛠️ план | подтверждение операций, ввод PIN на устройстве |
| **NFC** | 🛠️ план | работа как бесконтактная карта |

---

## ⚠️ Почему не официальная прошивка

Официальный `pico_fido_pico-8.0.uf2` на YD-RP2040 4 MB **не работает и опасен** ([pico-fido#294](https://github.com/polhenarejos/pico-fido/issues/294)):

1. **Зависает на первой CTAP2-команде.** Хранилище данных размещается в 2–4 MB flash, а запись туда на этой плате не доходит. Windows висит на «Are you there? Use your security key».
2. **На 200 MHz кривая математика ECC.** Примерно треть сгенерированных P-256 ключей — точки не на кривой. Кривые ECDSA-подписи потенциально **сливают приватный ключ**.

Что исправлено в этой сборке:

| Патч | Что делает |
|---|---|
| [`patches/pico-fido-clock-override.patch`](patches/pico-fido-clock-override.patch) | позволяет отключить разгон → **125 MHz** |
| [`patches/pico-keys-sdk-flash-cap.patch`](patches/pico-keys-sdk-flash-cap.patch) | хранилище ограничено первыми **2 MB**, начинается после сектора phymarker |
| [`board/vccgnd_yd_rp2040.h`](board/vccgnd_yd_rp2040.h) | описание платы: WS2812 на GPIO23, USR на GPIO24, 4 MB flash |

Результат прямой CTAP-диагностики ([`tools/fido_diag.py`](tools/fido_diag.py)):

```
                         официальный v8.0      эта сборка
CTAPHID ping             ⛔ HANG               ✅ 0.10s
CTAP2 getInfo            ⛔ HANG               ✅ 0.12s  FIDO_2_0…FIDO_2_3, rk, credMgmt
authenticatorSelection   —                    ✅
makeCredential (no PIN)  ⛔ HANG               ✅ PIN_NOT_SET (корректно)
```

---

## 🚀 Быстрый старт

### 1. Прошить

1. Зажми **BOOT** → нажми и отпусти **RST** → отпусти **BOOT**. Появится диск `RPI-RP2`.
2. Скопируй на него `pico_fido_yd-rp2040-8.0-fix.uf2` (из [Releases](../../releases) или собери сам).
3. Плата перезагрузится и определится как *Pico Key* (FIDO HID + CCID + клавиатура).

> RP2040 нельзя «окирпичить»: загрузчик в ROM, BOOT+RST работает всегда.

### 2. Задать PIN

*Параметры Windows → Учётные записи → Варианты входа → Ключ безопасности → Управление*
или просто при первой регистрации passkey — Windows сама предложит.

- PIN хранится **на ключе**, одинаковый для любого ПК / браузера / SSH.
- 8 неверных попыток → ключ блокируется, лечится только сбросом (все passkeys теряются).

### 3. Passkey на сайте

При регистрации в окне Windows: **Change → Security key** (иначе Windows сохранит passkey в Windows Hello, а не на ключ).

### 4. SSH-ключ на плате

```powershell
# Windows: только встроенный OpenSSH (он ходит в ключ через webauthn.dll без прав админа)
C:\Windows\System32\OpenSSH\ssh-keygen.exe -t ecdsa-sk `
  -O resident -O verify-required -O application=ssh:myserver `
  -C "fido-key@myserver" -f $HOME\.ssh\fido_myserver
```

Публичную часть — в `~/.ssh/authorized_keys` на сервере (OpenSSH ≥ 8.2). Вход:

```powershell
C:\Windows\System32\OpenSSH\ssh.exe -i $HOME\.ssh\fido_myserver root@myserver
```

Ключ **resident** — на новом компьютере его можно вытащить с платы: `ssh-keygen -K`.

> ℹ️ В этой сборке EdDSA выключена → используй `ecdsa-sk`, а не `ed25519-sk`.
> ℹ️ Git Bash / MSYS `ssh` не видит FIDO без прав администратора — используй `System32\OpenSSH`.

---

## 🛠️ Сборка из исходников

Linux или WSL (проверено на Arch WSL, `arm-none-eabi-gcc 16.2`, `cmake 4.4`, pico-sdk `2.2.0`):

```bash
# Arch:   pacman -S arm-none-eabi-gcc arm-none-eabi-newlib cmake ninja git python
# Debian: apt install gcc-arm-none-eabi libnewlib-arm-none-eabi cmake ninja-build git python3
./scripts/build.sh
# → firmware/pico_fido_yd-rp2040-8.0-fix.uf2
```

Скрипт клонирует pico-sdk и pico-fido нужных версий, накладывает патчи, ставит board-файл и собирает с `PICO_BOARD=vccgnd_yd_rp2040`, `PICO_USE_FASTEST_SUPPORTED_CLOCK=0`.

---

## 🧰 Утилиты

| Файл | Что делает |
|---|---|
| [`tools/fido_diag.py`](tools/fido_diag.py) | прямая CTAP-диагностика по шагам с детектом зависаний (Windows — от админа). `pip install fido2` |
| [`tools/phy_led.py`](tools/phy_led.py) | чтение/запись PHY-конфига ключа через CCID (драйвер LED, GPIO, порядок цветов). `pip install pyscard` |

```bash
python tools/phy_led.py read
python tools/phy_led.py set-ws2812 gpio=23 order=GRB
```

---

## 🔒 Модель безопасности — честно

| | |
|---|---|
| ✅ | приватные ключи генерируются на плате и не покидают её по USB |
| ✅ | PIN / user verification, блокировка после 8 попыток |
| ⚠️ | **у RP2040 нет secure element** — при физическом доступе к плате flash можно снять программатором |
| ⚠️ | кнопка присутствия по умолчанию выключена (касание подтверждается автоматически) — защиту держит PIN |
| 🔜 | для серьёзного уровня — переезд на **RP2350** (OTP-ключи, secure boot) |

Это отличный ключ для домашней инфраструктуры и личных аккаунтов, но не замена сертифицированному YubiKey для угроз уровня «украли и разобрали».

---

## 🗺️ Роадмап

- [x] Плата определяется, pico-fido прошит
- [x] Найдено и исправлено зависание + ECC на 200 MHz
- [x] RGB-индикация статуса
- [x] Passkey для SSO
- [x] SSH resident key, вход на сервер
- [ ] Кнопка присутствия (BOOT/USR) вместо авто-подтверждения
- [ ] EdDSA → `ed25519-sk`
- [ ] Маленький дисплей (SSD1306 / ST7789): что подтверждаю, ввод PIN на устройстве
- [ ] NFC-модуль → бесконтактная карта
- [ ] Корпус

---

## 📁 Структура

```
fido-key/
├── board/        # board-header YD-RP2040 для pico-sdk
├── patches/      # патчи к pico-fido v8.0 / pico-keys-sdk
├── scripts/      # build.sh
├── tools/        # диагностика и конфиг ключа
└── firmware/     # собранные .uf2 (в git не коммитятся → Releases)
```

## Лицензия

[AGPL-3.0](LICENSE) — как и [pico-fido](https://github.com/polhenarejos/pico-fido) / [pico-keys-sdk](https://github.com/polhenarejos/pico-keys-sdk), на которых построен проект. Вся криптография и протоколы — заслуга Pol Henarejos; здесь — адаптация под плату и исправления.
