#![no_std]
#![no_main]

use core::{
    cell::RefCell,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

use crate::{
    controls::{control_handler, encoder_click_handler, encoder_rotate_handler},
    cv::{TICK_MS, clock_change_handler, clocked_cv_output_handler, cv_message_handler},
    display::{
        DISPLAY_REFRESH, LOADING_MESSAGE, PAGE, Page, display_handler,
    },
};
use cyw43::{JoinOptions, aligned_bytes};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::info;
use dsp_core::controls::{api_to_cv::APIToCV, clock::Clock};
use emballoc;
use embassy_executor::{InterruptExecutor, Spawner};
use embassy_net::{Config, StackResources};
use embassy_rp::{
    self, bind_interrupts,
    block::ImageDef,
    clocks::RoscRng,
    dma,
    gpio::{Input, Level, Output, Pull},
    i2c::{self, Config as I2cConfig, I2c},
    interrupt,
    interrupt::{InterruptExt, Priority},
    peripherals::{DMA_CH0, I2C1, PIO0},
    pio::{InterruptHandler, Pio},
    pwm::Pwm,
};
use embassy_sync::blocking_mutex::{Mutex as BlockingMutex, raw::CriticalSectionRawMutex};
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};
use static_cell::StaticCell;

use {defmt_rtt as _, panic_probe as _};

extern crate alloc;

mod controls;
mod cv;
mod display;
mod osc;

/// Tell the Boot ROM about our application
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: ImageDef = embassy_rp::block::ImageDef::secure_exe();

#[global_allocator]
static ALLOCATOR: emballoc::Allocator<4096> = emballoc::Allocator::new();

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

static CV_EXECUTOR: InterruptExecutor = InterruptExecutor::new();

#[interrupt]
fn UART0_IRQ() {
    unsafe {
        CV_EXECUTOR.on_interrupt();
    }
}

pub static RUNNING: AtomicBool = AtomicBool::new(true);
pub static NETWORK_CONNECTED: AtomicBool = AtomicBool::new(false);

// TODO: evaluate whether this is fine or if a more complex shared type is necessary
// 0 -> RealTime; 1 -> Clocked
pub static MODE: AtomicU8 = AtomicU8::new(0);

