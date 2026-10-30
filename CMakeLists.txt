# Code built into the ZMK app from this repo (it's a Zephyr module, see zephyr/module.yml).
target_sources_ifdef(CONFIG_INPUT_AZOTEQ_IQS5XX app PRIVATE drivers/input/iqs5xx.c)
target_sources_ifdef(CONFIG_CORNE_BATTERY_HID app PRIVATE src/battery_hid.c)
