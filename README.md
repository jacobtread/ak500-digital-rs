# AK500 DIGITIAL RS

WIP Open Source system monitor for the DEEPCOOL AK500 DIGITAL CPU display with the main purpose to 
be driving the display under linux (But also boasts not having to run electron and JS just for a CPU monitor)


## Linux Notes

I have only tested development on Fedora, you will need to adjust these to be relevant for your distro

## Dependencies

libudev - Used for USB device access

```sh
sudo dnf install libudev-devel 
```

## Unprivileged USB access

By default linux will not allow access to HID devices without sudo which makes it hard to debug and develop this program.

You can allow unprivileged access to the UPS HID device by doing the following

Create a rules file at `/etc/udev/rules.d/50-ak500-digitial.rules` in this file put the following line:

```
KERNEL=="hidraw*", ATTRS{idVendor}=="3633", ATTRS{idProduct}=="0003", TAG+="uaccess"
```

Then, replug your device or run:

```sh
sudo udevadm control --reload-rules && sudo udevadm trigger
```

You should now be able to run the program without privileges