# Rusty Alarm Clock
An alarm clock using an ESP32-C3 written with Rust and Embassy

# WIP
This project is still a work in progress

# Highlights
- Uses external RTC to keep time and to set alarms
- Connects to (S)NTP to correct RTC time
- [WIP] Remote control via Bluetooth
- [WIP] Remote control via Web Server
- [Planned] 16x2 LCD Screen
- [Planned] Deep Sleep

## Components Used
- ESP32-C3 Mini
- DS3231 RTC Module
- Breadboard Power Supply
- IRLZ44N Logic-Level MOSFET
- 3-12V Active Buzzer
- Flyback diode
- A10K Potentiometer
- A bunch of resistors and wires (ofc)


# Learning Resources
<details>
  <summary>
  The documentation and resources I had to read while making this
  project is honestly quite fragmented and all over the place.
  So here are the quick links
  </summary>


  Honestly it's a lot of going back and forth between `docs.rs` and the
  GitHub examples of the various crates. It's not uncommon for me
  to have at least 30+ tabs open. I went back and forth between:
  
  ## ESP-HAL
  - [Example project](https://github.com/claudiomattera/esp32c3-embassy): A similar project
  - [esp-hal examples](https://github.com/esp-rs/esp-hal/tree/main/examples): By far the most useful and informative
  - [embassy_sync](https://docs.embassy.dev/embassy-sync/git/default/index.html)
  - [impl Rust for ESP32](https://esp32.implrust.com/)

  ## Wifi & BLE
  - [esp-radio docs](https://docs.espressif.com/projects/rust/esp-radio/0.16.0/esp32c2/esp_radio/index.html#feature-flags)
  - [embassy-net docs](https://docs.embassy.dev/embassy-net/git/default/index.html)
  - [TrouBLE docs](https://docs.rs/trouble-host/latest/trouble_host/index.html)
  - [TrouBLE examples](https://github.com/embassy-rs/trouble/tree/main/examples/esp32/src/bin)
  - [smoltcp docs](https://docs.rs/smoltcp/latest/smoltcp/): Most logic is done through `embassy-net` that reuses some `smoltcp` types
  - [Website GET](https://esp32.implrust.com/wifi/embassy/async-access-website.html)
  - [TrouBLE Starting Code](https://esp32.implrust.com/bluetooth/trouble/index.html)

  ### DHCP
  - [esp-hal sntpc example](https://github.com/esp-rs/esp-hal/blob/9e4c652d1aa1d1cbc8f2483c93b7d98d0ba1bcb6/examples/wifi/embassy_sntp/src/main.rs#L103): This single line handles DHCP for embassy
  
  ### DNS
  - [esp-hal sntpc example](https://github.com/esp-rs/esp-hal/blob/9e4c652d1aa1d1cbc8f2483c93b7d98d0ba1bcb6/examples/wifi/embassy_sntp/src/main.rs#L140C1-L140C81): Also a single line (assuming you setup Stack correctly)
  - ~~[smoltcp DNS example](https://github.com/smoltcp-rs/smoltcp/blob/main/examples/dns.rs)~~: Not that useful in my case

  ### (S)NTP (Simple Network Time Protocol)
  Sync our alaram clock to correct time
  - [esp-hal sntpc embassy](https://github.com/esp-rs/esp-hal/blob/main/examples/wifi/embassy_sntp/src/main.rs#L158)
  - [sntpc docs](https://docs.rs/sntpc/latest/sntpc/)
  - [sntpc embassy example](https://github.com/vpetrigo/sntpc/blob/master/examples/embassy-net/src/main.rs): Example uses std but useful for learning how to implement traits

  ## Web Server
  - [picoserve embassy example](https://github.com/sammhicks/picoserve/blob/development/examples/embassy/hello_world/src/main.rs)

  ## DS3231
  Our nifty external RTC module
  - [ds3231 docs](https://docs.rs/ds3231/latest/ds3231/)

  ## Time
  Timezones are pain
  - [jiff Timezones](https://docs.rs/jiff/latest/jiff/tz/struct.TimeZone.html): Jiff is so much nicer and ergonomic to work with compared to chrono
  - [chrono Datetime](https://docs.rs/chrono/latest/chrono/struct.DateTime.html)
  - [chrono NaiveDatetime](https://docs.rs/chrono/latest/chrono/struct.NaiveDateTime.html)

  It seems rather daunting but I was able to do most of the wireless
  features in 2 days. From zero knowledge about GATT, interfaces, SNTP and stacks,
  to barely enough to use them.
</details>

