/*
 * Dongle only: a second, vendor-defined USB HID interface whose feature report holds each split
 * peripheral's battery level, so tools/corne-battery can read it without Bluetooth.
 *
 * Feature report 1: [0x01, level of peripheral 0, level of peripheral 1, ...]; 0-100 %, 0xFF =
 * not reported yet. Peripheral numbers are ZMK's slots (the order the halves first paired).
 */

#include <zephyr/device.h>
#include <zephyr/init.h>
#include <zephyr/usb/usb_device.h>
#include <zephyr/usb/class/usb_hid.h>

#include <zmk/event_manager.h>
#include <zmk/events/battery_state_changed.h>

#define REPORT_ID 1
#define PERIPHERALS CONFIG_ZMK_SPLIT_BLE_CENTRAL_PERIPHERALS

static const uint8_t report_desc[] = {
    0x06, 0x00, 0xFF,  // Usage Page (Vendor Defined 0xFF00)
    0x09, 0x01,        // Usage (0x01)
    0xA1, 0x01,        // Collection (Application)
    0x85, REPORT_ID,   //   Report ID
    0x09, 0x02,        //   Usage (0x02)
    0x15, 0x00,        //   Logical Minimum (0)
    0x26, 0xFF, 0x00,  //   Logical Maximum (255)
    0x75, 0x08,        //   Report Size (8)
    0x95, PERIPHERALS, //   Report Count
    0xB1, 0x02,        //   Feature (Data, Variable, Absolute)
    0xC0,              // End Collection
};

static uint8_t report[1 + PERIPHERALS] = {REPORT_ID, [1 ... PERIPHERALS] = 0xFF};

static int get_report_cb(const struct device *dev, struct usb_setup_packet *setup, int32_t *len,
                         uint8_t **data) {
    // wValue: report type (3 = feature) in the high byte, report ID in the low byte
    if ((setup->wValue >> 8) != 3 || (setup->wValue & 0xFF) != REPORT_ID) {
        return -ENOTSUP;
    }
    *data = report;
    *len = sizeof(report);
    return 0;
}

static const struct hid_ops ops = {.get_report = get_report_cb};

static int battery_listener(const zmk_event_t *eh) {
    const struct zmk_peripheral_battery_state_changed *ev =
        as_zmk_peripheral_battery_state_changed(eh);
    if (ev && ev->source < PERIPHERALS) {
        report[1 + ev->source] = ev->state_of_charge;
    }
    return ZMK_EV_EVENT_BUBBLE;
}

ZMK_LISTENER(corne_battery_hid, battery_listener);
ZMK_SUBSCRIPTION(corne_battery_hid, zmk_peripheral_battery_state_changed);

// Same priority as ZMK's own HID_0 registration: both must happen before usb_enable().
static int battery_hid_init(void) {
    const struct device *hid = device_get_binding("HID_1");
    if (hid == NULL) {
        return -ENODEV;
    }
    usb_hid_register_device(hid, report_desc, sizeof(report_desc), &ops);
    return usb_hid_init(hid);
}

SYS_INIT(battery_hid_init, APPLICATION, CONFIG_ZMK_USB_HID_INIT_PRIORITY);
