"""Read/patch pico-keys PHY config over CCID (rescue applet).

Usage:
  python phy_led.py read
  python phy_led.py set-ws2812 [gpio=23] [order=GRB]   # needs BOOT button press on the board
"""
import sys
from smartcard.System import readers

RESCUE_AID = [0xA0, 0x58, 0x3F, 0xC1, 0x9B, 0x7E, 0x4F, 0x21]
TAG_LED_GPIO, TAG_LED_DRIVER = 0x04, 0x0C
DRIVER_WS2812 = 0x03
ORDERS = {"RGB": 0, "RBG": 1, "GRB": 2, "GBR": 3, "BRG": 4, "BGR": 5}


def connect():
    r = [x for x in readers() if "Pico Key" in str(x)]
    if not r:
        sys.exit("Pico Key reader not found")
    c = r[0].createConnection()
    c.connect()
    data, sw1, sw2 = c.transmit([0x00, 0xA4, 0x04, 0x04, len(RESCUE_AID)] + RESCUE_AID + [0x00])
    if (sw1, sw2) != (0x90, 0x00):
        sys.exit(f"select rescue failed: {sw1:02X}{sw2:02X}")
    print(f"rescue: mcu={data[0]} product={data[1]} fw={data[2]}.{data[3]} serial={bytes(data[4:]).hex()}")
    return c


def parse(b):
    out, i = {}, 0
    while i + 2 <= len(b):
        t, l = b[i], b[i + 1]
        out[t] = bytes(b[i + 2:i + 2 + l])
        i += 2 + l
    return out


def serialize(d):
    b = []
    for t, v in d.items():
        b += [t, len(v)] + list(v)
    return b


def read_phy(c):
    data, sw1, sw2 = c.transmit([0x80, 0x1E, 0x01, 0x00, 0x00])
    if (sw1, sw2) != (0x90, 0x00):
        sys.exit(f"read PHY failed: {sw1:02X}{sw2:02X}")
    return parse(data)


def show(d):
    for t, v in d.items():
        print(f"  tag 0x{t:02X}: {v.hex()}")


def main():
    cmd = sys.argv[1] if len(sys.argv) > 1 else "read"
    c = connect()
    d = read_phy(c)
    print("current PHY:")
    show(d)
    if cmd != "set-ws2812":
        return
    kv = dict(a.split("=", 1) for a in sys.argv[2:])
    gpio = int(kv.get("gpio", 23))
    order = ORDERS[kv.get("order", "GRB").upper()]
    d[TAG_LED_GPIO] = bytes([gpio])
    d[TAG_LED_DRIVER] = bytes([DRIVER_WS2812, order])
    payload = serialize(d)
    print("new PHY:")
    show(d)
    print(">>> Нажми кнопку BOOT на плате (подтверждение записи)...")
    _, sw1, sw2 = c.transmit([0x80, 0x1C, 0x01, 0x00, len(payload)] + payload)
    print(f"write SW={sw1:02X}{sw2:02X}", "OK" if (sw1, sw2) == (0x90, 0x00) else "FAILED")
    if (sw1, sw2) == (0x90, 0x00):
        print("Готово. Переподключи плату (вынь/вставь), чтобы применилось.")


if __name__ == "__main__":
    main()
