
async fn init_wifi() {
    let (mut wifi_controller, interfaces) = esp_radio::wifi::new(, device, config)
}
