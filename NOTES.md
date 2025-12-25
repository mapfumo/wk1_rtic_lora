# WEEK 1 ENGINEERING LOG: LoRa Module Bringup

## 1. Project Overview

- **Hardware:** STM32 Nucleo-F446RE (final working version)
- **Peripherals:**
  - LoRa Module (RYLR998 or similar, AT-command based) on **UART4**
  - SSD1306 OLED 128x64 (I2C1)
  - On-board LED (PA5)
- **Language:** Rust with RTIC 2.1 framework
- **Debug Tools:**
  - defmt + probe-rs RTT for logging
  - Logic Analyzer with PulseView (8-channel) for signal verification

---

## 2. Critical Knowledge / Hard-Won Lessons

### A. Power Isolation (Stability Breakthrough) ⚡

**The Problem:**

- Powering LoRa module directly from Nucleo 3.3V caused:
  - Intermittent system crashes
  - UART frame errors and corrupted data
  - OLED display glitches
  - Brownouts during LoRa transmission (high current draw)

**The Solution:**

- Use **isolated 3.3V supply** (Elegoo MB102 Breadboard Power Supply)
- **CRITICAL:** Connect **common ground** between Nucleo and LoRa supply
  - Without common ground: Logic levels are undefined
  - UART communication requires same voltage reference

**Why This Matters:**

- LoRa modules can draw 100-150mA during transmission
- Nucleo 3.3V rail is limited and shared with other peripherals
- Voltage drops during transmission cause system instability

---

### B. UART Wiring: "Mouth-to-Ear" Confusion 🔌

**The Trap:**
Some LoRa modules use "target labeling" - pins labeled for what they connect to, not what they are!

**Discovery Process:**

1. Initially tried standard wiring (TX→TX, RX→RX) - didn't work
2. Logic analyzer showed Nucleo sending but not receiving
3. Realized module TX must go to Nucleo RX (crossover)

**Correct Wiring (Standard Crossover):**

```
Nucleo PC10 (TX / Mouth) ──► LoRa RXD (Ear)
LoRa TXD (Mouth) ──► Nucleo PC11 (RX / Ear)
```

**Verification Method:**

```
Logic Analyzer D0 ─► PC10 (Nucleo TX) — Shows AT commands going out
Logic Analyzer D1 ─► PC11 (Nucleo RX) — Shows +OK responses coming back
Analyzer GND ─► Common Ground
```

**Lesson:** If you're sending commands but not receiving responses, swap TX/RX!

---

### C. Logic Analyzer: Your "Third Eye" 👁️

**Setup:**

- **Sample Rate:** ≥ 4× baud rate (use 1-2 MHz for 115,200 baud)
- **Idle State:** UART idle = HIGH (3.3V)
- **Protocol:** 8N1 (8 data bits, no parity, 1 stop bit)

**What You'll See:**

- **Sent:** `AT+ADDRESS?\r\n` → `41 54 2B 41 44 44 52 45 53 53 3F 0D 0A`
- **Received:** `+ADDRESS=0\r\n` → Confirms module is responding

**PulseView Tips:**

1. Add UART decoder (Protocols > UART)
2. Set baud rate to 115200
3. If text appears garbled, toggle "Invert" in decoder settings
4. Zoom in to see individual bytes as ASCII

**Why It's Critical:**

- Proves electrical connectivity before debugging code
- Shows exact timing issues (baud rate mismatches, framing errors)
- Reveals what the module actually sends vs what you expect

---

### D. UART Pin Selection: USART2 vs UART4 🎯

**Key Discovery:** Different UARTs map to different pins!

**USART2 (First Attempt):**

- PA2 (TX), PA3 (RX)
- **Problem:** These are the ST-Link Virtual COM port pins!
- Conflict: Can't use for LoRa while debugging via USB

**UART4 (Final Solution):**

- **PC10 (TX), PC11 (RX)**
- Dedicated UART, no conflicts
- Available on Arduino connector headers (D0/D1 area)

**Lesson:** Check STM32 pinout diagrams for alternate functions!

---

### E. Rust / HAL / RTIC Gotchas 🦀

#### E1. SSD1306 Version Hell (v0.8 → v0.9)

**Problem:** `ssd1306` crate v0.9.0 changed I2C interface handling

**Symptoms:**

```rust
error: type annotations needed
  --> BufferedGraphicsMode<DisplaySize128x64>
```

**Solutions:**

1. **Add dependency:**

   ```toml
   display_interface_i2c = "0.4"
   ```

2. **Import correct interface:**

   ```rust
   use display_interface_i2c::I2CInterface;
   ```

