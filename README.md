# AK500 DIGITAL RS

WIP Open Source system monitor for the DEEPCOOL AK500 DIGITAL CPU display with the main purpose to 
be driving the display under linux (But also boasts not having to run electron and JS just for a CPU monitor)

Only supports linux at the moment, library for CPU temps doesn't support windows yet.

## Installation

Below are the instructions for installing:

```sh
# Build release binary
cargo build --release

# Copy release binary to /usr/local/bin
sudo cp ./target/release/ak500-digital-rs /usr/local/bin/ak500-digital

# Create the config directory
sudo mkdir /etc/ak500-digital

# Copy the example config
sudo cp ./example-config.toml /etc/ak500-digital/config.toml

# Copy service to systemd
sudo cp ./ak500-digital.service /etc/systemd/system/ak500-digital.service

# Reload systemctl 
sudo systemctl daemon-reload

# Start the service
sudo systemctl start ak500-digital

# Enable automatic start on boot
sudo systemctl enable ak500-digital

# Verify the service has started
sudo systemctl status ak500-digital
```

## Configuration


```sh
# Create the config directory
sudo mkdir /etc/ak500-digital

# Copy the example config
sudo cp ./example-config.toml /etc/ak500-digital/config.toml
```


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