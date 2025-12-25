# LoRa Module Troubleshooting Guide

## Quick Diagnostic Checklist

Run through this checklist systematically when LoRa isn't working:

```
[ ] Power: Module has 3.3V on VCC pin (measure with multimeter)
[ ] Ground: Common ground between Nucleo and LoRa supply
[ ] Wiring: TX/RX crossover (Nucleo TX → LoRa RX, LoRa TX → Nucleo RX)
[ ] Pins: Using correct UART pins (PC10/PC11 for UART4)
[ ] Code: UART4 interrupt enabled (listen for RxNotEmpty)
[ ] Baud: 115200 configured in both code and module (try 9600 if unsure)
[ ] Logic Analyzer: Can see data on both TX and RX lines
[ ] LED: Blinking = code is running
```

---

## Problem: "No Response from LoRa Module"

### Symptom

- Nucleo sends AT commands (verified on logic analyzer)
- No response received
- defmt shows no RX bytes

### Diagnosis Tree

#### 1. Check TX/RX Wiring

```
Logic Analyzer Setup:
  D0 → PC10 (Nucleo TX) - Should show: 41 54 0D 0A (AT\r\n)
  D1 → PC11 (Nucleo RX) - Should show: 2B 4F 4B 0D 0A (+OK\r\n)
  GND → Common ground

If D0 shows data but D1 is flat:
  → Swap TX/RX wires (crossover issue)

If both lines flat:
  → Check power to LoRa module

If D1 shows data but code doesn't see it:
  → Software issue (interrupt not firing, wrong pins)
```

**Action:**

```rust
// Verify UART pins in code
let tx = gpioc.pc10.into_alternate();  // ✅ PC10 = UART4_TX
let rx = gpioc.pc11.into_alternate();  // ✅ PC11 = UART4_RX
```

---

#### 2. Check Power Supply

**Measure with Multimeter:**

```
LoRa VCC → Should read 3.3V (±0.1V)
LoRa GND → Must be same as Nucleo GND (0V difference)

If VCC < 3.0V:
  → Power supply insufficient
  → Use isolated MB102 supply

If GND has voltage difference:
  → No common ground connection
  → Add ground wire between supplies
```

**Test:** Disconnect LoRa from breadboard, measure VCC directly on module pins.

---

#### 3. Check Baud Rate

**LoRa modules ship with different defaults:**

- RYLR998: Usually 115200
- HC-12: Often 9600
- E32/E22: Can be 9600 or 115200

**Action: Try all common rates**

```rust
// In init():
let serial_config = Config::default()
    .baudrate(115200.bps());  // Try: 115200, 9600, 57600, 38400

// Flash and test each one
```

**Logic Analyzer Method:**

1. Capture LoRa response
2. Measure bit width (time between transitions)
3. Calculate: `baud = 1 / bit_width`
4. Example: 8.68 µs/bit → 115,207 baud ≈ 115200

---

#### 4. Check UART Interrupt Configuration

**Common mistake: Interrupt not enabled**

```rust
// ❌ Wrong - interrupt never fires
let lora_uart = Serial::new(dp.UART4, (tx, rx), config, &clocks).unwrap();
// Missing: lora_uart.listen(...)

// ✅ Correct
let mut lora_uart = Serial::new(dp.UART4, (tx, rx), config, &clocks).unwrap();
lora_uart.listen(SerialEvent::RxNotEmpty);  // Enable RX interrupt
```

**Verify in code:**

```rust
#[task(binds = UART4, shared = [lora_uart, ...])]
fn uart4_handler(cx: ...) {
    // This should fire when data arrives
    defmt::info!("UART interrupt fired!");  // Add this for debugging
}
```

---

## Problem: "Module Powers On But No AT Response"

### Symptom

- LoRa has power (LED on module is lit)
- Logic analyzer shows Nucleo sending commands
- No response on RX line

### Possible Causes

#### A. Module in Wrong Mode

Some modules have MODE pins or jumpers:

- **Mode 0:** AT command mode (what we want)
- **Mode 1:** Transparent transmission
- **Mode 2:** Low power/sleep

**Check:** Look for MODE, M0, M1 pins on module. Connect to GND for AT mode.

---

#### B. Module Address/Network Mismatch

Some modules ignore commands if network ID doesn't match.

**Solution:** Use broadcast address

```
AT+ADDRESS=0     # Address 0 = broadcast
AT+NETWORKID=18  # Default network
```

---

#### C. Module Firmware Corrupted

If module was mishandled or had power glitch during firmware update.

**Solution:** Some modules have firmware recovery mode:

1. Hold RESET while powering on
2. Send specific recovery command
3. Check manufacturer datasheet

---

## Problem: "Intermittent Responses"

### Symptom

- Sometimes works, sometimes doesn't
- Random crashes or resets
- OLED glitches when LoRa transmits

### Root Cause: Power Supply Issues

**Why It Happens:**

```
LoRa TX Current Draw: 100-150mA
Nucleo 3.3V Rail Limit: ~200mA (shared with OLED, MCU, etc.)

Result: Voltage droops during TX → brownout → crash
```

