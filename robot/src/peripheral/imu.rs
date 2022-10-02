use rppal::spi::Spi;
use tracing::trace;
use common::types::{Degrees, Gauss, GForce, InertialFrame, MagFrame};
use crate::peripheral::lis3mdl::{LIS3MDL_CTRL_REG1, LIS3MDL_CTRL_REG2, LIS3MDL_CTRL_REG3, LIS3MDL_OUT_BLOCK, LIS3MDL_WHO_AM_I};
use crate::peripheral::lsm6dsl::{LSM6DSL_CTRL1_XL, LSM6DSL_CTRL2_G, LSM6DSL_CTRL3_C, LSM6DSL_CTRL8_XL, LSM6DSL_OUT_BLOCK, LSM6DSL_WHO_AM_I};
use crate::peripheral::spi::Device;

// TODO Verify correctness
// TODO Simplify impl
// TODO Extract constants
// TODO interact with chip registers more cleanly (bitflags?)
// TODO Extract spi stuff
// TODO Make api better lol

pub struct Magnetometer(Spi);
pub struct Inertial(Spi);

impl Magnetometer {
    const MAGNETIC_SENSITIVITY: f64 = 1.0/3421.0;

    #[tracing::instrument]
    pub fn new(spi: Spi) -> anyhow::Result<Self> {
        trace!("Magnetometer::new()");

        let mut magnetometer = Magnetometer(spi);
        let who_am_i = magnetometer.read_byte(LIS3MDL_WHO_AM_I)?;
        assert_eq!(who_am_i, 0x3D);

        // Initialise the accelerometer
        // TODO adjust
        magnetometer.write_byte(LIS3MDL_CTRL_REG1, 0b11011100)?;           // Temp sensor enabled, High performance, ODR 80 Hz, FAST ODR disabled and Self test disabled.
        magnetometer.write_byte(LIS3MDL_CTRL_REG2, 0b00100000)?;           // +/- 8 gauss
        magnetometer.write_byte(LIS3MDL_CTRL_REG3, 0b00000000)?;           // Continuous-conversion mode

        Ok(magnetometer)
    }

    pub fn read_sensor(&mut self) -> anyhow::Result<MagFrame> {
        let mut buffer = [0; 6];
        self.read(LIS3MDL_OUT_BLOCK, &mut buffer)?;

        Ok(MagFrame {
            // TODO check units
            mag_x: Gauss(((buffer[1] as i16) << 8 | buffer[0] as i16) as f64 / Self::MAGNETIC_SENSITIVITY),
            mag_y: Gauss(((buffer[3] as i16) << 8 | buffer[2] as i16) as f64 / Self::MAGNETIC_SENSITIVITY),
            mag_z: Gauss(((buffer[5] as i16) << 8 | buffer[4] as i16) as f64 / Self::MAGNETIC_SENSITIVITY),
        })
    }
}

impl Inertial {
    const LINEAR_SENSITIVITY: f64 = 0.244;
    const ANGULAR_SENSITIVITY: f64 = 70.0;

    #[tracing::instrument]
    pub fn new(spi: Spi) -> anyhow::Result<Self> {
        trace!("Inertial::new()");

        let mut inertial = Inertial(spi);
        let who_am_i = inertial.read_byte(LSM6DSL_WHO_AM_I)?;
        assert_eq!(who_am_i, 0x6A);

        // Initialise the accelerometer
        // TODO adjust
        inertial.write_byte(LSM6DSL_CTRL1_XL, 0b10011111)?;           // ODR 3.33 kHz, +/- 8g , BW = 400hz
        inertial.write_byte(LSM6DSL_CTRL8_XL, 0b11001000)?;           // Low pass filter enabled, BW9, composite filter
        inertial.write_byte(LSM6DSL_CTRL3_C, 0b01000100)?;            // Enable Block Data update, increment during multi byte read

        // Initialise the gyroscope
        inertial.write_byte(LSM6DSL_CTRL2_G, 0b10011100)?;            // ODR 3.3 kHz, 2000 dps

        Ok(inertial)
    }

    pub fn read_sensor(&mut self) -> anyhow::Result<InertialFrame> {
        let mut buffer = [0; 12];
        self.read(LSM6DSL_OUT_BLOCK, &mut buffer)?;

        Ok(InertialFrame {
            // todo check units
            gyro_x: Degrees(((buffer[1] as i16) << 8 | buffer[0] as i16) as f64 / Self::ANGULAR_SENSITIVITY),
            gyro_y: Degrees(((buffer[3] as i16) << 8 | buffer[2] as i16) as f64 / Self::ANGULAR_SENSITIVITY),
            gyro_z: Degrees(((buffer[5] as i16) << 8 | buffer[4] as i16) as f64 / Self::ANGULAR_SENSITIVITY),

            accel_x: GForce(((buffer[7] as i16) << 8 | buffer[6] as i16) as f64 / Self::LINEAR_SENSITIVITY),
            accel_y: GForce(((buffer[9] as i16) << 8 | buffer[8] as i16) as f64 / Self::LINEAR_SENSITIVITY),
            accel_z: GForce(((buffer[11] as i16) << 8 | buffer[10] as i16) as f64 / Self::LINEAR_SENSITIVITY),
        })
    }
}

impl Device for Magnetometer {
    fn read(&mut self, address: u8, buffer: &mut [u8]) -> anyhow::Result<()> {
        let address = address & 0b00111111 | 0b11000000; // Read with address auto increment
        self.0.write(&[address])?;
        self.0.read(buffer)?;

        Ok(())
    }

    fn write(&mut self, address: u8, buffer: &[u8]) -> anyhow::Result<()> {
        let address = address & 0b00111111 | 0b01000000; // Write with address auto increment
        self.0.write(&[address])?;
        self.0.write(buffer)?;

        Ok(())
    }
}

impl Device for Inertial {
    fn read(&mut self, address: u8, buffer: &mut [u8]) -> anyhow::Result<()> {
        let address = address & 0b01111111 | 0b10000000; // Read with address auto increment
        self.0.write(&[address])?;
        self.0.read(buffer)?;

        Ok(())
    }

    fn write(&mut self, address: u8, buffer: &[u8]) -> anyhow::Result<()> {
        let address = address & 0b01111111 | 0b00000000; // Write with address auto increment
        self.0.write(&[address])?;
        self.0.write(buffer)?;

        Ok(())
    }
}
