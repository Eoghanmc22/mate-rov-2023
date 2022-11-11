use std::{
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use common::{
    kvdata::Value,
    protocol::Packet,
    state::RobotState,
    types::{Celsius, Component, Cpu, Disk, Memory, Network, Process, SystemInfo},
};
use sysinfo::{
    ComponentExt, CpuExt, DiskExt, NetworkExt, NetworksExt, PidExt, ProcessExt, System, SystemExt,
    UserExt,
};
use tracing::error;

use crate::{event::Event, events::EventHandle};

use super::System as RobotSystem;

pub struct HwStatSystem;

impl RobotSystem for HwStatSystem {
    fn start(_robot: Arc<RwLock<RobotState>>, mut events: EventHandle) -> anyhow::Result<()>
    where
        Self: Sized,
    {
        thread::spawn(move || {
            let mut system = System::new();
            loop {
                system.refresh_all();
                match collect_system_state(&system) {
                    Ok(hw_state) => {
                        let packet = Packet::KVUpdate(Value::SystemInfo(hw_state));
                        events.send(Event::PacketSend(packet));
                    }
                    Err(err) => error!("Could not collect system state: {err:?}"),
                }
                thread::sleep(Duration::from_secs(1));
            }
        });

        Ok(())
    }
}

fn collect_system_state(system: &System) -> anyhow::Result<SystemInfo> {
    // TODO sorting?
    let hw_state = SystemInfo {
        processes: system
            .processes()
            .values()
            .map(|process| Process {
                name: process.name().to_owned(),
                pid: process.pid().as_u32(),
                memory: process.memory(),
                cpu_usage: process.cpu_usage(),
                user: process
                    .user_id()
                    .and_then(|user| system.get_user_by_id(user))
                    .map(|user| user.name().to_owned()),
            })
            .collect(),
        load_average: (
            system.load_average().one,
            system.load_average().five,
            system.load_average().fifteen,
        ),
        networks: system
            .networks()
            .iter()
            .map(|(name, data)| Network {
                name: name.to_owned(),
                rx_bytes: data.total_received(),
                tx_bytes: data.total_transmitted(),
                rx_packets: data.total_packets_received(),
                tx_packets: data.total_packets_transmitted(),
                rx_errors: data.total_errors_on_received(),
                tx_errors: data.total_errors_on_transmitted(),
            })
            .collect(),
        cpu_total: Cpu {
            frequency: system.global_cpu_info().frequency(),
            usage: system.global_cpu_info().cpu_usage(),
            name: system.global_cpu_info().name().to_owned(),
        },
        cpus: system
            .cpus()
            .iter()
            .map(|cpu| Cpu {
                frequency: cpu.frequency(),
                usage: cpu.cpu_usage(),
                name: cpu.name().to_owned(),
            })
            .collect(),
        core_count: system.physical_core_count(),
        memory: Memory {
            total_mem: system.total_memory(),
            used_mem: system.used_memory(),
            free_mem: system.free_memory(),
            total_swap: system.total_swap(),
            used_swap: system.used_swap(),
            free_swap: system.free_swap(),
        },
        components: system
            .components()
            .iter()
            .map(|component| Component {
                tempature: Celsius(component.temperature() as f64),
                tempature_max: Celsius(component.max() as f64),
                tempature_critical: component.critical().map(|it| Celsius(it as f64)),
                name: component.label().to_owned(),
            })
            .collect(),
        disks: system
            .disks()
            .iter()
            .map(|disk| Disk {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total_space: disk.total_space(),
                available_space: disk.available_space(),
                removable: disk.is_removable(),
            })
            .collect(),
        uptime: Duration::from_secs(system.uptime()),
        name: system.name(),
        kernel_version: system.kernel_version(),
        os_version: system.long_os_version(),
        distro: system.distribution_id(),
        host_name: system.host_name(),
    };

    Ok(hw_state)
}
