//! Tray icon showing both Corne halves' battery levels (top = left, bottom = right), read over
//! USB from the dongle: feature report 1 of the vendor HID interface added by src/battery_hid.c
//! (dongle firmware, CONFIG_CORNE_BATTERY_HID). Windows only.
//!
//! Usage: corne-battery [--swap] [--print]
//!   --swap   when "left" shows the right half's level. ZMK numbers the halves in the order they
//!            first paired with the dongle; the left nice!view shows the real left level.
//!   --print  print the levels once and exit, instead of the tray icon
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // no console window

use std::error::Error;
use std::time::{Duration, Instant};

use hidapi::HidApi;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIconBuilder};

const VID: u16 = 0x1d50; // ZMK's default USB IDs
const PID: u16 = 0x615e;
const USAGE_PAGE: u16 = 0xff00;
const REPORT_ID: u8 = 1;
// The halves report to the dongle every CONFIG_ZMK_BATTERY_REPORT_INTERVAL (60 s by default).
const REFRESH: Duration = Duration::from_secs(30);

/// [left, right] in percent; None = dongle hasn't heard from that half yet.
type Levels = [Option<u8>; 2];

fn main() -> Result<(), Box<dyn Error>> {
    let swap = std::env::args().any(|a| a == "--swap");
    if std::env::args().any(|a| a == "--print") {
        // Release builds have no console of their own; borrow the terminal we were started from.
        unsafe { windows_sys::Win32::System::Console::AttachConsole(u32::MAX) };
        println!("{}", describe(read_levels(&HidApi::new()?, swap)?));
        return Ok(());
    }

    let mut api = HidApi::new()?;
    let menu = Menu::new();
    let quit = MenuItem::new("Quit", true, None);
    menu.append(&quit)?;
    let tray = TrayIconBuilder::new().with_menu(Box::new(menu)).build()?;

    let mut next_refresh = Instant::now();
    loop {
        pump_messages();
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == *quit.id() {
                return Ok(());
            }
        }
        if Instant::now() >= next_refresh {
            next_refresh = Instant::now() + REFRESH;
            let _ = api.refresh_devices(); // pick up a re-plugged dongle
            let (levels, tooltip) = match read_levels(&api, swap) {
                Ok(levels) => (levels, format!("Corne: {}", describe(levels))),
                Err(_) => ([None, None], "Corne: dongle not found".to_string()),
            };
            tray.set_icon(Some(Icon::from_rgba(render(levels), ICON, ICON)?))?;
            tray.set_tooltip(Some(tooltip))?;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn read_levels(api: &HidApi, swap: bool) -> Result<Levels, Box<dyn Error>> {
    let info = api
        .device_list()
        .find(|d| d.vendor_id() == VID && d.product_id() == PID && d.usage_page() == USAGE_PAGE)
        .ok_or("dongle battery interface not found: is the dongle plugged in and running firmware built with CONFIG_CORNE_BATTERY_HID?")?;
    let mut buf = [REPORT_ID, 0, 0];
    let n = info.open_device(api)?.get_feature_report(&mut buf)?;
    let [a, b] = parse(&buf[..n]);
    Ok(if swap { [b, a] } else { [a, b] })
}

/// `report` = [report id, slot 0 level, slot 1 level]; levels are 0-100, 0xFF = not reported yet.
fn parse(report: &[u8]) -> Levels {
    let level = |slot: usize| report.get(1 + slot).copied().filter(|&l| l <= 100);
    [level(0), level(1)]
}

fn describe([left, right]: Levels) -> String {
    let fmt = |l: Option<u8>| l.map_or("?".to_string(), |l| format!("{l}%"));
    format!("left {}  right {}", fmt(left), fmt(right))
}

// Tray tooltips/menus need this thread's Win32 message queue serviced.
fn pump_messages() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
    };
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

const ICON: u32 = 32;
// 3x5 pixel digits 0-9, then '?'; each row is 3 bits, MSB = leftmost pixel.
const FONT: [[u8; 5]; 11] = [
    [7, 5, 5, 5, 7], [2, 6, 2, 2, 7], [7, 1, 7, 4, 7], [7, 1, 7, 1, 7], [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7], [7, 4, 7, 5, 7], [7, 1, 1, 1, 1], [7, 5, 7, 5, 7], [7, 5, 7, 1, 7],
    [7, 1, 3, 0, 2],
];

/// 32x32 RGBA: dark badge, left level on top, right level below, red at 20 % or less.
/// Digits are drawn at 2x so Windows' 16 px tray size halves them back to crisp pixels.
fn render(levels: Levels) -> Vec<u8> {
    let size = ICON as usize;
    let mut px = [0x20, 0x20, 0x20, 0xff].repeat(size * size);
    for (row, level) in levels.into_iter().enumerate() {
        let text = level.map_or("?".to_string(), |l| l.to_string());
        let color = if level.is_some_and(|l| l <= 20) { [0xff, 0x50, 0x50, 0xff] } else { [0xff; 4] };
        let x0 = (size - (text.len() * 8 - 2)) / 2; // glyph 6 px + 2 px gap
        let y0 = 4 + row * 14;
        for (i, ch) in text.bytes().enumerate() {
            let glyph = FONT[if ch == b'?' { 10 } else { (ch - b'0') as usize }];
            for (gy, bits) in glyph.iter().enumerate() {
                for gx in (0..3).filter(|gx| bits >> (2 - gx) & 1 == 1) {
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let (x, y) = (x0 + i * 8 + gx * 2 + dx, y0 + gy * 2 + dy);
                        px[(y * size + x) * 4..][..4].copy_from_slice(&color);
                    }
                }
            }
        }
    }
    px
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_and_text() {
        assert_eq!(parse(&[1, 80, 0xFF]), [Some(80), None]);
        assert_eq!(parse(&[1]), [None, None]);
        assert_eq!(describe([Some(55), Some(80)]), "left 55%  right 80%");
        assert_eq!(describe([Some(80), None]), "left 80%  right ?");
    }

    #[test]
    fn icon() {
        let lit = |px: &[u8], rows: std::ops::Range<usize>| {
            rows.flat_map(|y| (0..32).map(move |x| (y, x)))
                .filter(|&(y, x)| px[(y * 32 + x) * 4] == 0xff)
                .count()
        };
        // widest text and '?' stay inside 32x32 (would panic otherwise)
        let px = render([Some(100), None]);
        assert_eq!(px.len(), 32 * 32 * 4);
        assert!(lit(&px, 4..14) > 0 && lit(&px, 18..28) > 0);
        assert_eq!(lit(&px, 0..4) + lit(&px, 14..18) + lit(&px, 28..32), 0);
        // low battery is red: red channel lit, green not
        let low = render([Some(5), Some(90)]);
        let (y, x) = (4..14).flat_map(|y| (0..32).map(move |x| (y, x)))
            .find(|&(y, x)| low[(y * 32 + x) * 4] == 0xff).unwrap();
        assert_eq!(low[(y * 32 + x) * 4 + 1], 0x50);
    }
}
