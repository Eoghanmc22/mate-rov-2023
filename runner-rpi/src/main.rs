use std::{
    env,
    process::{Command, Stdio},
};

use anyhow::{bail, Context};

pub fn main() -> anyhow::Result<()> {
    let Some(bin) = env::args().nth(1) else {
        bail!("No binary provided");
    };

    println!("Finding ip");
    let output = Command::new("doas")
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .arg("arp-scan")
        .arg("--plain")
        .arg("--quiet")
        .arg("-B")
        .arg("80M")
        .arg("-I")
        .arg(include_str!("../../interface").trim())
        .arg("169.254.0.0/16")
        .output()
        .context("Execute arp-scan")?;

    if !output.status.success() {
        bail!("arp-scan was not successful");
    }

    let output = core::str::from_utf8(&output.stdout).context("Convert output to utf8")?;

    let Some(ip) = output.split(char::is_whitespace).next() else {
        bail!("No ip was found");
    };

    if ip.is_empty() {
        bail!("Ip was blank");
    }

    println!("Using ip {ip}");

    Command::new("sshpass")
        .arg("-p")
        .arg("raspberry")
        .arg("scp")
        .arg(bin)
        .arg(format!("pi@{ip}:~/mate/exec"))
        .spawn()
        .context("Execute scp")?;

    println!("Success!");

    Ok(())
}
