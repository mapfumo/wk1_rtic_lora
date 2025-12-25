# STM32 LoRa + OLED with RTIC

> Real-time embedded Rust: Interrupt-driven LoRa communication with OLED display feedback

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![RTIC](https://img.shields.io/badge/RTIC-2.1-blue.svg)](https://rtic.rs/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

## 📋 Overview

A complete bare-metal Rust project demonstrating interrupt-driven UART communication with a LoRa module and real-time OLED feedback. Built with RTIC (Real-Time Interrupt-driven Concurrency) framework for the STM32F446RE Nucleo board.

**Key Features:**

- ⚡ Interrupt-driven UART (no polling)
- 📡 AT command interface for LoRa modules (RYLR998, HC-12, etc.)
- 📊 Real-time OLED display updates
- 🔒 Type-safe resource sharing with RTIC
- 🐛 Comprehensive debugging with defmt + probe-rs

## 🎥 Demo

```
┌─────────────────────────┐
│  SSD1306 OLED Display   │
│                         │
│  +ADDRESS=7             │  ← Response from LoRa
│                         │
└─────────────────────────┘

LED: ●○●○●○  (1Hz heartbeat)

UART Traffic:
  TX → AT+ADDRESS?\r\n
  RX ← +ADDRESS=7\r\n
```

## 🛠️ Hardware Requirements

| Component        | Model                     | Connection        |
| ---------------- | ------------------------- | ----------------- |
| **MCU**          | STM32 Nucleo-F446RE       | -                 |
| **LoRa Module**  | RYLR998 / HC-12 / E32     | UART4 (PC10/PC11) |
| **Display**      | SSD1306 OLED 128x64       | I2C1 (PB8/PB9)    |
| **Power Supply** | MB102 Breadboard PSU      | 3.3V for LoRa     |
| **Debug**        | Logic Analyzer (optional) | Verify signals    |

### Wiring Diagram

```
STM32F446RE Nucleo          LoRa Module (RYLR998)
┌────────────────────┐      ┌─────────────────┐
│                    │      │                 │
│  PC10 (TX) ────────┼──────┼─► RXD (Ear)    │
│  PC11 (RX) ◄───────┼──────┼─── TXD (Mouth) │
│                    │      │                 │
│  3.3V ─────────┐   │      │  VCC ◄─ MB102   │
│  GND ──────┬───┼───┼──────┼─ GND            │
│            │   └───┼──┐   └─────────────────┘
│  PB8 (SCL) ─────┐  │  │
│  PB9 (SDA) ────┐│  │  │   SSD1306 OLED
│            ││  ││  │  │   ┌───────────────┐
│            ││  ││  │  └───┼─ VCC          │
│            ││  ││  └──────┼─ GND          │
│            ││  │└─────────┼─ SCL          │
│            ││  └──────────┼─ SDA          │
│            ││             └───────────────┘
└────────────────────┘

⚠️  Critical: LoRa powered from MB102, common ground required!
```

## 🚀 Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add ARM target
rustup target add thumbv7em-none-eabihf

# Install probe-rs for flashing
cargo install probe-rs --features cli
```

### Build and Flash

```bash
# Clone repository
git clone https://github.com/mapfumo/wk1_rtic_lora.git
cd wk1_rtic_lora

# Build
cargo build --release

# Flash to board
cargo run --release

# View debug output (RTT)
# In separate terminal:
probe-rs run --chip STM32F446RETx target/thumbv7em-none-eabihf/release/wk1_rtic_lora
```

### First Test

1. **Power up** - LED should blink at 1 Hz
2. **Check OLED** - Should initialize (may show previous text)
3. **Send AT command** - Module auto-queries every second
4. **Watch display** - Should show `+ADDRESS=X` response

## 📚 Documentation

| Document                                             | Description                                  |
| ---------------------------------------------------- | -------------------------------------------- |
| [WEEK1_ENGINEERING_LOG.md](WEEK1_ENGINEERING_LOG.md) | Complete bring-up story with lessons learned |
| [LORA_TROUBLESHOOTING.md](LORA_TROUBLESHOOTING.md)   | Systematic debugging guide                   |
| [NOTES.md](NOTES.md)                                 | Quick reference and gotchas                  |

## 🔧 Project Structure

```
.
├── src/
│   └── main.rs              # Main application (RTIC)
├── Cargo.toml               # Dependencies
├── memory.x                 # Linker script
├── .cargo/
│   └── config.toml          # Build configuration
└── docs/
    ├── WEEK1_ENGINEERING_LOG.md
    ├── LORA_TROUBLESHOOTING.md
    └── wiring_photos/       # Hardware setup photos
```

## 🎯 Key Features Explained

### Interrupt-Driven Architecture

```rust
// Timer fires every second
#[task(binds = TIM2, shared = [lora_uart], local = [led, timer])]
fn tim2_handler(mut cx: tim2_handler::Context) {
    cx.local.led.toggle();              // Heartbeat
    cx.shared.lora_uart.lock(|uart| {
        uart.write_all(b"AT+ADDRESS?\r\n");  // Query LoRa
    });
}

// UART interrupt fires when data received
#[task(binds = UART4, shared = [lora_uart, display, rx_buffer])]
fn uart4_handler(cx: uart4_handler::Context) {
    // Buffer incoming bytes until newline
    // Parse and display on OLED
}
```

### Safe Resource Sharing

RTIC provides compile-time guarantees:

- ✅ No data races (proven at compile time)
- ✅ No deadlocks (priority ceiling protocol)
- ✅ Zero runtime overhead (static scheduling)

```rust
#[shared]
struct Shared {
    lora_uart: Serial<pac::UART4>,      // Multiple tasks access
    display: Ssd1306<...>,               // Shared display
    rx_buffer: Vec<u8, 32>,              // Shared buffer
}
```

## 🐛 Common Issues

### "No Response from LoRa Module"

**Quick Fix:**

1. Check TX/RX wiring (should be crossover)
2. Verify common ground
3. Try different baud rate (115200 → 9600)

See [LORA_TROUBLESHOOTING.md](LORA_TROUBLESHOOTING.md) for complete diagnostic guide.

### "System Crashes When LoRa Transmits"

**Root Cause:** LoRa draws 100-150mA during TX, overloading Nucleo 3.3V rail

**Solution:** Use isolated MB102 power supply for LoRa module

- LoRa VCC → MB102 3.3V
- LoRa GND → Common GND with Nucleo
- OLED stays on Nucleo 3.3V (low current)

### "OLED Not Updating"

**Checklist:**

- [ ] Called `display.flush()` after drawing?
- [ ] I2C address correct? (try 0x3C and 0x3D)
- [ ] SDA/SCL not swapped?

## 📊 Performance Metrics

| Metric            | Value                |
| ----------------- | -------------------- |
| UART Baud Rate    | 115,200 bps          |
| I2C Speed         | 400 kHz (Fast Mode)  |
| Timer Period      | 1 Hz (AT query rate) |
| Interrupt Latency | ~2-3 µs              |
| RAM Usage         | ~2 KB (inc. buffers) |
| Flash Usage       | ~12 KB               |

## 🎓 Learning Outcomes

After completing this project, you'll understand:

- ✅ RTIC framework fundamentals
- ✅ Interrupt-driven UART communication
- ✅ I2C peripheral driver usage
- ✅ Safe concurrency in embedded Rust
- ✅ Hardware debugging with logic analyzer
- ✅ Power supply considerations for RF modules
- ✅ Real-world embedded systems troubleshooting

## 🗺️ Roadmap

### Week 1 (Current)

- [x] LoRa module bring-up (AT commands)
- [x] OLED display integration
- [x] Interrupt-driven UART
- [x] Power isolation solution

### Week 2 (Next)

- [ ] Point-to-point LoRa link
- [ ] Bidirectional messaging
- [ ] RSSI/SNR monitoring
- [ ] Range testing

### Week 3-4 (Future)

- [ ] Multi-node network
- [ ] Packet acknowledgment
- [ ] Low-power modes
- [ ] Over-the-air updates

## 🛠️ Dependencies

```toml
[dependencies]
cortex-m = "0.7"
cortex-m-rt = "0.7"
panic-probe = { version = "0.3", features = ["print-defmt"] }
stm32f4xx-hal = { version = "0.21", features = ["stm32f446"] }
rtic = { version = "2.1", features = ["thumbv7-backend"] }
ssd1306 = "0.9"
display-interface-i2c = "0.4"
embedded-graphics = "0.8"
heapless = "0.8"
defmt = "0.3"
defmt-rtt = "0.4"
```

## 🤝 Contributing

Contributions welcome! Areas of interest:

- Additional LoRa module support (SX1276, E220, etc.)
- More display drivers (ST7789, ILI9341)
- Protocol implementations (LoRaWAN, custom)
- Power consumption optimization
- Documentation improvements

## 📖 Resources

### Hardware

- [STM32F446RE Reference Manual](https://www.st.com/resource/en/reference_manual/dm00135183.pdf)
- [Nucleo-F446RE Pinout](https://os.mbed.com/platforms/ST-Nucleo-F446RE/)
- [RYLR998 Datasheet](https://reyax.com/products/rylr998/)

### Software

- [RTIC Book](https://rtic.rs/) - Framework documentation
- [Embedded Rust Book](https://docs.rust-embedded.org/book/) - General embedded Rust
- [stm32f4xx-hal Docs](https://docs.rs/stm32f4xx-hal/0.21.0/) - HAL reference

### Tools

- [probe-rs](https://probe.rs/) - Flashing and debugging
- [PulseView](https://sigrok.org/wiki/PulseView) - Logic analyzer software

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **RTIC Team** - Excellent framework for embedded Rust
- **embedded-hal** - Hardware abstraction traits
- **Rust Embedded Community** - Support and documentation

## 📧 Contact

**Author:** Antony Mapfumo  
**Project Link:** [https://github.com/mapfumo/wk1_rtic_lora](https://github.com/mapfumo/wk1_rtic_lora)

---

⭐ **Star this repo if it helped you!**

💬 **Questions?** Open an issue or check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)

🐛 **Found a bug?** Please report it with details from the troubleshooting guide
