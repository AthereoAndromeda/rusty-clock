use bilge::prelude::*;
use embassy_time::Timer;
use explicit_cast::Widen as _;
use pcf857x::PcAsync;

use crate::i2c::I2cBus;

/// Our concrete implementation of [`lcd::Hardware`].
pub(crate) struct LcdHardware {
    driver: PcAsync<I2cBus>,
    register: LcdRegister,
}

impl LcdHardware {
    /// Create a new instance of [`LcdHardware`].
    pub fn new(driver: PcAsync<I2cBus>) -> Self {
        LcdHardware {
            driver,
            register: LcdRegister::from(0),
        }
    }
}

pub(super) type LcdDisplay = lcd::Display<LcdHardware>;

#[bilge::bitsize(8)]
#[derive(FromBits, Clone, Copy)]
struct LcdRegister {
    pin_0: bool,
    pin_1: bool,
    pin_2: bool,
    pin_3: bool,
    data: u4,
}

impl lcd::Hardware for LcdHardware {
    async fn rs(&mut self, bit: bool) {
        self.register.set_pin_0(bit);
        self.driver.set(self.register.into()).await.unwrap();
    }

    async fn enable(&mut self, bit: bool) {
        self.register.set_pin_2(bit);
        self.driver.set(self.register.into()).await.unwrap();
    }

    async fn data(&mut self, data: u8) {
        let data = u4::from_u8(data);
        self.register.set_data(data);
        self.driver.set(self.register.into()).await.unwrap();
    }

    async fn wait_address(&mut self) {
        Timer::after_nanos(50).await;
    }

    async fn mode(&self) -> lcd::FunctionMode {
        lcd::FunctionMode::Bit4
    }

    // async fn can_read(&self) -> bool {
    //     true
    // }

    // fn rw(&mut self, _bit: bool) {
    //     unimplemented!()
    // }

    // fn read_data(&mut self) -> u8 {
    //     unimplemented!()
    // }

    // async fn apply(&mut self) {}
}

impl lcd::Delay for LcdHardware {
    async fn delay_us(&mut self, delay_usec: u32) {
        Timer::after_micros(delay_usec.widen()).await;
    }
}

impl lcd::Backlight for LcdHardware {
    async fn set_backlight(&mut self, enabled: bool) {
        self.register.set_pin_3(enabled);
        self.driver.set(self.register.into()).await.unwrap();
    }
}