**Solution: Isolated Power Supply**

```
Before (BAD):
  Nucleo 3.3V ─┬─► LoRa VCC
               ├─► OLED VCC
               └─► MCU peripherals

  Problem: All devices fight for current

After (GOOD):
  Nucleo 3.3V ──► OLED VCC
  MB102 3.3V ──► LoRa VCC
  Common GND ──► All devices

  Result: LoRa has dedicated supply
```

**Hardware Fix:**

1. Get MB102 breadboard power supply (~$5)
2. Set jumper to 3.3V output
3. Connect LoRa VCC to MB102 3.3V rail
4. **Critical:** Connect MB102 GND to Nucleo GND
5. Leave OLED on Nucleo 3.3V (low current, no issues)

---

## Problem: "Garbled/Corrupted Data"

### Symptom

- Receive random characters instead of `+OK`
- defmt shows: `RX: 0xFF 0x00 0xAA` (noise)
- Logic analyzer shows distorted waveforms

### Causes & Solutions

#### 1. Baud Rate Mismatch

**Symptom:** Characters appear but are wrong  
**Fix:** Change baud rate in code to match module

#### 2. Electrical Noise

**Symptom:** Random bits flipped, works better with short wires  
**Fix:**

- Add 100Ω series resistors on TX/RX lines (damping)
- Use twisted pair for TX/RX (reduces crosstalk)
- Keep wires < 30cm for breadboard

#### 3. Ground Loops

**Symptom:** Works when unplugged, fails when connected  
**Fix:**

- Single-point grounding (all grounds meet at one place)
- Avoid ground loops (multiple ground paths)

#### 4. Voltage Level Issues

**Symptom:** Works sometimes, random errors  
**Fix:**

- Ensure Nucleo and LoRa both use 3.3V logic
- If LoRa is 5V, use level shifter

---

## Problem: "OLED Stops Working When LoRa Active"

### Symptom

- OLED works alone
- OLED freezes or shows garbage when LoRa transmits

### Root Cause

Both OLED and LoRa powered from same 3.3V rail → voltage drops during LoRa TX

### Solution

Option 1: Isolated LoRa power (recommended)

```
MB102 3.3V → LoRa VCC
Nucleo 3.3V → OLED VCC
```

Option 2: Add bulk capacitors

```
100µF electrolytic + 100nF ceramic near LoRa VCC
100µF electrolytic + 100nF ceramic near OLED VCC
```

Option 3: Lower LoRa TX power

```
AT+CRFOP=10  # Set to 10 dBm instead of 20 dBm
```

---

## Problem: "Code Compiles But UART Interrupt Never Fires"

### Diagnostic Steps

#### 1. Verify Interrupt Name

```rust
// ❌ Wrong interrupt name
#[task(binds = USART4, ...)]  // No such interrupt!

// ✅ Correct
#[task(binds = UART4, ...)]  // Check PAC for exact name
```

**Find Correct Name:**

```bash
# Search in generated docs
grep -r "UART4" target/thumbv7em-none-eabihf/doc/stm32f4xx_hal/pac/interrupt/
```

#### 2. Verify Interrupt Enabled

```rust
// Must call listen() to enable interrupt
lora_uart.listen(SerialEvent::RxNotEmpty);
```

#### 3. Check NVIC Priority

RTIC sets priorities automatically, but verify no conflicts:

```rust
// In Cargo.toml, make sure:
[dependencies]
rtic = { version = "2.1", features = ["thumbv7-backend"] }
```

#### 4. Test with Loopback

Short TX to RX on Nucleo (PC10 to PC11) and see if interrupt fires:

```rust
// In timer task:
cx.shared.lora_uart.lock(|uart| {
    nb::block!(uart.write(b'X')).ok();  // Send byte
});

// Should trigger uart4_handler() and receive 'X'
```

---

## Problem: "defmt Shows RX Bytes But OLED Doesn't Update"

### Symptom

- `defmt::info!("RX: 0x{:02x}", byte)` shows bytes
- OLED stays blank or shows old data

### Causes

#### 1. Missing `display.flush()`

```rust
// ❌ Wrong - framebuffer not sent to display
Text::new(text, Point::new(0, 20), style).draw(display).unwrap();
// Missing flush!

// ✅ Correct
Text::new(text, Point::new(0, 20), style).draw(display).unwrap();
display.flush().unwrap();  // Actually update screen
```

#### 2. Lock Contention

Display locked by another task when trying to update:

```rust
// Check if lock times out
display_res.lock(|display| {
    defmt::info!("Got display lock");  // Add this
    // ... update display
});
```

#### 3. I2C Bus Error

```rust
// Add error handling
match display.flush() {
    Ok(_) => defmt::info!("Display updated"),
    Err(e) => defmt::error!("Display error: {:?}", e),
}
```

---

## Logic Analyzer Workflow

### Step-by-Step Debugging

#### 1. Setup

