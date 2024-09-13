use anyhow::Context;
use hidapi::{HidApi, HidDevice};
use serde::Deserialize;
use std::fs::read_to_string;
use std::path::Path;
use std::time::Duration;
use sysinfo::{Components, CpuRefreshKind, RefreshKind, System};

/// Path to the configuration file
const CONFIGURATION_PATH: &str = "/etc/ak500-digital/config.toml";

// DeepCool AK500-DIGITAL
const VENDOR_ID: u16 = 13875;
const PRODUCT_ID: u16 = 3;
const REPORT_ID: u8 = 16;

/// Configuration options
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct Configuration {
    /// Unit of to use for showing the temperature
    temperature_unit: TemperatureUnit,

    /// Whether to show a high temperature warning
    #[serde(default = "default_show_warning")]
    show_warning: bool,

    /// Temperature to show warnings at
    #[serde(default = "default_warning_temperature")]
    warning_temperature: f32,

    /// Display mode
    display_mode: DisplayMode,
}

// By default warn when temperature reaches 90° celsius
fn default_warning_temperature() -> f32 {
    90.
}

// By default should show warnings
fn default_show_warning() -> bool {
    true
}

// Loads the configuration file
fn load_configuration() -> anyhow::Result<Configuration> {
    let path = Path::new(CONFIGURATION_PATH);
    if !path.exists() {
        return Ok(Configuration::default());
    }

    let contents = read_to_string(path).context("failed reading configuration file")?;

    toml::from_str(&contents).context("failed to parse configuration file")
}

fn main() -> anyhow::Result<()> {
    // Connect to HID device
    let api = HidApi::new().context("failed to create hid api")?;
    let mut device = api
        .open(VENDOR_ID, PRODUCT_ID)
        .context("failed to open device")?;

    // Write initial loading state
    write_device_state(&mut device, ControlUnit::Loading, 0, 0, false)?;

    let configuration: Configuration = match load_configuration() {
        Ok(value) => value,
        Err(err) => {
            eprintln!(
                "failed to load configuration, falling back to defaults: {}",
                err
            );

            Default::default()
        }
    };

    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_cpu(CpuRefreshKind::new().with_cpu_usage()),
    );

    let mut components = Components::new();

    let report_unit = configuration.temperature_unit;
    let warning_temperature: Temperature =
        Temperature(report_unit, configuration.warning_temperature);

    let is_automatic = matches!(configuration.display_mode, DisplayMode::Automatic);

    let mut frame_count = 0;

    loop {
        // Get load and temperature
        let load = get_cpu_load(&mut sys)?;
        let cpu_temp = get_cpu_temp(&mut components)?;

        // Determine if warning should be shown
        let warning = configuration.show_warning && cpu_temp >= warning_temperature;

        // Convert the load percent to 1-10 for the square usage indicator
        let load_progress = ((load / 100.0) * 10.0).clamp(1.0, 10.0) as u8;

        // Convert to chosen unit type
        let cpu_temp_local = cpu_temp.convert(report_unit);
        let cpu_temp_value = Into::<u32>::into(cpu_temp_local) as u16;

        // Clamp load value for display
        let load_value = load.clamp(0., 999.) as u16;

        // Determine control unit for the temperature
        let control_unit = ControlUnit::from(report_unit);

        // Check if we are displaying temperature
        let is_temperature = (is_automatic && frame_count < 5)
            || matches!(configuration.display_mode, DisplayMode::Temperature);

        if is_temperature {
            // Write the temperature state to the device
            write_device_state(
                &mut device,
                control_unit,
                load_progress,
                cpu_temp_value,
                warning,
            )?;
        } else {
            // Write the utilization state to the device
            write_device_state(
                &mut device,
                ControlUnit::Percentage,
                load_progress,
                load_value,
                warning,
            )?;
        }

        // Wait
        std::thread::sleep(Duration::from_secs(1));

        frame_count += 1;

        // Reset on 11th frame
        if frame_count == 10 {
            frame_count = 0;
        }
    }
}

