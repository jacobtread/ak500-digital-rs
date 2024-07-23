use anyhow::Context;
use hidapi::{HidApi, HidDevice};
use std::thread;
use std::time::Duration;
use systemstat::{Platform, System};

const VENDOR_ID: u16 = 13875;
const PRODUCT_ID: u16 = 3;
const REPORT_ID: u8 = 16;

fn main() -> anyhow::Result<()> {
    println!("Hello, world!");

    let api = HidApi::new().context("failed to create hid api")?;
    let mut device = api
        .open(VENDOR_ID, PRODUCT_ID)
        .context("failed to open device")?;

    let sys = System::new();

    loop {
        let mut load = 0.;
        match sys.cpu_load_aggregate() {
            Ok(cpu) => {
                println!("\nMeasuring CPU load...");
                thread::sleep(Duration::from_secs(1));
                let cpu = cpu.done().unwrap();

                load = (cpu.user + cpu.nice + cpu.system + cpu.interrupt) * 100.0;

                println!(
                    "CPU load: {}% user, {}% nice, {}% system, {}% intr, {}% idle ",
                    cpu.user * 100.0,
                    cpu.nice * 100.0,
                    cpu.system * 100.0,
                    cpu.interrupt * 100.0,
                    cpu.idle * 100.0
                );
            }
            Err(x) => println!("\nCPU load: error: {}", x),
        }

        match sys.cpu_temp() {
            Ok(cpu_temp) => println!("\nCPU temp: {}", cpu_temp),
            Err(x) => println!("\nCPU temp: {}", x),
        }

        // Warning condition = C >= 90 F >= 194

        write_device_state(
            &mut device,
            ControlUnit::Centigrade,
            ((load / 100.) * 10.0).clamp(1., 10.) as u8,
            70,
            false,
        )?;

        std::thread::sleep(Duration::from_secs(5));
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum ControlUnit {
    // C
    Centigrade = 19,
    // F
    Fahrenheit = 35,
    // %
    Percentage = 76,
    // Plays loading animation
    Loading = 170,
}

fn write_device_state(
    device: &mut HidDevice,
    control_unit: ControlUnit,
    progress: u8,
    value: u16,
    warning: bool,
) -> anyhow::Result<()> {
    let mut data: Vec<u8> = Vec::new();
    data.push(REPORT_ID);
    data.push(control_unit as u8 /* Centigrade */);
    data.push(progress);
    data.extend_from_slice(&num_to_arr(value));
    data.push(warning as u8);

    // Write temperature
    device.write(data.as_slice()).context("write hid")?;

    Ok(())
}

fn num_to_arr(e: u16) -> [u8; 3] {
    let hundreds = (e / 100);
    let tens = (e % 100) / 10;
    let units = e % 10;

    [hundreds as u8, tens as u8, units as u8]
}