```
Channels:
  D0 → PC10 (Nucleo TX, outgoing)
  D1 → PC11 (Nucleo RX, incoming)
  D2 → PB8 (I2C SCL, optional)
  D3 → PB9 (I2C SDA, optional)
  GND → Common ground

Sample Rate: 1-2 MHz (4x baud rate minimum)
```

#### 2. Capture

```
1. Start capture
2. Press reset on Nucleo
3. Wait for 5-10 seconds
4. Stop capture
```

#### 3. Analysis

```
Add Decoders:
  - UART (115200, 8N1) on D0 and D1
  - I2C (100/400 kHz) on D2/D3

Look For:
  ✅ Regular pulses on D0 (Nucleo sending)
  ✅ Responses on D1 (LoRa replying)
  ✅ Correct timing (bit width matches baud rate)
  ❌ Flat line = no signal
  ❌ Noisy = electrical issues
  ❌ Wrong timing = baud mismatch
```

#### 4. Common Patterns

```
Good Communication:
  D0: AT\r\n (41 54 0D 0A) repeating every 1s
  D1: +OK\r\n (2B 4F 4B 0D 0A) after each AT

No Response:
  D0: AT\r\n repeating
  D1: Flat or garbage

Baud Mismatch:
  D0: Looks good
  D1: Decoder shows gibberish (timing wrong)
```

---

## Emergency Recovery Procedures

### Procedure 1: Factory Reset LoRa Module

Some modules support reset command:

```
AT+RESET      # Software reset
AT+RESTORE    # Factory defaults
```

Hardware reset (if software fails):

1. Disconnect power
2. Short RST pin to GND
3. Apply power while shorted
4. Release RST after 1 second

---

### Procedure 2: Module Won't Respond to Anything

**Last Resort Steps:**

1. Try every common baud rate: 9600, 19200, 38400, 57600, 115200
2. Try with and without hardware flow control (RTS/CTS)
3. Check MODE pins (some modules have multiple modes)
4. Review module datasheet for "recovery mode"
5. Contact manufacturer support

---

### Procedure 3: Nucleo Not Flashing

If you get `JtagNoDeviceConnected`:

```bash
# Erase and reconnect
probe-rs erase --chip STM32F446RETx --protocol swd --connect-under-reset

# Then flash normally
cargo run --release
```

---

## Prevention Best Practices

### ✅ Do This

- Use isolated power for LoRa modules
- Add decoupling caps near VCC pins (100nF + 10µF)
- Keep wires short and neat
- Document your wiring with photos
- Test power supply with multimeter before connecting
- Use logic analyzer to verify before debugging code
- Save working configurations in version control

### ❌ Don't Do This

- Power LoRa from Nucleo 3.3V rail
- Use long, loose wires (>30cm)
- Assume TX labels mean signal direction
- Hot-plug modules while powered
- Skip common ground connection
- Debug software before verifying hardware
- Trust default settings without verification

---

## Testing Checklist

Before declaring "it works":

```
Power Supply Test:
[ ] Measure LoRa VCC during idle: 3.3V ±0.05V
[ ] Measure LoRa VCC during TX: ≥3.2V (no major drop)
[ ] Check common ground with multimeter (0.00V difference)

Electrical Test:
[ ] Logic analyzer shows clean waveforms
[ ] UART idle state is HIGH (3.3V)
[ ] No noise or glitches visible

Communication Test:
[ ] AT → +OK
[ ] AT+VER? → +VER=...
[ ] AT+ADDRESS? → +ADDRESS=X
[ ] AT+ADDRESS=7 → +OK
[ ] AT+ADDRESS? → +ADDRESS=7 (verify change)

Software Test:
[ ] LED blinking (proves code running)
[ ] defmt shows all received bytes
[ ] OLED updates with received text
[ ] No crashes or resets for 5 minutes

Range Test:
[ ] Works at 1 meter
[ ] Works at 10 meters
[ ] Works through walls (optional)
```

---

## Tools Checklist

Essential:

- Logic analyzer (8-channel, $10-20)
- Multimeter (voltage, continuity)
- MB102 breadboard power supply
- Jumper wires (quality, short)

Nice to Have:

- Oscilloscope (for analog signal inspection)
- USB-UART adapter (for PC-side LoRa testing)
- Second Nucleo (for two-node testing)

Software:

- PulseView (logic analyzer)
- probe-rs (flashing/debugging)
- minicom/screen (serial terminal)

---

## Getting Help

When asking for help, provide:

1. **Hardware Info:**

   - Board model (Nucleo-F446RE)
   - LoRa module model (RYLR998, HC-12, etc.)
   - Power supply setup
   - Wiring diagram or photo

2. **Software Info:**

   - Rust/HAL versions (`cargo tree | grep stm32f4xx-hal`)
   - Full code (ideally minimal reproducible example)
   - defmt output (copy/paste last 50 lines)

3. **Logic Analyzer Capture:**

   - Screenshot or exported data
   - Clearly label which channel is TX/RX

4. **What You've Tried:**
   - Checklist of steps already completed
   - What worked/didn't work

---

**Remember:** 90% of LoRa issues are hardware (power, wiring, baud rate). Use logic analyzer to prove hardware first, then debug software!
