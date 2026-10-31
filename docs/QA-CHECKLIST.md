# Hardware QA checklist

Run this on real hardware before publishing a release draft. Copy it into the release's notes or an issue, and record the machine, OS version and cables used. Automated tests use simulated data and can't replace it.

## Test kit

- A 240 W or 100 W e-marked cable
- A cable with no e-marker, such as a cheap USB 2.0 charging cable
- A USB-C charger of 60 W or more
- A USB 3 drive

## Every platform

- [ ] The app opens and shows the right version and channel
- [ ] Unplugging and replugging updates the page within a few seconds
- [ ] Charge test finishes, and reports a dropout when the cable is pulled mid-test
- [ ] The "What this device lets us see" section matches what actually appears
- [ ] Nothing claims to know something this OS hides

## macOS and Linux, where port data is available

- [ ] E-marked cable: speed, wattage and maker match the cable's label
- [ ] Unmarked cable: reported as having no e-marker, limited to 60 W
- [ ] Charger's power profiles match the charger's label
- [ ] A 100 W charger on a 60 W cable shows "Cable is limiting charging speed"
- [ ] The USB 3 drive appears under the correct port

## Windows

- [ ] The installer runs, and the app opens on real Windows, not Wine
- [ ] The USB 3 drive on a USB 2.0 cable shows "running slower than it can"
- [ ] The same drive on a fast cable shows full speed
- [ ] Battery power is about right on a laptop

## Android

- [ ] Installs and opens on a real phone
- [ ] Charging power is about right for the charger, and has the right units
- [ ] Unplugging shows the phone on battery

## Drive write test (desktop)

- [ ] Result is plausible for the drive
- [ ] The temporary file is gone afterwards
