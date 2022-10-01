use crate::peripheral::imu::{Inertial, Magnetometer};

pub trait Device {
    fn read_byte(&mut self, address: u8) -> anyhow::Result<u8> {
        let bytes = &mut [0];
        self.read(address, bytes)?;
        Ok(bytes[0])
    }
    fn write_byte(&mut self, address: u8, byte: u8) -> anyhow::Result<()> {
        self.write(address, &[byte])
    }

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> anyhow::Result<()>;
    fn write(&mut self, address: u8, buffer: &[u8]) -> anyhow::Result<()>;
}
