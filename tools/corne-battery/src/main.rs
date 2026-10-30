//! Prints both Corne halves' battery levels, read over USB from the dongle: feature report 1 of
//! the vendor HID interface added by src/battery_hid.c (dongle firmware, CONFIG_CORNE_BATTERY_HID).
//!
//! Usage: corne-battery [--swap]
//!   --swap  when "left" shows the right half's level. ZMK numbers the halves in the order they
//!           first paired with the dongle; the left nice!view shows the real left level.

use hidapi::HidApi;

const VID: u16 = 0x1d50; // ZMK's default USB IDs
const PID: u16 = 0x615e;
const USAGE_PAGE: u16 = 0xff00;
const REPORT_ID: u8 = 1;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let swap = std::env::args().any(|a| a == "--swap");
    let api = HidApi::new()?;
    let info = api
        .device_list()
        .find(|d| d.vendor_id() == VID && d.product_id() == PID && d.usage_page() == USAGE_PAGE)
        .ok_or("dongle battery interface not found: is the dongle plugged in and running firmware built with CONFIG_CORNE_BATTERY_HID?")?;
    let mut buf = [0u8; 3];
    buf[0] = REPORT_ID;
    let n = info.open_device(&api)?.get_feature_report(&mut buf)?;
    println!("{}", describe(&buf[..n], swap));
    Ok(())
}

/// `report` = [report id, slot 0 level, slot 1 level]; levels are 0-100, 0xFF = not reported yet.
fn describe(report: &[u8], swap: bool) -> String {
    let level = |slot: usize| match report.get(1 + slot) {
        Some(&l) if l <= 100 => format!("{l}%"),
        _ => "?".into(),
    };
    let (left, right) = if swap { (1, 0) } else { (0, 1) };
    format!("left {}  right {}", level(left), level(right))
}

#[cfg(test)]
mod tests {
    #[test]
    fn describe() {
        assert_eq!(super::describe(&[1, 80, 0xFF], false), "left 80%  right ?");
        assert_eq!(super::describe(&[1, 80, 55], true), "left 55%  right 80%");
        assert_eq!(super::describe(&[1], false), "left ?  right ?");
    }
}
