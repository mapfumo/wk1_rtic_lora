#![no_std]
#![no_main]

use panic_probe as _;
use defmt_rtt as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true)]
mod app {
    use stm32f4xx_hal::{
        prelude::*,
        gpio::{Output, Pin},
        pac,
        timer::{CounterHz, Event, Timer, Flag},
        serial::{Serial, Config, Event as SerialEvent},
        i2c::I2c,
    };
    use ssd1306::{prelude::*, Ssd1306, I2CDisplayInterface, mode::BufferedGraphicsMode};
    use display_interface_i2c::I2CInterface;
    use embedded_graphics::{
        mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
        pixelcolor::BinaryColor,
        prelude::*,
        text::Text,
    };
    use heapless::Vec;
    use defmt::info;

    // Correct Type Alias for ssd1306 v0.9.0
    type LoraDisplay = Ssd1306<
        I2CInterface<I2c<pac::I2C1>>, 
        DisplaySize128x64, 
        BufferedGraphicsMode<DisplaySize128x64>
    >;

    #[shared]
    struct Shared {
        lora_uart: Serial<pac::UART4>,
        display: LoraDisplay,
        rx_buffer: Vec<u8, 32>,
    }

    #[local]
    struct Local {
        led: Pin<'A', 5, Output>,
        timer: CounterHz<pac::TIM2>,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        let dp = cx.device;
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.freeze(); 

        let gpioa = dp.GPIOA.split();
        let gpiob = dp.GPIOB.split();
        let gpioc = dp.GPIOC.split();
        
        let led = gpioa.pa5.into_push_pull_output();

        // --- LoRa UART (PC10=TX, PC11=RX) ---
        let tx = gpioc.pc10.into_alternate();
        let rx = gpioc.pc11.into_alternate();
        let mut lora_uart = Serial::new(
            dp.UART4, (tx, rx),
            Config::default().baudrate(115200_u32.bps()),
            &clocks,
        ).unwrap();
        lora_uart.listen(SerialEvent::RxNotEmpty);

        // --- OLED I2C (PB8=SCL, PB9=SDA) ---
        let scl = gpiob.pb8.into_alternate_open_drain();
        let sda = gpiob.pb9.into_alternate_open_drain();
        let i2c = I2c::new(dp.I2C1, (scl, sda), 400.kHz(), &clocks);
        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().unwrap();

        // --- Timer (1Hz) ---
        let mut timer = Timer::new(dp.TIM2, &clocks).counter_hz();
        timer.start(1_u32.Hz()).unwrap(); 
        timer.listen(Event::Update);

        info!("System Live. Verified Mouth-to-Ear Wiring.");

        (Shared { 
            lora_uart, 
            display, 
            rx_buffer: Vec::new() 
        }, Local { led, timer })
    }

    #[task(binds = UART4, shared = [lora_uart, display, rx_buffer])]
    fn uart4_handler(cx: uart4_handler::Context) {
        let mut uart_res = cx.shared.lora_uart;
        let mut buffer_res = cx.shared.rx_buffer;
        let mut display_res = cx.shared.display;

        uart_res.lock(|uart| {
            if let Ok(byte) = uart.read() {
                buffer_res.lock(|buffer| {
                    if byte == b'\n' {
                        // Sentence complete!
                        if let Ok(s) = core::str::from_utf8(buffer.as_slice()) {
                            let clean_str = s.trim();
                            info!("Sentence: {}", clean_str);
                            
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

    #[task(binds = TIM2, shared = [lora_uart], local = [led, timer])]
    fn tim2_handler(mut cx: tim2_handler::Context) {
        cx.local.timer.clear_flags(Flag::Update);
        cx.local.led.toggle();

        cx.shared.lora_uart.lock(|uart| {
            // Victorious Query!
            for b in b"AT+ADDRESS?\r\n" {
                let _ = nb::block!(uart.write(*b));
            }
        });
    }
}