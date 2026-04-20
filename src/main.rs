//! # Rusty-Clock
//! An Embassy powered alarm clock.
//!
//! WARN: I accidentally shorted out my GPIO9, destroying the LED but somehow it still works??
//! I'm assuming the LED acted like a fuse, however the GPIO9 might be damaged in some way
//! that I am not aware of.

#![no_std]
#![no_main]
#![recursion_limit = "256"]
#![feature(
    decl_macro,
    strict_provenance_lints,
    // NIGHTLY: Required by Picoserve
    impl_trait_in_assoc_type,
    // NIGHTLY: Allows env vars to be parsed at compile time
    const_result_trait_fn,
    const_trait_impl,
    const_option_ops,
    const_index,
    const_convert,
)]
// Clippy Lints
#![forbid(
    clippy::infinite_loop,
    reason = "Force usage of ! to denote infinite loops."
)]
#![forbid(
    clippy::undocumented_unsafe_blocks,
    reason = "All unsafe blocks must be documented."
)]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(
    clippy::indexing_slicing,
    reason = "Prefer `.get()` unless absolutely sure index cannot be out of bounds."
)]
#![deny(
    clippy::as_conversions,
    reason = "`as` conversions are not explicit enough."
)]
#![deny(
    clippy::integer_division,
    reason = "Integer divison discards the remainder"
)]
#![deny(
    clippy::map_with_unused_argument_over_ranges,
    clippy::empty_enum_variants_with_brackets,
    clippy::empty_structs_with_brackets,
    clippy::get_unwrap,
    clippy::large_stack_frames,
    clippy::lossy_float_literal,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::char_lit_as_u8,
    clippy::fn_to_numeric_cast,
    clippy::fn_to_numeric_cast_with_truncation,
    clippy::ptr_as_ptr,
    clippy::arithmetic_side_effects,
    clippy::string_slice,
    clippy::as_pointer_underscore,
    clippy::assertions_on_result_states,
    clippy::big_endian_bytes,
    clippy::cfg_not_test,
    clippy::empty_drop,
    clippy::fn_to_numeric_cast_any,
    clippy::multiple_inherent_impl,
    clippy::ref_patterns
)]
// CLIPPY: Use pedantic
#![warn(clippy::allow_attributes, reason = "Prefer `expect` macros")]
#![warn(
    clippy::allow_attributes_without_reason,
    reason = "All `allow/expect` macros should be documented"
)]
#![warn(
    clippy::pedantic,
    clippy::doc_paragraphs_missing_punctuation,
    clippy::unused_trait_names,
    clippy::semicolon_if_nothing_returned,
    clippy::needless_raw_strings,
    clippy::clone_on_ref_ptr,
    clippy::if_then_some_else_none,
    clippy::missing_assert_message,
    clippy::decimal_literal_representation,
    clippy::doc_include_without_cfg,
    clippy::module_name_repetitions,
    clippy::renamed_function_params,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::string_lit_chars_any,
    fuzzy_provenance_casts,
    lossy_provenance_casts
)]
#![allow(clippy::inline_always, reason = "Lessen bloat")]
#![allow(clippy::similar_names, reason = "Using TX/RX naming convention")]
#![allow(clippy::large_futures, reason = "Cannot use heap or `Box::pin`")]
#![allow(
    clippy::ok_expect,
    reason = "`Result::unwrap` is not const fn, while `Option::unwrap` is. \
    Thus it is necessary to convert `Result` to `Option` in const contexts if we want to avoid using unsafe."
)]

extern crate alloc;

mod buzzer;
mod i2c;
mod lcd;
mod priority_command;
mod pwm;
mod rtc_ds3231;
mod utils;
mod wireless;

// use defmt_rtt as _;
// use esp_backtrace as _;
// use esp_println as _;
use panic_rtt_target as _;

use defmt::info;
use embassy_executor::Spawner;
#[cfg(target_arch = "riscv32")]
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::{
    clock::CpuClock,
    gpio::{Output, OutputConfig, Pin as _},
    timer::timg::TimerGroup,
};

use crate::pwm::Channels;

// NOTE: Using TZ_OFFSET since IANA Timezones adds unnecessary weight
// PERF: Faster and leaner than LazyLock if you're
// okay with using nightly features
pub(crate) const TZ_OFFSET: i8 = {
    let tz = option_env!("TZ_OFFSET").unwrap_or("0");

    // This also works
    // match i8::from_str_radix(tz, 10) {
    //     Ok(n) => n,
    //     Err(_) => {
    //         panic!("Failed to parse .env: TZ_OFFSET");
    //     }
    // }

    // We convert to `Option` since `Result::unwrap` is not const fn,
    // but `Option::unwrap` is.
    i8::from_str_radix(tz, 10)
        .ok()
        .expect("Failed to parse .env: TZ_OFFSET")
};

// TEST: Within UTC Offset range
static_assertions::const_assert!(TZ_OFFSET <= 12 && TZ_OFFSET >= -12);
esp_bootloader_esp_idf::esp_app_desc!();

/// Represent time since boot.
pub(crate) static BOOT_TIME: esp_hal::time::Instant = esp_hal::time::Instant::EPOCH;

// #[expect(
//     clippy::large_stack_frames,
//     reason = "it's not unusual to allocate larger buffers etc. in main"
// )]
#[esp_rtos::main]
async fn main(spawner: Spawner) {
    rtt_target::rtt_init_defmt!();
    defmt::timestamp!("[{=u64}ms]", BOOT_TIME.elapsed().as_millis());

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Previously 66320
    // Any lower may cause issues with WiFi/BLE connections
    esp_alloc::heap_allocator!(#[unsafe(link_section = ".dram2_uninit")] size: 60000);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(size: 32 * 1024); // Previously 64 * 1024

    #[cfg(target_arch = "riscv32")]
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(
        timg0.timer0,
        #[cfg(target_arch = "riscv32")]
        sw_int.software_interrupt0,
    );

    info!("ESP-RTOS Started!");

    info!("Init I2C...");
    let [i2c_rtc, i2c_lcd] = i2c::init(
        peripherals.I2C0,
        peripherals.GPIO2.degrade(),
        peripherals.GPIO3.degrade(),
    );

    info!("Init LCD Display...");
    lcd::init(spawner, i2c_lcd);

    info!("Init PWM/LEDC...");
    let output = Output::new(
        peripherals.GPIO5,
        esp_hal::gpio::Level::Low,
        OutputConfig::default().with_drive_strength(esp_hal::gpio::DriveStrength::_5mA),
    );

    let Channels { channel0 } = pwm::init(peripherals.LEDC);
    let chan0 = channel0.with_output(output);

    info!("Init Buzzer...");
    buzzer::init(
        spawner,
        chan0,
        peripherals.GPIO7.degrade(),
        peripherals.GPIO6.degrade(),
    );

    info!("Init Wireless...");
    wireless::init(
        spawner,
        peripherals.WIFI,
        #[cfg(feature = "ble")]
        peripherals.BT,
    );

    // Keep RTC Init last to avoid blocking other inits
    info!("Init RTC...");
    rtc_ds3231::init(spawner, i2c_rtc).await;

    info!("All System Tasks Spawned!");
}
