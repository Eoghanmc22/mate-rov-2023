use anyhow::Context;
use rppal::spi;
use rppal::spi::Spi;
use tracing::trace;
use common::types::{Degrees, Gauss, GForce, InertialFrame, MagFrame};

// TODO Verify correctness
// TODO Simplify impl
// TODO Extract constants
// TODO interact with chip registers more cleanly (bitflags?)
// TODO Extract spi stuff

const LSM6DSL_ADDRESS: u8          =  0x6A;

const LSM6DSL_WHO_AM_I: u8         =  0x0F;
const LSM6DSL_RAM_ACCESS: u8       =  0x01;
const LSM6DSL_CTRL1_XL: u8         =  0x10;
const LSM6DSL_CTRL8_XL: u8         =  0x17;
const LSM6DSL_CTRL2_G: u8          =  0x11;
const LSM6DSL_CTRL10_C: u8         =  0x19;
const LSM6DSL_TAP_CFG1: u8         =  0x58;
const LSM6DSL_INT1_CTR: u8         =  0x0D;
const LSM6DSL_CTRL3_C: u8          =  0x12;
const LSM6DSL_CTRL4_C: u8          =  0x13;

const LSM6DSL_STEP_COUNTER_L: u8   =  0x4B;
const LSM6DSL_STEP_COUNTER_H: u8   =  0x4C;

const LSM6DSL_OUT_L_TEMP: u8       =  0x20;
const LSM6DSL_OUT_H_TEMP: u8       =  0x21;

const LSM6DSL_OUT_BLOCK: u8         =  0x22;
const LSM6DSL_OUTX_L_G: u8         =  0x22;
const LSM6DSL_OUTX_H_G: u8         =  0x23;
const LSM6DSL_OUTY_L_G: u8         =  0x24;
const LSM6DSL_OUTY_H_G: u8         =  0x25;
const LSM6DSL_OUTZ_L_G: u8         =  0x26;
const LSM6DSL_OUTZ_H_G: u8         =  0x27;
const LSM6DSL_OUTX_L_XL: u8        =  0x28;
const LSM6DSL_OUTX_H_XL: u8        =  0x29;
const LSM6DSL_OUTY_L_XL: u8        =  0x2A;
const LSM6DSL_OUTY_H_XL: u8        =  0x2B;
const LSM6DSL_OUTZ_L_XL: u8        =  0x2C;
const LSM6DSL_OUTZ_H_XL: u8        =  0x2D;

const LSM6DSL_TAP_CFG: u8          =  0x58;
const LSM6DSL_WAKE_UP_SRC: u8      =  0x1B;
const LSM6DSL_WAKE_UP_DUR: u8      =  0x5C;
const LSM6DSL_FREE_FALL: u8        =  0x5D;
const LSM6DSL_MD1_CFG: u8          =  0x5E;
const LSM6DSL_MD2_CFG: u8          =  0x5F;
const LSM6DSL_TAP_THS_6D: u8       =  0x59;
const LSM6DSL_INT_DUR2: u8         =  0x5A;
const LSM6DSL_WAKE_UP_THS: u8      =  0x5B;
const LSM6DSL_FUNC_SRC1: u8        =  0x53;


const LIS3MDL_ADDRESS: u8     = 0x1C;

const LIS3MDL_WHO_AM_I: u8    = 0x0F;

const LIS3MDL_CTRL_REG1: u8   = 0x20;

const LIS3MDL_CTRL_REG2: u8   = 0x21;
const LIS3MDL_CTRL_REG3: u8   = 0x22;
const LIS3MDL_CTRL_REG4: u8   = 0x23;
const LIS3MDL_CTRL_REG5: u8   = 0x24;

const LIS3MDL_STATUS_REG: u8  = 0x27;

const LIS3MDL_OUT_BLOCK: u8     = 0x28;
const LIS3MDL_OUT_X_L: u8     = 0x28;
const LIS3MDL_OUT_X_H: u8     = 0x29;
const LIS3MDL_OUT_Y_L: u8     = 0x2A;
const LIS3MDL_OUT_Y_H: u8     = 0x2B;
const LIS3MDL_OUT_Z_L: u8     = 0x2C;
const LIS3MDL_OUT_Z_H: u8     = 0x2D;

