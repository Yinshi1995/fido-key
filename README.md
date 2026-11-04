<div align="center">

# 🔑 fido-key

**Многофункциональный аппаратный ключ на плате YD-RP2040.**

FIDO2 / passkeys · SSH · OATH TOTP/HOTP · <sub>план:</sub> дисплей · NFC

![board](https://img.shields.io/badge/board-YD--RP2040-black?style=flat-square)
![mcu](https://img.shields.io/badge/MCU-RP2040%20%40125MHz-c51a4a?style=flat-square&logo=raspberrypi&logoColor=white)
![fw](https://img.shields.io/badge/pico--fido-v8.0%20%2B%20patches-2ea44f?style=flat-square)
![fido](https://img.shields.io/badge/FIDO-2.0%E2%80%932.3-blue?style=flat-square&logo=fidoalliance&logoColor=white)
![license](https://img.shields.io/badge/license-AGPL--3.0-lightgrey?style=flat-square)

</div>

---

## 1. Назначение

Устройство — аппаратный ключ безопасности.
Основа — плата **YD-RP2040** (VCC-GND Studio): 4 МБ flash, светодиод RGB WS2812, кнопки BOOT, RST, USR.
Прошивка — [pico-fido](https://github.com/polhenarejos/pico-fido) v8.0 с исправлениями для этой платы.

| Функция | Статус | Применение |
|---|:---:|---|
| **FIDO2 / WebAuthn (passkeys)** | ✅ | Вход на сайты без пароля. Вход через SSO (Pocket ID). |
| **SSH, ключ `ecdsa-sk` (resident)** | ✅ | Вход на сервер. Закрытый ключ остаётся на плате. |
| **U2F (CTAP1)** | ✅ | Вход в старые сервисы. |
| **OATH TOTP / HOTP** (CCID) | ✅ | Коды двухфакторной аутентификации. |
| **OTP-клавиатура** | ✅ | Ключ вводит одноразовый пароль как клавиатура. |
| **Индикация RGB** (WS2812, GPIO23) | ✅ | Цвет показывает состояние ключа. |
| **Дисплей** | 🛠️ план | Показ операции. Ввод PIN на устройстве. |
| **NFC** | 🛠️ план | Работа как бесконтактная карта. |

---

## 2. Почему нельзя использовать официальную прошивку

> ⚠️ **ПРЕДУПРЕЖДЕНИЕ:** Не используйте официальный файл `pico_fido_pico-8.0.uf2` на плате YD-RP2040 4 МБ.
> Причина описана в [pico-fido#294](https://github.com/polhenarejos/pico-fido/issues/294).

Официальная прошивка имеет две неисправности на этой плате:

1. **Ключ зависает на первой команде CTAP2.** Прошивка хранит данные в области 2–4 МБ. На этой плате запись в эту область не выполняется. Windows показывает «Are you there?» и не продолжает.
2. **На частоте 200 МГц ключи ECC неправильные.** Около 30 % ключей P-256 не лежат на кривой. Неправильная подпись ECDSA может раскрыть закрытый ключ.

Исправления в этой сборке:

| Файл | Изменение |
|---|---|
| [`patches/pico-fido-clock-override.patch`](patches/pico-fido-clock-override.patch) | Разрешает частоту **125 МГц** вместо 200 МГц. |
| [`patches/pico-keys-sdk-flash-cap.patch`](patches/pico-keys-sdk-flash-cap.patch) | Ограничивает хранилище первыми **2 МБ**. Хранилище начинается после сектора phymarker. |
| [`board/vccgnd_yd_rp2040.h`](board/vccgnd_yd_rp2040.h) | Описывает плату: WS2812 на GPIO23, USR на GPIO24, 4 МБ flash. |

Результат проверки утилитой [`tools/fido_diag.py`](tools/fido_diag.py):

```
Команда                   Официальный v8.0     Эта сборка
CTAPHID ping              ⛔ зависание         ✅ 0.10 с
CTAP2 getInfo             ⛔ зависание         ✅ 0.12 с  FIDO_2_0…FIDO_2_3, rk, credMgmt
authenticatorSelection    —                    ✅
makeCredential (без PIN)  ⛔ зависание         ✅ PIN_NOT_SET (правильный ответ)
```

---

## 3. Порядок работы

### 3.1. Прошивка

1. Нажмите и удерживайте кнопку **BOOT**.
2. Нажмите и отпустите кнопку **RST**.
3. Отпустите кнопку **BOOT**. Компьютер показывает диск `RPI-RP2`.
4. Скопируйте файл `pico_fido_yd-rp2040-8.0-fix.uf2` на диск `RPI-RP2`. Файл находится в [Releases](../../releases).
5. Подождите 5 секунд. Плата перезагружается и определяется как *Pico Key*.

> ℹ️ Загрузчик RP2040 находится в ROM. Ошибка прошивки не повреждает плату. Шаги 1–3 работают всегда.

### 3.2. Установка PIN

> ⚠️ **ПРЕДУПРЕЖДЕНИЕ:** После 8 неправильных попыток ввода PIN ключ блокируется. Разблокировка — только сброс. Сброс удаляет все passkeys.

1. Откройте *Параметры Windows → Учётные записи → Варианты входа → Ключ безопасности → Управление*.
2. Введите новый PIN. Используйте 6 символов или больше.
3. Запишите PIN в надёжном месте.

PIN хранится на ключе. PIN одинаковый для всех компьютеров, браузеров и SSH.

### 3.3. Регистрация passkey на сайте

1. На сайте выберите добавление passkey.
2. В окне Windows нажмите **Change**.
3. Выберите **Security key**.
4. Введите PIN.

> ℹ️ Если не выбрать **Security key**, Windows сохраняет passkey в Windows Hello, а не на ключ.

### 3.4. Создание ключа SSH

> ℹ️ В Windows используйте только `C:\Windows\System32\OpenSSH`. Этот OpenSSH работает с ключом без прав администратора. `ssh` из Git Bash требует прав администратора.
> ℹ️ В этой сборке EdDSA выключена. Используйте тип `ecdsa-sk`, а не `ed25519-sk`.

1. Выполните команду:
   ```powershell
   C:\Windows\System32\OpenSSH\ssh-keygen.exe -t ecdsa-sk `
     -O resident -O verify-required -O application=ssh:myserver `
     -C "fido-key@myserver" -f $HOME\.ssh\fido_myserver
   ```
2. Введите PIN в окне Windows.
3. Добавьте содержимое файла `fido_myserver.pub` в `~/.ssh/authorized_keys` на сервере. Сервер должен иметь OpenSSH 8.2 или новее.
4. Выполните вход:
   ```powershell
   C:\Windows\System32\OpenSSH\ssh.exe -i $HOME\.ssh\fido_myserver root@myserver
   ```

Ключ имеет тип resident. На новом компьютере получите ключ с платы командой `ssh-keygen -K`.

---

## 4. Сборка из исходного кода

Требования: Linux или WSL. Проверено: Arch (WSL), `arm-none-eabi-gcc 16.2`, `cmake 4.4`, pico-sdk `2.2.0`.

1. Установите пакеты:
   ```bash
   # Arch
   pacman -S arm-none-eabi-gcc arm-none-eabi-newlib cmake ninja git python
   # Debian / Ubuntu
   apt install gcc-arm-none-eabi libnewlib-arm-none-eabi cmake ninja-build git python3
   ```
2. Выполните скрипт:
   ```bash
   ./scripts/build.sh
   ```
3. Возьмите результат: `firmware/pico_fido_yd-rp2040-8.0-fix.uf2`.

Скрипт выполняет эти операции:
- загружает pico-sdk и pico-fido нужных версий;
- применяет патчи;
- копирует описание платы;
- собирает прошивку с `PICO_BOARD=vccgnd_yd_rp2040` и `PICO_USE_FASTEST_SUPPORTED_CLOCK=0`.

---

## 5. Утилиты

| Файл | Назначение | Зависимость |
|---|---|---|
| [`tools/fido_diag.py`](tools/fido_diag.py) | Проверяет команды CTAP по шагам. Находит зависания. В Windows запускайте от администратора. | `pip install fido2` |
| [`tools/phy_led.py`](tools/phy_led.py) | Читает и записывает конфигурацию PHY через CCID: драйвер LED, GPIO, порядок цветов. | `pip install pyscard` |

```bash
python tools/phy_led.py read
python tools/phy_led.py set-ws2812 gpio=23 order=GRB
```

---

## 6. Модель безопасности

| | Свойство |
|---|---|
| ✅ | Закрытые ключи создаются на плате. Ключи не передаются по USB. |
| ✅ | Ключ требует PIN. После 8 ошибок ключ блокируется. |
| ⚠️ | RP2040 не имеет защищённого элемента. При физическом доступе flash можно прочитать программатором. |
| ⚠️ | Кнопка присутствия выключена. Прошивка подтверждает касание автоматически. Защиту обеспечивает PIN. |
| 🔜 | Для высокого уровня защиты нужен RP2350: ключи в OTP, secure boot. |

Устройство подходит для домашней инфраструктуры и личных учётных записей.
Устройство не заменяет сертифицированный ключ (например, YubiKey) при риске кражи и разборки.

---

## 7. План работ

- [x] Плата определяется. Прошивка pico-fido установлена.
- [x] Зависание и ошибка ECC на 200 МГц исправлены.
- [x] Индикация RGB работает.
- [x] Passkey для SSO работает.
- [x] Ключ SSH (resident) работает. Вход на сервер проверен.
- [ ] Включить кнопку присутствия (BOOT или USR).
- [ ] Включить EdDSA для `ed25519-sk`.
- [ ] Подключить дисплей (SSD1306 или ST7789). Показ операции. Ввод PIN на устройстве.
- [ ] Подключить модуль NFC. Работа как бесконтактная карта.
- [ ] Сделать корпус.

---

## 8. Структура репозитория

```
fido-key/
├── board/        # описание платы YD-RP2040 для pico-sdk
├── patches/      # патчи к pico-fido v8.0 и pico-keys-sdk
├── scripts/      # скрипт сборки build.sh
├── tools/        # утилиты диагностики и конфигурации
└── firmware/     # файлы .uf2 (не хранятся в git, см. Releases)
```

## 9. Лицензия

[AGPL-3.0](LICENSE). Лицензия совпадает с лицензией [pico-fido](https://github.com/polhenarejos/pico-fido) и [pico-keys-sdk](https://github.com/polhenarejos/pico-keys-sdk).
Автор криптографии и протоколов — Pol Henarejos. Этот проект добавляет адаптацию для платы и исправления.
