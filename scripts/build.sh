#!/usr/bin/env bash
# Сборка pico-fido под YD-RP2040 (Linux / WSL).
# Нужно: arm-none-eabi-gcc, arm-none-eabi-newlib, cmake, ninja, git, python3.
set -euo pipefail

PICO_FIDO_TAG="${PICO_FIDO_TAG:-v8.0}"
PICO_SDK_TAG="${PICO_SDK_TAG:-2.2.0}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="${WORK:-$HOME/fido-key-build}"

mkdir -p "$WORK" && cd "$WORK"

[ -d pico-sdk ] || git clone -q --depth 1 --branch "$PICO_SDK_TAG" https://github.com/raspberrypi/pico-sdk.git
git -C pico-sdk submodule update --init --depth 1 lib/tinyusb
cp "$ROOT/board/vccgnd_yd_rp2040.h" pico-sdk/src/boards/include/boards/

if [ ! -d pico-fido ]; then
  git clone -q --depth 1 --branch "$PICO_FIDO_TAG" --recurse-submodules --shallow-submodules \
    https://github.com/polhenarejos/pico-fido.git
  git -C pico-fido apply "$ROOT/patches/pico-fido-clock-override.patch"
  git -C pico-fido/pico-keys-sdk apply "$ROOT/patches/pico-keys-sdk-flash-cap.patch"
fi

cmake -S pico-fido -B pico-fido/build -G Ninja \
  -DPICO_SDK_PATH="$WORK/pico-sdk" \
  -DPICO_BOARD=vccgnd_yd_rp2040 \
  -DPICO_USE_FASTEST_SUPPORTED_CLOCK=0 \
  -DCMAKE_BUILD_TYPE=Release
ninja -C pico-fido/build

mkdir -p "$ROOT/firmware"
cp pico-fido/build/pico_fido.uf2 "$ROOT/firmware/pico_fido_yd-rp2040-${PICO_FIDO_TAG#v}-fix.uf2"
echo "OK -> firmware/pico_fido_yd-rp2040-${PICO_FIDO_TAG#v}-fix.uf2"
