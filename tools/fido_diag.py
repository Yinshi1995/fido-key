"""Direct CTAP diagnostics for the Pico key. Run from an ELEVATED (admin) shell on Windows."""
import sys, time, threading
from fido2.hid import CtapHidDevice
from fido2.ctap2 import Ctap2


def step(name, fn, timeout=15):
    res = {}
    def run():
        try:
            res["ok"] = fn()
        except Exception as e:
            res["err"] = e
    t0 = time.time()
    th = threading.Thread(target=run, daemon=True)
    th.start()
    th.join(timeout)
    dt = time.time() - t0
    if th.is_alive():
        print(f"[HANG] {name}: no answer in {timeout}s")
        return None
    if "err" in res:
        print(f"[ERR ] {name} ({dt:.2f}s): {type(res['err']).__name__}: {res['err']}")
        return None
    print(f"[ OK ] {name} ({dt:.2f}s)")
    return res["ok"]


devs = list(CtapHidDevice.list_devices())
print("devices:", [(d.descriptor.vid, d.descriptor.pid, d.descriptor.product_name) for d in devs])
if not devs:
    sys.exit("No FIDO HID devices visible (need admin on Windows?)")
dev = [d for d in devs if d.descriptor.vid == 0x2E8A][0] if any(d.descriptor.vid == 0x2E8A for d in devs) else devs[0]
print("version:", dev.device_version, "caps:", dev.capabilities)

step("CTAPHID ping", lambda: dev.ping(b"hello"))
ctap = step("CTAP2 init", lambda: Ctap2(dev))
if ctap:
    info = ctap.info
    print("  versions:", info.versions)
    print("  options:", dict(info.options))
    print("  pin_uv_protocols:", info.pin_uv_protocols, "algorithms:", info.algorithms)
    print("  remaining_disc_creds:", info.remaining_disc_creds, "firmware:", info.firmware_version)
    step("authenticatorSelection (touch)", lambda: ctap.selection(), timeout=20)
    # Same thing Windows does to ask for touch before PIN exists: makeCredential with empty pinUvAuthParam
    step("makeCredential w/ empty pinUvAuthParam", lambda: ctap.make_credential(
        b"\x00" * 32, {"id": "diag.local", "name": "diag"},
        {"id": b"diag", "name": "diag"}, [{"type": "public-key", "alg": -7}],
        pin_uv_param=b"", pin_uv_protocol=1), timeout=20)
    step("getInfo again (alive?)", lambda: ctap.get_info())
