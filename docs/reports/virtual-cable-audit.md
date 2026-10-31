# QloudiCable Virtual Cable Testing & R&D Audit Report

**Generated:** 2026-10-31 13:57:02 UTC  
**Execution Environment:** Local Worker (Zero-Cost / Non-Metered Engine)  
**Binary Version:** qloudicable 0.1.0

---

## 1. Synthetic Hardware Scenario Simulation Matrix

```
Running Synthetic Hardware R&D Simulation Matrix...
[PASS] 1. EPR 240W Cable on 140W Charger: Ensures 240W cable correctly permits 140W contract without false warnings
     - [Info] Battery is charging at about 138.0 W: Measured at the battery, so it reads lower than the charger's rating.
     - [Good] Charging contract is 140 W, the most this setup allows: The charger offers up to 140 W and nothing in the chain is holding it back.
[PASS] 2. 60W Cable on 100W Charger: Detects 3A / 60W cable restricting a 100W capable charger
     - [Warn] Cable is limiting charging speed: The cable's chip states a 60 W limit. The charger can supply 100 W. A higher-rated cable would allow more.
     - [Good] Charging contract is 60 W, the most this setup allows: The charger offers up to 100 W and nothing in the chain is holding it back.
[PASS] 3. Fast 10Gbps NVMe SSD Throttled by USB 2.0 Cable: Verifies data rate bottleneck detection when a 10Gbps SSD is plugged with USB 2.0 cable
     - [Warn] Cable is limiting data speed: NVMe High Speed SSD Case supports 10 Gbps but the cable is rated for 480 Mbps.

All 3 R&D simulation scenarios verified successfully.
```

---

## 2. Virtual Cable Testing Engine Profile Sweep

```
=== QloudiCable Virtual Cable Testing Engine ===

--- Profile: AppleWovenEpr240W ---
  140W Charger -> Negotiated: 140 W, Dissipation: 2.38 W (Risk: false)
  10Gbps SSD   -> Link Speed: 480 Mbps [CABLE THROTTLES DATA], Stability: 100/100 (RockSolid)
  Resistance   : 95 mΩ (Nominal (<150 mΩ))
    * [Info] Charging contract is 0 W (up to 15 W is possible): The charger and cable allow more. Devices usually ask for less when the battery is nearly full or the device needs less.
    * [Info] Battery is charging at about 140.0 W: Measured at the battery, so it reads lower than the charger's rating.

--- Profile: Usb4Gen3x2_40Gbps_240W ---
  140W Charger -> Negotiated: 140 W, Dissipation: 2.62 W (Risk: false)
  10Gbps SSD   -> Link Speed: 10000 Mbps, Stability: 100/100 (RockSolid)
  Resistance   : 105 mΩ (Nominal (<150 mΩ))
    * [Info] Charging contract is 0 W (up to 15 W is possible): The charger and cable allow more. Devices usually ask for less when the battery is nearly full or the device needs less.
    * [Info] Battery is charging at about 140.0 W: Measured at the battery, so it reads lower than the charger's rating.

--- Profile: Usb3Gen2_10Gbps_100W ---
  140W Charger -> Negotiated: 100 W [CABLE LIMITS CHARGING] [THERMAL RISK], Dissipation: 3.12 W (Risk: true)
  10Gbps SSD   -> Link Speed: 10000 Mbps, Stability: 100/100 (RockSolid)
  Resistance   : 125 mΩ (Nominal (<150 mΩ))
    * [Info] Charging contract is 0 W (up to 15 W is possible): The charger and cable allow more. Devices usually ask for less when the battery is nearly full or the device needs less.
    * [Info] Battery is charging at about 100.0 W: Measured at the battery, so it reads lower than the charger's rating.

--- Profile: Usb2Charging60W ---
  140W Charger -> Negotiated: 60 W [CABLE LIMITS CHARGING], Dissipation: 0.99 W (Risk: false)
  10Gbps SSD   -> Link Speed: 480 Mbps [CABLE THROTTLES DATA], Stability: 100/100 (RockSolid)
  Resistance   : 110 mΩ (Nominal (<150 mΩ))
    * [Info] Charging contract is 0 W (up to 15 W is possible): The charger and cable allow more. Devices usually ask for less when the battery is nearly full or the device needs less.
    * [Info] Battery is charging at about 60.0 W: Measured at the battery, so it reads lower than the charger's rating.

--- Profile: CounterfeitOrDamaged ---
  140W Charger -> Negotiated: 60 W [CABLE LIMITS CHARGING] [THERMAL RISK], Dissipation: 4.32 W (Risk: true)
  10Gbps SSD   -> Link Speed: 480 Mbps [CABLE THROTTLES DATA], Stability: 40/100 (Jittery)
  Resistance   : 480 mΩ (High (300-500 mΩ))
    * [Info] Charging contract is 0 W (up to 15 W is possible): The charger and cable allow more. Devices usually ask for less when the battery is nearly full or the device needs less.
    * [Info] This cable's chip reports unusual values: Its vendor ID is zero; its speed field holds a reserved value; its current field holds a reserved value. This is common on budget cables and does not prove a fault, but treat its stated ratings with caution.
    * [Info] Battery is charging at about 60.0 W: Measured at the battery, so it reads lower than the charger's rating.

--- Profile: UnmarkedPassive ---
  140W Charger -> Negotiated: 60 W [CABLE LIMITS CHARGING], Dissipation: 1.44 W (Risk: false)
  10Gbps SSD   -> Link Speed: 480 Mbps [CABLE THROTTLES DATA], Stability: 100/100 (RockSolid)
  Resistance   : 160 mΩ (Moderate (150-300 mΩ))
    * [Info] Charging contract is 0 W (up to 15 W is possible): The charger and cable allow more. Devices usually ask for less when the battery is nearly full or the device needs less.
    * [Info] Battery is charging at about 60.0 W: Measured at the battery, so it reads lower than the charger's rating.
```

---

## 3. Engineering Analysis & Verification Criteria
- **EPR Charging Safety:** Tested 240W cables negotiate 140W (28V @ 5A) without false watt ceiling warnings.
- **Data Bottleneck Attribution:** Verified charging-only cables (USB 2.0 480 Mbps) plugged into high-speed NVMe drives correctly raise a `cable_limits_data` finding.
- **Resistance & Thermal Safety:** Counterfeit profiles with R_loop > 300	ext{ m}\Omega$ correctly flag `[THERMAL RISK]` under high current loads (P_loss = I^2 \cdot R_\{loop} > 3	ext{ W}$).
- **Contact Bounce & Stability:** Micro-dropout sentinel correctly penalizes contact disconnections, lowering cable stability scores.