3. **Explicit type alias (CRITICAL):**
   ```rust
   type LoraDisplay = Ssd1306<
       I2CInterface<I2c<pac::I2C1>>,
       DisplaySize128x64,
       BufferedGraphicsMode<DisplaySize128x64>
   >;
   ```

**Why:** v0.9 uses `display_interface_i2c::I2CInterface` instead of `ssd1306::I2CInterface`

---

#### E2. RTIC Shared Resource Locking 🔒

**Problem:** Multiple tasks accessing same resource causes deadlock or compile errors

**Wrong (Causes Error):**

```rust
#[local]
struct Local {
    rx_buffer: Vec<u8, 32>,  // Used by both TIM2 and UART4
}
```

**Correct (Shared Resource):**

```rust
#[shared]
struct Shared {
    lora_uart: Serial<pac::UART4>,
    display: LoraDisplay,
    rx_buffer: Vec<u8, 32>,  // Move to shared
}
```

**Locking Syntax:**

Single resource:

```rust
cx.shared.rx_buffer.lock(|buf| {
    // Use buf here
});
```

Multiple resources (nested locks):

```rust
uart_res.lock(|uart| {
    buffer_res.lock(|buffer| {
        display_res.lock(|display| {
            // All three locked
        });
    });
});
```

**Why Nested?** RTIC prevents deadlocks by allowing only one lock at a time per task.

---

#### E3. Serial Event Names Changed

**HAL 0.21:**

```rust
use stm32f4xx_hal::serial::Event as SerialEvent;
lora_uart.listen(SerialEvent::RxNotEmpty);  // ✅ Correct
```

**Wrong:**

```rust
lora_uart.listen(SerialEvent::Rxne);  // ❌ Doesn't exist in 0.21
```

**Binding Name Must Match:**

```rust
#[task(binds = UART4, ...)]  // Interrupt vector name
fn uart4_handler(cx: ...) {  // Function name can be anything
```

---

#### E4. UART Read/Write Traits

**Problem:** `read()` and `write()` not found

**Solution:** Import the traits:

```rust
use embedded_hal::serial::{Read, Write};  // For HAL 0.21
```

**Write Blocking Pattern:**

```rust
for b in b"AT+ADDRESS?\r\n" {
    let _ = nb::block!(uart.write(*b));  // Blocking write
}
```

**Read Non-Blocking:**

```rust
if let Ok(byte) = uart.read() {
    // Process byte
}
```

---

## 3. Final Working System

### A. Complete Wiring Table

| Component        | Pin      | Connection             | Logic Analyzer | Notes                     |
| ---------------- | -------- | ---------------------- | -------------- | ------------------------- |
| **LoRa Module**  |          |                        |                |                           |
| VCC              | 3.3V     | Elegoo MB102 3.3V      | -              | Isolated supply!          |
| GND              | GND      | Common GND             | GND clip       | Critical for logic levels |
| TXD (Mouth)      | TXD      | Nucleo PC11 (RX/Ear)   | D1             | Crossover!                |
| RXD (Ear)        | RXD      | Nucleo PC10 (TX/Mouth) | D0             | Crossover!                |
| RST              | -        | Not connected          | -              | Optional                  |
| **OLED Display** |          |                        |                |                           |
| VCC              | 3.3V     | Nucleo 3.3V            | -              | OK to share with Nucleo   |
| GND              | GND      | Common GND             | -              |                           |
| SCL              | I2C1_SCL | Nucleo PB8             | -              | 400 kHz I2C               |
| SDA              | I2C1_SDA | Nucleo PB9             | -              |                           |
| **LED**          |          |                        |                |                           |
| LD2              | PA5      | On-board               | -              | Active-low, 1Hz blink     |

---

### B. Software Architecture

**RTIC Tasks:**

1. **init()** - One-time hardware setup

   - Configure clocks (no HSE in final version - just default)
   - Initialize GPIO, UART4, I2C1, TIM2
   - Enable interrupts

2. **tim2_handler()** - Timer interrupt (1 Hz)

   - Toggle LED (heartbeat)
   - Send `AT+ADDRESS?\r\n` to LoRa module
   - Verify query/response cycle

3. **uart4_handler()** - UART RX interrupt

   - Read incoming bytes
   - Buffer until `\n` detected (complete sentence)
   - Parse and display on OLED
   - Clear buffer for next message

4. **idle()** - Low-power sleep
   - WFI (Wait For Interrupt)

---

### C. Communication Protocol

**Observed UART Traffic:**