const LIS3MDL_TEMP_OUT_L: u8  = 0x2E;
const LIS3MDL_TEMP_OUT_H: u8  = 0x2F;

const LIS3MDL_INT_CFG: u8     = 0x30;
const LIS3MDL_INT_SRC: u8     = 0x31;
const LIS3MDL_INT_THS_L: u8   = 0x32;
const LIS3MDL_INT_THS_H: u8   = 0x33;

pub struct ImuSensor {
    lsm6dsl: Inertial, // Gyro and accelerometer
    lis3mdl: Magnetometer, // Magnetometer TODO
}

impl ImuSensor {
    /// Blocks until connected
    #[tracing::instrument]
    pub fn new() -> anyhow::Result<Self> {
        trace!("ImuSensor::new()");

        let lsm6dsl = Spi::new(spi::Bus::Spi0, spi::SlaveSelect::Ss0, 10_000_000, spi::Mode::Mode2).context("Create spi for lsm6dsl")?;
        let lis3mdl = Spi::new(spi::Bus::Spi0, spi::SlaveSelect::Ss1, 10_000_000, spi::Mode::Mode2).context("Create spi for lis3mdl")?;

        Ok(Self {
            lsm6dsl: Inertial::new(lsm6dsl)?,
            lis3mdl: Magnetometer::new(lis3mdl)?,
        })
    }
}

struct Magnetometer(Spi);
struct Inertial(Spi);

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

trait SpiDevice {
    fn read_byte(&mut self, address: u8) -> anyhow::Result<u8>;
    fn read(&mut self, address: u8, buffer: &mut [u8]) -> anyhow::Result<()>;
    fn write_byte(&mut self, address: u8, byte: u8) -> anyhow::Result<()>;
    fn write(&mut self, address: u8, buffer: &[u8]) -> anyhow::Result<()>;
}

impl SpiDevice for Magnetometer {
    fn read_byte(&mut self, address: u8) -> anyhow::Result<u8> {
        let address = address & 0b00111111 | 0b10000000; // Read
        let buffer = &mut [0];
        self.0.write(&[address])?;
        self.0.read(buffer)?;

        Ok(buffer[0])
    }

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> anyhow::Result<()> {
        let address = address & 0b00111111 | 0b11000000; // Read with address auto increment
        self.0.write(&[address])?;
        self.0.read(buffer)?;

        Ok(())
    }

    fn write_byte(&mut self, address: u8, byte: u8) -> anyhow::Result<()> {
        let address = address & 0b00111111 | 0b00000000; // Write
        self.0.write(&[address, byte])?;

        Ok(())
    }

    fn write(&mut self, address: u8, buffer: &[u8]) -> anyhow::Result<()> {
        let address = address & 0b00111111 | 0b01000000; // Write with address auto increment
        self.0.write(&[address])?;
        self.0.write(buffer)?;

        Ok(())
    }
}

impl SpiDevice for Inertial {
    fn read_byte(&mut self, address: u8) -> anyhow::Result<u8> {
        let address = address & 0b01111111 | 0b10000000; // Read
        let buffer = &mut [0];
        self.0.write(&[address])?;
        self.0.read(buffer)?;

        Ok(buffer[0])
    }

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> anyhow::Result<()> {
        let address = address & 0b01111111 | 0b10000000; // Read with address auto increment
        self.0.write(&[address])?;
        self.0.read(buffer)?;

        Ok(())
    }

    fn write_byte(&mut self, address: u8, byte: u8) -> anyhow::Result<()> {
        let address = address & 0b01111111 | 0b00000000; // Write
        self.0.write(&[address, byte])?;

        Ok(())
    }

    fn write(&mut self, address: u8, buffer: &[u8]) -> anyhow::Result<()> {
        let address = address & 0b01111111 | 0b00000000; // Write with address auto increment
        self.0.write(&[address])?;
        self.0.write(buffer)?;

        Ok(())
    }
}
