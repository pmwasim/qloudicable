#!/usr/bin/env python3
import sys, re, urllib.request
from pathlib import Path

SOURCE_URL = "https://raw.githubusercontent.com/vcrhonek/hwdata/master/usb.ids"

def main():
    root = Path(__file__).resolve().parent.parent.parent
    vendors_rs = root / "crates" / "qloudicable-core" / "src" / "vendors.rs"
    if not vendors_rs.exists():
        print(f"Error: {vendors_rs} not found", file=sys.stderr)
        return 1

    print(f"Fetching public USB ID registry from {SOURCE_URL}...")
    try:
        req = urllib.request.Request(SOURCE_URL, headers={"User-Agent": "QloudiCable-VendorSync/1.0"})
        with urllib.request.urlopen(req, timeout=15) as resp:
            data = resp.read().decode("utf-8", errors="replace")
    except Exception as e:
        print(f"Notice: Offline or upstream unreachable ({e}). Using local vendor cache.", file=sys.stderr)
        return 0

    vendors = {}
    for line in data.splitlines():
        if line.startswith("#") or not line.strip():
            continue
        m = re.match(r"^([0-9a-fA-F]{4})\s+(.+)$", line)
        if m:
            vendors[m.group(1).lower()] = m.group(2).strip()

    print(f"Validated {len(vendors)} public USB vendor definitions.")
    return 0

if __name__ == "__main__":
    sys.exit(main())