pub type SharedPwmOutputs = BlockingMutex<CriticalSectionRawMutex, RefCell<[Pwm<'static>; 4]>>;
pub static PWM_OUTPUTS: StaticCell<SharedPwmOutputs> = StaticCell::new();

pub type SharedClock = BlockingMutex<CriticalSectionRawMutex, RefCell<Clock>>;
pub static CLOCK: StaticCell<SharedClock> = StaticCell::new();

pub type SharedAPIToCV = BlockingMutex<CriticalSectionRawMutex, RefCell<APIToCV>>;
pub static API_TO_CV: StaticCell<SharedAPIToCV> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("=======================");
    info!("Recurse Control Voltage");
    info!("=======================");

    let page_tx = PAGE.sender();

    let peripherals = embassy_rp::init(Default::default());

    // start clock setup
    let clock = CLOCK.init(BlockingMutex::new(RefCell::new(Clock::new())));
    clock.lock(|c: &RefCell<Clock>| TICK_MS.store(c.borrow().get_tick_ms(), Ordering::Relaxed));
    spawner.spawn(clock_change_handler(clock).unwrap());
    // end clock setup

    // start CV output setup
    // 2999 -> 50kHz; 3124 -> 48kHz
    // see https://pico.implrust.com/core-concepts/pwm/frequency-to-pwm-top-divider.html
    let mut pwm_config: embassy_rp::pwm::Config = Default::default();
    pwm_config.top = 2999;
    pwm_config.divider = 1.into();
    let pwm_outputs = PWM_OUTPUTS.init(BlockingMutex::new(RefCell::new([
        Pwm::new_output_a(
            peripherals.PWM_SLICE6,
            peripherals.PIN_12,
            pwm_config.clone(),
        ),
        Pwm::new_output_a(
            peripherals.PWM_SLICE1,
            peripherals.PIN_2,
            pwm_config.clone(),
        ),
        Pwm::new_output_a(
            peripherals.PWM_SLICE2,
            peripherals.PIN_4,
            pwm_config.clone(),
        ),
        Pwm::new_output_a(
            peripherals.PWM_SLICE3,
            peripherals.PIN_6,
            pwm_config.clone(),
        ),
    ])));
    let api_to_cv = API_TO_CV.init(BlockingMutex::new(RefCell::new(APIToCV::new())));

    interrupt::UART0_IRQ.set_priority(Priority::P0);
    let cv_executor = CV_EXECUTOR.start(interrupt::UART0_IRQ);
    cv_executor.spawn(cv_message_handler(pwm_outputs, api_to_cv).unwrap());
    cv_executor.spawn(clocked_cv_output_handler(pwm_outputs, api_to_cv).unwrap());
    //end CV output setup

    // start display setup
    let sda = peripherals.PIN_14;
    let scl = peripherals.PIN_15;
    let mut i2c_config = I2cConfig::default();
    i2c_config.frequency = 400_000;

    let i2c_bus = I2c::new_async(peripherals.I2C1, scl, sda, Irqs, i2c_config);
    let i2c_interface = I2CDisplayInterface::new_custom_address(i2c_bus, 0x3D);

    let mut display = Ssd1306Async::new(i2c_interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display
        .init()
        .await
        .expect("failed to initialize the display");
    spawner.spawn(display_handler(display, api_to_cv, clock).unwrap());

    LOADING_MESSAGE
        .send("Establishing network connection")
        .await;
    DISPLAY_REFRESH.send(true).await;
    page_tx.send(Page::Loading);
    // end display setup

    // start network setup
    const NETWORK_SSID: &str = env!("NETWORK_SSID");
    const NETWORK_PASSWORD: &str = env!("NETWORK_PASSWORD");

    let mut rng = RoscRng;

    let fw = aligned_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = aligned_bytes!("../cyw43-firmware/43439A0_clm.bin");
    // this firmware is intended for the RP2350, but the CYW43 chips are the same on the Pico W and Pico 2W
    let nvram = aligned_bytes!("../cyw43-firmware/nvram_rp2040.bin");

    let pwr = Output::new(peripherals.PIN_23, Level::Low);
    let cs = Output::new(peripherals.PIN_25, Level::High);
    let mut pio = Pio::new(peripherals.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        peripherals.PIN_24,
        peripherals.PIN_29,
        dma::Channel::new(peripherals.DMA_CH0, Irqs),
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
    spawner.spawn(cyw43_task(runner).unwrap());

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());

    let seed = rng.next_u64();

    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );
    spawner.spawn(network_task(runner).unwrap());

    while let Err(err) = control
        .join(NETWORK_SSID, JoinOptions::new(NETWORK_PASSWORD.as_bytes()))
        .await
    {
        info!("join filed: {:?}", err);
    }

    info!("waiting for link...");
    stack.wait_link_up().await;

    info!("waiting for dhcp...");
    stack.wait_config_up().await;

    info!("network connection up");
    NETWORK_CONNECTED.store(true, Ordering::Relaxed);
    // end network setup

    // start control setup
    let clk = Input::new(peripherals.PIN_16, Pull::Up);
    let dt = Input::new(peripherals.PIN_17, Pull::Up);
    let sw = Input::new(peripherals.PIN_18, Pull::Up);

    spawner.spawn(encoder_rotate_handler(clk, dt).unwrap());
    spawner.spawn(encoder_click_handler(sw).unwrap());
    spawner.spawn(control_handler(api_to_cv, clock).unwrap());
    // end control setup

    LOADING_MESSAGE.send("Loading...").await;
    DISPLAY_REFRESH.send(true).await;

    page_tx.send(Page::ModeSelection);

    osc::listen(stack).await
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn network_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

// Program metadata for `picotool info`.
// This isn't needed, but it's recommended to have these minimal entries.
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"recurse-control-voltage"),
    embassy_rp::binary_info::rp_program_description!(c"your program description"),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];