| Direction     | Data (ASCII)       | Hex                                      | Notes             |
| ------------- | ------------------ | ---------------------------------------- | ----------------- |
| Nucleo → LoRa | `AT\r\n`           | `41 54 0D 0A`                            | Test connectivity |
| LoRa → Nucleo | `+OK\r\n`          | `2B 4F 4B 0D 0A`                         | Module alive      |
| Nucleo → LoRa | `AT+ADDRESS?\r\n`  | `41 54 2B 41 44 44 52 45 53 53 3F 0D 0A` | Query address     |
| LoRa → Nucleo | `+ADDRESS=0\r\n`   | `2B 41 44 44 52 45 53 53 3D 30 0D 0A`    | Default address   |
| Nucleo → LoRa | `AT+ADDRESS=7\r\n` | -                                        | Set new address   |
| LoRa → Nucleo | `+OK\r\n`          | -                                        | Confirmed         |
| Nucleo → LoRa | `HELLO\r\n`        | `48 45 4C 4C 4F 0D 0A`                   | Invalid command   |
| LoRa → Nucleo | `+ERR=2\r\n`       | -                                        | Unknown command   |

**Line Endings:** All AT commands require `\r\n` (CR+LF)

---

### D. Verified AT Commands

| Command         | Purpose       | Response          | Notes                   |
| --------------- | ------------- | ----------------- | ----------------------- |
| `AT`            | Test          | `+OK`             | Module is alive         |
| `AT+VER?`       | Get version   | `+VER=...`        | Firmware info           |
| `AT+ADDRESS?`   | Get address   | `+ADDRESS=0`      | Default is 0            |
| `AT+ADDRESS=7`  | Set address   | `+OK`             | Range: 0-65535          |
| `AT+NETWORKID?` | Get network   | `+NETWORKID=18`   | Default 18              |
| `AT+BAND?`      | Get frequency | `+BAND=915000000` | 915 MHz (US)            |
| `AT+PARAMETER?` | Get params    | `+PARAMETER=...`  | Spread factor, BW, etc. |

---

## 4. Code Highlights

### Message Parsing Logic

```rust
#[task(binds = UART4, shared = [lora_uart, display, rx_buffer])]
fn uart4_handler(cx: uart4_handler::Context) {
    uart_res.lock(|uart| {
        if let Ok(byte) = uart.read() {
            buffer_res.lock(|buffer| {
                if byte == b'\n' {
                    // Complete sentence received!
                    if let Ok(s) = core::str::from_utf8(buffer.as_slice()) {
                        let clean_str = s.trim();

                        // Update OLED with received text
                        display_res.lock(|display| {
                            display.clear(BinaryColor::Off).unwrap();
                            let text_style = MonoTextStyleBuilder::new()
                                .font(&FONT_6X10)
                                .text_color(BinaryColor::On)
                                .build();

                            Text::new(clean_str, Point::new(0, 20), text_style)
                                .draw(display)
                                .unwrap();

                            display.flush().unwrap();
                        });
                    }
                    buffer.clear();
                } else if byte != b'\r' && !buffer.is_full() {
                    let _ = buffer.push(byte);
                }
            });
        }
    });
}
```

**Key Points:**

- Buffers bytes until `\n` (newline) detected
- Strips `\r` (carriage return) automatically
- Updates OLED with complete sentence
- Clears buffer for next message

---

### Periodic Query Pattern

```rust
#[task(binds = TIM2, shared = [lora_uart], local = [led, timer])]
fn tim2_handler(mut cx: tim2_handler::Context) {
    cx.local.timer.clear_flags(Flag::Update);
    cx.local.led.toggle();

    cx.shared.lora_uart.lock(|uart| {
        // Send AT command every second
        for b in b"AT+ADDRESS?\r\n" {
            let _ = nb::block!(uart.write(*b));
        }
    });
}
```

**Purpose:** Verify communication link every second

---

## 5. Debugging Workflow

### Step 1: Verify Power

- [ ] LoRa module powered from isolated supply
- [ ] Common ground connected
- [ ] Measure 3.3V on LoRa VCC pin
- [ ] Check for voltage drops during transmission

### Step 2: Verify Wiring

- [ ] TX/RX crossover confirmed
- [ ] Logic analyzer shows data on both lines
- [ ] UART idle state is HIGH (3.3V)

### Step 3: Verify Software

- [ ] defmt output shows "System Live" message
- [ ] LED blinking at 1 Hz
- [ ] Logic analyzer decodes valid AT commands
- [ ] Responses appear in logic analyzer

### Step 4: Verify LoRa Module

