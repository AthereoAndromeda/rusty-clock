# Rusty Alarm Clock
An alarm clock using an ESP32-C3 written with Rust and Embassy

# WIP
This project is still a work in progress

## Components Used
- ESP32-C3 Mini
- IRLZ44N Logic-Level MOSFET
- 3-12V Active Buzzer
- A10K Potentiometer

# Learning Resources
<details>
  <summary>
  The documentation and resources I had to read while making this
  project is honestly quite fragmented and all over the place.
  So here are the quick links
  </summary>

  
  ## ESP-HAL
  -

  ## Wifi

  ### DNS
  - [smoltcp DNS example](https://github.com/smoltcp-rs/smoltcp/blob/main/examples/dns.rs)

  ### NTP (Network Time Protocol)
  Sync our alaram clock to correct time

  - [sntpc docs](https://docs.rs/sntpc/latest/sntpc/)
  - [sntpc embassy example](https://github.com/vpetrigo/sntpc/blob/master/examples/embassy-net/src/main.rs): Example uses std but useful for learning how to implement traits

  ## Bluetooth

  ## DS3231
</details>

