/// Implemented based of the sample library. See https://github.com/bluerobotics/BlueRobotics_MS5837_Library/
/// TODO Cleanup

use std::thread;
use std::time::Duration;
use anyhow::Context;

use rppal::i2c::I2c;
use tracing::{error, trace};

const MS5837_ADDR: u16 = 0x76;
const MS5837_RESET: u8 = 0x76;
const MS5837_ADC_READ: u8 = 0x76;
const MS5837_PROM_READ: u8 = 0x76;
const MS5837_CONVERT_D1_8192: u8 = 0x76;
const MS5837_CONVERT_D2_8192: u8 = 0x76;

const MS5837_02BA01: u16 = 0x00;
const MS5837_02BA21: u16 = 0x15;

#[derive(Debug)]
pub struct DepthSensor {
    i2c: I2c,
    fluid_density: f64,
    calibration: [u16; 8],
}

impl DepthSensor {
    /// Blocks until connected
    #[tracing::instrument]
    pub fn new(fluid_density: f64) -> anyhow::Result<Self> {
        trace!("DepthSensor::new()");

        let i2c = I2c::new().context("Create i2c")?;

        let mut sensor = Self {
            fluid_density,
            i2c,
            calibration: [0; 8],
        };

        sensor.connect().context("Connect")?;

        Ok(sensor)
    }

    #[tracing::instrument]
    fn connect(&mut self) -> anyhow::Result<()> {
        trace!("DepthSensor::connect()");

        loop {
            trace!("Attempting to connect to depth sensor");

            self.i2c.set_slave_address(MS5837_ADDR).context("Set address")?;

            // Reset the depth sensor
            self.i2c.write(&[MS5837_RESET]).context("Reset")?;

            // Wait for reset to complete
            thread::sleep(Duration::from_millis(10));

            // Read calibration data
            for (offset, data) in self.calibration.iter_mut().enumerate() {
                let buffer = &mut [0, 0];
                self.i2c.block_read(MS5837_PROM_READ + offset as u8 * 2, buffer).context("Read calibration")?;
                *data = (buffer[0] as u16) << 8 | buffer[1] as u16;
            }

            let crc_read = (self.calibration[0] >> 12) as u8;
            let crc_calculated = crc4(&mut self.calibration);

            if crc_read == crc_calculated {
                let version = self.calibration[0] >> 5 & 0x7F;
                match version {
                    MS5837_02BA01 | MS5837_02BA21 => {}
                    ver => unimplemented!("Version {} is not implemented", ver)
                }

                return Ok(());
            } else {
                error!("Got bad crc from depth sensor. Retrying!");
                thread::sleep(Duration::from_secs(5))
            }
        }
    }

    /// Takes a minimum of 40ms
    pub fn read_depth(&mut self) -> anyhow::Result<(f64, f64)> { // depth, temperature
        let buffer = &mut [0, 0, 0];
        // Read new data from the sensor
        // Request D1 conversion
        self.i2c.write(&[MS5837_CONVERT_D1_8192]).context("Request D1")?;
        thread::sleep(Duration::from_millis(20)); // Max conversion time

        // Read D1
        self.i2c.block_read(MS5837_ADC_READ, buffer).context("Read D1")?;
        let pressure_D1 = (buffer[0] as u32) << 16 | (buffer[1] as u32) << 8 | buffer[2] as u32;

        // Request D2 conversion
        self.i2c.write(&[MS5837_CONVERT_D2_8192]).context("Request D2")?;
        thread::sleep(Duration::from_millis(20)); // Max conversion time

        // Read D2
        self.i2c.block_read(MS5837_ADC_READ, buffer).context("Read D2")?;
        let temperature_D2 = (buffer[0] as u32) << 16 | (buffer[1] as u32) << 8 | buffer[2] as u32;

        // Given C1-C6 and D1, D2, calculated TEMP and P
        // Do conversion first and then second order temp compensation

        // TODO check that the inferred types of these match up
        // TODO rustify this code
        let mut dT = 0;
        let mut SENS = 0;
        let mut OFF = 0;
        let mut SENSi = 0;
        let mut OFFi = 0;
        let mut Ti = 0;
        let mut OFF2 = 0;
        let mut SENS2 = 0;
        let mut P = 0;
        let mut TEMP = 0;

        // Terms called
        dT = temperature_D2 - self.calibration[5] as u32 * 256;
        SENS = self.calibration[1] as i64 * 65536 + self.calibration[3] as i64 * dT as i64 / 128;
        OFF = self.calibration[2] as i64 * 131072 + self.calibration[4] as i64 * dT as i64 / 64;
        P = (pressure_D1 as i32 * SENS as i32 / 2097152 - OFF as i32) / 32768;

        // Temp conversion
        TEMP = 2000 + dT as i64 * self.calibration[6] as i64 / 8388608;

        // Second order compensation
        if TEMP / 100 < 20 { // Low temp
            Ti = 11 * dT as i64 * dT as i64 / 34359738368;
            OFFi = 31 * (TEMP - 2000) * (TEMP - 2000) / 8;
            SENSi = 63 * (TEMP - 2000) * (TEMP - 2000) / 32;
        }

        OFF2 = OFF - OFFi; // Calculate pressure and temp second order
        SENS2 = SENS - SENSi;

        TEMP = TEMP - Ti;

        P = (pressure_D1 as i32 * SENS2 as i32 / 2097152 - OFF2 as i32) / 32768;

        // Compute pressure into depth
        Ok(((P as f64 - 101300.0) / (self.fluid_density * 9.80665), TEMP as f64 / 100.0))
    }
}

// Might not be implemented correctly
// Should not need mut
fn crc4(n_prom: &mut [u16]) -> u8 {
    let mut n_rem = 0;

    n_prom[0] = n_prom[0] & 0x0FFF;
    n_prom[7] = 0;

    for i in 0..16 {
        if i % 2 == 1 {
            n_rem ^= n_prom[i >> 1] & 0x00FF;
        } else {
            n_rem ^= n_prom[i >> 1] >> 8;
        }

        for n_bit in (1..=8).rev() {
            if n_rem & 0x8000 != 0 {
                n_rem = n_rem << 1 ^ 0x3000;
            } else {
                n_rem = n_rem << 1;
            }
        }
    }

    (n_rem >> 12) as u8
}
