use std::{env, process::Command};

use anyhow::{bail, Context};

pub fn main() -> anyhow::Result<()> {
    let Some(bin) = env::args().nth(1) else {
        bail!("No binary provided");
    };

    eprintln!("Killing process");

    let status = Command::new("ssh")
        .arg("pi@mate.local")
        .arg("sudo pkill mate-exec || exit 0")
        .spawn()
        .context("Spawn ssh")?
        .wait()
        .context("Wait on ssh")?;

    if status.success() {
        eprintln!("Killed");
    } else {
        bail!("Could not kill process");
    }
    eprintln!();

    eprintln!("Uploading");

    let rst = Command::new("scp")
        .arg("./detect_cameras.sh")
        .arg("pi@mate.local:~/mate/detect_cameras.sh")
        .spawn()
        .context("Spawn scp")?
        .wait();

    let status = Command::new("scp")
        .arg(bin)
        .arg("pi@mate.local:~/mate/mate-exec")
        .spawn()
        .context("Spawn scp")?
        .wait()
        .and(rst)
        .context("Wait on scp")?;

    if status.success() {
        eprintln!("Upload success!");
    } else {
        bail!("Upload failed: {status}")
    }
    eprintln!();

    eprintln!("Running binary");

    let status = Command::new("ssh")
        .arg("pi@mate.local")
        .arg("sudo ~/mate/mate-exec")
        .spawn()
        .context("Spawn ssh")?
        .wait()
        .context("Wait on ssh")?;

    if status.success() {
        eprintln!("Remote run success!");
    } else {
        bail!("Remote run failed: {status}")
    }

    Ok(())
}