- [ ] Send `AT\r\n` → Expect `+OK\r\n`
- [ ] Try `AT+ADDRESS?\r\n` → Expect `+ADDRESS=X\r\n`
- [ ] Check baud rate (try 115200, 9600, 57600)

### Step 5: Verify OLED

- [ ] I2C address correct (0x3C or 0x3D)
- [ ] Display shows received text
- [ ] No I2C bus errors in defmt output

---

## 6. Common Issues & Solutions

### Issue: No Response from LoRa Module

**Possible Causes:**

1. TX/RX swapped → Swap wires
2. Wrong baud rate → Try 115200, 9600, 57600
3. No common ground → Connect GND
4. Module not powered → Check 3.3V supply
5. Wrong UART pins → Verify PC10/PC11 for UART4

**Debug:**

- Use logic analyzer to see if module is transmitting
- Check if bytes appear on RX line but aren't being read

---

### Issue: OLED Not Updating

**Possible Causes:**

1. I2C address wrong → Try 0x3C and 0x3D
2. SDA/SCL swapped → Check wiring
3. Display not initialized → Check `display.init()` in code
4. Not calling `display.flush()` → Required to update screen

**Debug:**

- Use I2C scanner to find device address
- Check defmt output for I2C errors

---

### Issue: System Crashes / Brownouts

**Possible Causes:**

1. LoRa powered from Nucleo 3.3V → Use isolated supply
2. Insufficient decoupling caps → Add 10µF + 100nF near LoRa VCC
3. Long wires creating noise → Keep wires short, twisted pairs

---

### Issue: Garbled UART Data

**Possible Causes:**

1. Baud rate mismatch → Verify 115200 in both code and module config
2. Noise on lines → Add 100Ω series resistors
3. Ground loops → Use single-point grounding
4. Cable too long → Keep under 30cm for breadboard

**Debug:**

- Logic analyzer will show timing errors (bits too wide/narrow)
- Try lower baud rate (9600) for testing

---

## 7. Performance Metrics

| Metric              | Value       | Notes                   |
| ------------------- | ----------- | ----------------------- |
| UART Baud Rate      | 115,200 bps | 8N1 format              |
| I2C Speed           | 400 kHz     | Fast mode               |
| Timer Period        | 1 second    | 1 Hz query rate         |
| RX Buffer Size      | 32 bytes    | Enough for AT responses |
| OLED Update         | ~10-15ms    | I2C transfer time       |
| Current Draw (Idle) | ~50mA       | Nucleo + OLED           |
| Current Draw (TX)   | ~150mA      | LoRa transmitting       |

---

## 8. Week 2 Objectives

### Goal: Build a Point-to-Point LoRa Link

**Setup:**

1. **Node A:** Nucleo + LoRa (current setup)
2. **Node B:** Second LoRa module + USB-UART adapter + PC

**Workflow:**

1. Set Node A address to 1: `AT+ADDRESS=1`
2. Set Node B address to 2: `AT+ADDRESS=2`
3. Node A sends: `AT+SEND=2,5,HELLO\r\n`
4. Node B receives: `+RCV=1,5,HELLO,-50,30` (sender, len, data, RSSI, SNR)
5. Display received message on OLED
6. Measure RSSI and range

**Success Criteria:**

- [ ] Send message from Nucleo
- [ ] Receive on PC serial terminal
- [ ] Display confirmation on OLED
- [ ] Measure signal strength
- [ ] Test range (indoor/outdoor)

---

## 9. Lessons Learned Summary

✅ **Power isolation is critical** - LoRa modules need clean, isolated power  
✅ **TX/RX crossover confusion** - Always verify with logic analyzer  
✅ **Logic analyzer is essential** - Proves hardware before debugging software  
✅ **Rust type system helps** - Explicit types catch errors at compile time  
✅ **RTIC locking is powerful** - Safe concurrency without RTOS overhead  
✅ **Documentation matters** - Save hours by documenting gotchas immediately

---

## 10. References

- [STM32F446RE Reference Manual](https://www.st.com/resource/en/reference_manual/dm00135183.pdf)
- [Nucleo-F446RE Pinout](https://os.mbed.com/platforms/ST-Nucleo-F446RE/)
- [RYLR998 Datasheet](https://reyax.com/products/rylr998/)
- [RTIC Book](https://rtic.rs/)
- [stm32f4xx-hal Documentation](https://docs.rs/stm32f4xx-hal/0.21.0/)
- [ssd1306 Driver](https://docs.rs/ssd1306/0.9.0/)

---

**Status:** ✅ Week 1 Complete - LoRa Module Verified, OLED Working, Ready for Week 2
