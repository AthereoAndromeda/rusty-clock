use bilge::prelude::*;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;
use explicit_cast::Widen as _;
use pcf857x::PcAsync;

/// Our concrete implementation of [`lcd::Hardware`].
pub(crate) struct LcdHardware<B: I2c> {
    driver: PcAsync<B>,
    last_mask: u8,
}

impl<B: I2c> LcdHardware<B> {
    /// Create a new instance of [`LcdHardware`].
    pub fn new(driver: PcAsync<B>) -> Self {
        LcdHardware {
            driver,
            last_mask: 0,
        }
    }
}

#[bilge::bitsize(8)]
#[derive(TryFromBits)]
enum Pins {
    P0 = 0b0000_0001,
    P1 = 0b0000_0010,
    P2 = 0b0000_0100,
    P3 = 0b0000_1000,
    P4 = 0b0001_0000,
    P5 = 0b0010_0000,
    P6 = 0b0100_0000,
    P7 = 0b1000_0000,
}

impl<B: I2c> lcd::Hardware for LcdHardware<B> {
    async fn rs(&mut self, bit: bool) {
        let bit = u8::from(bit);
        let mask = (self.last_mask & !u8::from(Pins::P0)) | bit;
        self.driver.set(mask).await.unwrap();
        self.last_mask = mask;
    }

    async fn enable(&mut self, bit: bool) {
        let bit = u8::from(bit);
        let mask = (self.last_mask & !u8::from(Pins::P2)) | bit << 2;
        self.driver.set(mask).await.unwrap();
        self.last_mask = mask;
    }

    async fn data(&mut self, data: u8) {
        let new_mask = (self.last_mask & 0b0000_1111) | (data << 4);
        self.driver.set(new_mask).await.unwrap();
        self.last_mask = new_mask;
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

impl<B: I2c> lcd::Delay for LcdHardware<B> {
    async fn delay_us(&mut self, delay_usec: u32) {
        Timer::after_micros(delay_usec.widen()).await;
    }
}

impl<B: I2c> lcd::Backlight for LcdHardware<B> {
    async fn set_backlight(&mut self, enabled: bool) {
        let enable_bit = u8::from(enabled);
        // Clear the 3rd bit, then OR enable bit
        let mask = (self.last_mask & !(1 << 3)) | enable_bit << 3;
        self.driver.set(mask).await.unwrap();
        self.last_mask = mask;
    }
}
