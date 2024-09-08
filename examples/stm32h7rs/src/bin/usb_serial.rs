#![no_std]
#![no_main]

use defmt::{panic, *};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_stm32::rcc::Hse;
use embassy_stm32::time::Hertz;
use embassy_stm32::usb::{Driver, Instance};
use embassy_stm32::{bind_interrupts, peripherals, usb, Config};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::Builder;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
OTG_HS => usb::InterruptHandler<peripherals::USB_OTG_HS>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Hello World!");

    let mut config = Config::default();

    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse { freq: Hertz(24_000_000), mode: HseMode::Oscillator });
        config.rcc.hsi = Some(HSIPrescaler::DIV1);
        config.rcc.hsi48 = Some(Hsi48Config { sync_from_usb: true });
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSE,
            prediv: PllPreDiv::DIV12,
            mul: PllMul::MUL300,
            divp: Some(PllDiv::DIV1), //600 MHz
            divq: Some(PllDiv::DIV2), // 300 MHz
            divr: Some(PllDiv::DIV2), // 300 MHz
        });
        config.rcc.sys = Sysclk::PLL1_P; // 600 MHz
        config.rcc.ahb_pre = AHBPrescaler::DIV2; // 300 MHz
        config.rcc.apb1_pre = APBPrescaler::DIV2; // 150 MHz
        config.rcc.apb2_pre = APBPrescaler::DIV2; // 150 MHz
        config.rcc.apb4_pre = APBPrescaler::DIV2; // 150 MHz
        config.rcc.apb5_pre = APBPrescaler::DIV2; // 150 MHz
        config.rcc.voltage_scale = VoltageScale::HIGH;
        config.rcc.mux.usb_otg_fssel = mux::UsbOtgFssel::HSI48;
        config.rcc.pll2 = Some(Pll {
            source: PllSource::HSE,
            prediv: PllPreDiv::DIV12,
            mul: PllMul::MUL200,
            divp: Some(PllDiv::DIV2), // 200 MHz
            divq: Some(PllDiv::DIV2), // 200 MHz
            divr: Some(PllDiv::DIV2), // 200 MHz
        });
    }

    let p = embassy_stm32::init(config);

    // Create the driver, from the HAL.
    let mut ep_out_buffer = [0u8; 256];
    let mut config = embassy_stm32::usb::Config::default();

    config.vbus_detection = false;

    let driver = Driver::new_hs(p.USB_OTG_HS, Irqs, p.PM6, p.PM5, &mut ep_out_buffer, config);

    // Create embassy-usb Config
    let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("USB-serial example");
    config.serial_number = Some("12345678");
    // Required for windows compatibility.
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    info!("RILEYHERE");

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [], // no msos descriptors
        &mut control_buf,
    );

    // Create classes on the builder.
    let mut class = CdcAcmClass::new(&mut builder, &mut state, 64);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    info!("RILEYHERE2");

    // Do stuff with the class!
    let echo_fut = async {
        loop {
            info!("RILEYHERE3");
            class.wait_connection().await;
            info!("Connected");
            let _ = echo(&mut class).await;
            info!("Disconnected");
        }
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, echo_fut).await;
}

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn echo<'d, T: Instance + 'd>(class: &mut CdcAcmClass<'d, Driver<'d, T>>) -> Result<(), Disconnected> {
    let mut buf = [0; 64];
    loop {
        let n = class.read_packet(&mut buf).await?;
        let data = &buf[..n];
        info!("data: {:x}", data);
        class.write_packet(data).await?;
    }
}
