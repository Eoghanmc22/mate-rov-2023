use std::{thread, time::Duration};

use robot::peripheral::pca9685::Pca9685;

fn main() -> anyhow::Result<()> {
    let mut pca = Pca9685::new(
        Pca9685::I2C_BUS,
        Pca9685::I2C_ADDRESS,
        Duration::from_secs_f64(1.0 / 100.0),
    )?;

    pca.output_enable();

    // pca.set_pwm(15, Duration::from_micros(1900))?;
    let mut pwms = [Duration::from_micros(1500); 16];
    pwms[15] = Duration::from_micros(1100);
    pca.set_pwms(pwms)?;

    loop {
        thread::sleep(Duration::MAX);
    }
}