/// Obtains the CPU temperature
fn get_cpu_temp(components: &mut Components) -> anyhow::Result<Temperature> {
    components.refresh_list();

    // Take average of all available packages
    let mut total_temps = 0;
    let mut total_temp = 0.0;

    for component in components {
        let label = component.label();
        let temp = component.temperature();

        // Intel CPU package
        if label.starts_with("coretemp Package") {
            total_temp += temp;
            total_temps += 1;
        }
    }

    let avg = total_temp / (total_temps as f32);

    Ok(Temperature(TemperatureUnit::Celsius, avg))
}

/// Obtains the CPU load, sleeps for 1s to allow time to aggregate the load
/// information, this is required.
fn get_cpu_load(sys: &mut System) -> anyhow::Result<f32> {
    sys.refresh_cpu_usage(); // Refreshing CPU information.

    let mut total_cpus = 0;
    let mut total_usage = 0.0;

    for cpu in sys.cpus() {
        let usage = cpu.cpu_usage();

        total_usage += usage;
        total_cpus += 1;
    }

    let avg = total_usage / (total_cpus as f32);

    Ok(avg)
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
enum DisplayMode {
    // Show temperature as the main focus
    #[default]
    Temperature,
    // Show CPU utilization as the main focus
    Utilization,
    // Switch between temps and utilization
    Automatic,
}

/// Unit of temperature
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
enum TemperatureUnit {
    /// Temperature in degrees celsius
    #[default]
    Celsius,
    /// Temperature in degrees fahrenheit
    Fahrenheit,
}

/// Represents a temperature in a specific unit
#[derive(Debug, Clone, Copy)]
struct Temperature(TemperatureUnit, f32);

impl Temperature {
    /// Converts the temperature to the provided unit
    pub fn convert(self, unit: TemperatureUnit) -> Self {
        match (self.0, unit) {
            (TemperatureUnit::Celsius, TemperatureUnit::Fahrenheit) => {
                Self(unit, self.1 * 9.0 / 5.0 + 32.0)
            }
            (TemperatureUnit::Fahrenheit, TemperatureUnit::Celsius) => {
                Self(unit, (self.1 - 32.0) * 5.0 / 9.0)
            }

            // No conversion needed
            (TemperatureUnit::Celsius, TemperatureUnit::Celsius)
            | (TemperatureUnit::Fahrenheit, TemperatureUnit::Fahrenheit) => self,
        }
    }
}

impl From<Temperature> for u32 {
    fn from(value: Temperature) -> Self {
        value.1.round() as u32
    }
}

impl From<TemperatureUnit> for ControlUnit {
    fn from(value: TemperatureUnit) -> Self {
        match value {
            TemperatureUnit::Celsius => ControlUnit::Celsius,
            TemperatureUnit::Fahrenheit => ControlUnit::Fahrenheit,
        }
    }
}

impl PartialEq for Temperature {
    fn eq(&self, other: &Self) -> bool {
        // Convert to same units
        let converted = self.convert(other.0);

        // Compare underling unit
        converted.1.eq(&other.1)
    }
}

impl PartialOrd for Temperature {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Convert to same units
        let converted = self.convert(other.0);

        // Compare underling unit
        converted.1.partial_cmp(&other.1)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum ControlUnit {
    // °C
    Celsius = 19,
    // °F
    Fahrenheit = 35,
    // %
    Percentage = 76,
    // Plays loading animation
    Loading = 170,
}

/// Writes the state to the display
fn write_device_state(
    device: &mut HidDevice,
    control_unit: ControlUnit,
    progress: u8,
    value: u16,
    warning: bool,
) -> anyhow::Result<()> {
    let control_unit_byte = control_unit as u8;
    let warning_byte = warning as u8;

    let [hundreds, tens, units] = convert_to_digits(value);

    let message = [
        REPORT_ID,
        control_unit_byte,
        progress,
        hundreds,
        tens,
        units,
        warning_byte,
    ];

    // Write device state
    device.write(&message).context("write hid")?;

    Ok(())
}

/// Converts the provided number 0-999 into the 3 digits that
/// can be shown on the digital display
fn convert_to_digits(value: u16) -> [u8; 3] {
    // Display can only show numbers up to 999
    let value = value.clamp(0, 999);

    let hundreds = value / 100;
    let tens = (value % 100) / 10;
    let units = value % 10;

    [hundreds as u8, tens as u8, units as u8]
}
