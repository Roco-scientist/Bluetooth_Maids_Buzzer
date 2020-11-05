#![no_main]
#![no_std]

#[cfg(debug_assertions)]
use panic_itm as _;

#[cfg(not(debug_assertions))]
use panic_abort as _;

use cortex_m::NVIC;
use rtic::app;
use stm32f4xx_hal::{
    block,
    gpio::{gpiob::PB10, Input, PullDown},
    prelude::*,
    serial::{config, Rx, Serial, Tx},
    stm32, time,
};

#[rtic::app(device = stm32f4xx_hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        bluetooth_tx: Tx<stm32::USART1>,
        bluetooth_rx: Rx<stm32::USART1>,
        button: PB10<Input<PullDown>>,
    }
    #[init()]
    fn init(cx: init::Context) -> init::LateResources {
        // pulling peripherals
        let peripherals: stm32::Peripherals = cx.device;
        // enable syscfg for interrupt below
        peripherals.RCC.apb2enr.write(|w| w.syscfgen().set_bit());
        // using rcc
        let rcc = peripherals.RCC.constrain();

        // clock for usart1 timiing
        let clocks = rcc.cfgr.freeze();

        // setup gpioa for the tx and rx pins for the HC-05 bluetooth board
        let gpioa = peripherals.GPIOA.split();
        // setup gpiob for the button
        let gpiob = peripherals.GPIOB.split();

        // create pull down input button pin on pb2
        let button = gpiob.pb10.into_pull_down_input();

        // set pb10 as an external rising trigger interrupt
        // sets the rtsr at an offset of 8
        // make button push into an interrupt
        let syscfg = &peripherals.SYSCFG;
        syscfg.exticr3.write(|w| unsafe { w.exti10().bits(0b0001) }); // per the manual 001 indicates pb2 on exti2

        // from: https://flowdsp.io/blog/stm32f3-01-interrupts/
        let exti = &peripherals.EXTI;
        exti.imr3.modify(|_, w| w.mr10().set_bit()); // unmask interrupt
        exti.rtsr3.modify(|_, w| w.tr10().set_bit()); // trigger on rising-edge

        // connect the interrupt to NVIC
        unsafe { NVIC::unmask(stm32::interrupt::EXTI3) };

        // create tx and rx pins with alternative funcction 7
        // USART1 is found as AF07 within datasheet
        let usart1_txd = gpioa.pa9.into_alternate_af7();
        let usart1_rxd = gpioa.pa10.into_alternate_af7();

        // setup bluetooth config
        let bluetooth_config = config::Config {
            baudrate: time::Bps(9600),
            wordlength: config::WordLength::DataBits8,
            parity: config::Parity::ParityNone,
            stopbits: config::StopBits::STOP1,
        };

        // setup usart with tx and rx pins, along with bus and clocks
        let usart1 = Serial::usart1(
            peripherals.USART1,
            (usart1_txd, usart1_rxd),
            bluetooth_config,
            clocks,
        )
        .unwrap();

        // split out the tx and rx communication of the bluetooth
        let (bluetooth_tx, bluetooth_rx) = usart1.split();
        init::LateResources {
            bluetooth_tx,
            bluetooth_rx,
            button,
        }
    }

    #[task(binds = EXTI3, resources = [button, bluetooth_tx])]
    fn exti_3_10_interrupt(ctx: exti_3_10_interrupt::Context) {
        // When button is pressed send a BUZZ signal
        for byte in b"BUZZ" {
            block!(ctx.resources.bluetooth_tx.write(*byte)).unwrap();
        }
        block!(ctx.resources.bluetooth_tx.flush()).unwrap();
    }
};
