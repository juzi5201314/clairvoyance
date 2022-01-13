use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::time::Duration;

use bincode::{Decode, Encode};
use byte_unit::Byte;
#[cfg(target_os = "linux")]
use heim::process::os::linux::MemoryExt;

#[derive(Debug, Clone, Encode, Decode)]
pub struct Data {
    pub memory: Memory,
    pub cpu_time: CpuTime,
    pub cpu_usage: CpuUsage,
    pub io: Io,

    // linux only
    pub net_io: HashMap<String, NetIo>,
}

#[derive(Clone, Encode, Decode)]
pub struct CpuUsage(pub f32);

#[derive(Clone, Encode, Decode)]
pub struct Memory {
    pub rss: u64,
    pub vms: u64,

    // linux only
    pub shared: Option<u64>,
    pub text: Option<u64>,
    pub data: Option<u64>,
}

#[derive(Clone, Encode, Decode)]
pub struct CpuTime {
    pub user: f64,
    pub system: f64,
}

#[derive(Clone, Encode, Decode)]
pub struct Io {
    pub bytes_written: u64,
    pub bytes_read: u64,

    // linux only
    pub disk_written: Option<u64>,
    pub disk_read: Option<u64>,
    pub syscall_written: Option<u64>,
    pub syscall_read: Option<u64>,
}

#[derive(Clone, Encode, Decode)]
pub struct NetIo {
    bytes_sent: u64,
    bytes_recv: u64,
    packets_sent: u64,
    packets_recv: u64,
    errors_sent: u64,
    errors_recv: u64,
    drop_recv: u64,
    drop_sent: u64,
}

#[cfg(target_os = "linux")]
impl From<heim::process::IoCounters> for NetIo {
    fn from(io: heim::process::IoCounters) -> Self {
        use heim::net::os::linux::IoCountersExt;
        NetIo {
            bytes_sent: io.bytes_sent().get::<heim::units::information::byte>(),
            bytes_recv: io.bytes_recv().get::<heim::units::information::byte>(),
            packets_sent: io.packets_sent(),
            packets_recv: io.packets_recv(),
            errors_sent: io.errors_sent(),
            errors_recv: io.errors_recv(),
            drop_recv: io.drop_recv(),
            drop_sent: io.drop_sent(),
        }
    }
}

impl Debug for NetIo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let to_string = |x: Byte| x.get_appropriate_unit(false).to_string();
        f.debug_struct("NetIo")
            .field("bytes_sent", &to_string(Byte::from(self.bytes_sent)))
            .field("bytes_recv", &to_string(Byte::from(self.bytes_recv)))
            .field("packets_sent", &self.packets_sent)
            .field("packets_recv", &self.packets_recv)
            .field("errors_sent", &self.errors_sent)
            .field("errors_recv", &self.errors_recv)
            .field("drop_recv", &self.drop_recv)
            .field("drop_sent", &self.drop_sent)
            .finish()
    }
}

#[cfg(not(target_os = "linux"))]
impl From<heim::process::IoCounters> for Io {
    fn from(io: heim::process::IoCounters) -> Self {
        Io {
            bytes_written: io.bytes_written().get::<heim::units::information::byte>(),
            bytes_read: io.bytes_read().get::<heim::units::information::byte>(),

            disk_written: None,
            disk_read: None,
            syscall_written: None,
            syscall_read: None,
        }
    }
}

impl Debug for Io {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let to_string = |x: Byte| x.get_appropriate_unit(false).to_string();
        f.debug_struct("I/O")
            .field("bytes_written", &to_string(Byte::from(self.bytes_written)))
            .field("bytes_read", &to_string(Byte::from(self.bytes_read)))
            .field(
                "disk_written",
                &self.disk_written.map(Byte::from).map(to_string),
            )
            .field("disk_read", &self.disk_read.map(Byte::from).map(to_string))
            .field("syscall_written", &self.syscall_written)
            .field("syscall_read", &self.syscall_read)
            .finish()
    }
}

impl From<heim::process::CpuTime> for CpuTime {
    fn from(cpu_time: heim::process::CpuTime) -> Self {
        CpuTime {
            user: cpu_time.user().get::<heim::units::time::microsecond>(),
            system: cpu_time.system().get::<heim::units::time::microsecond>(),
        }
    }
}

impl Debug for CpuTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpuTime")
            .field("user", &Duration::from_micros(self.user as u64))
            .field("system", &Duration::from_micros(self.system as u64))
            .finish()
    }
}

impl Debug for CpuUsage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.0.round() / num_cpus::get() as f32)
    }
}

impl From<heim::process::Memory> for Memory {
    fn from(mem: heim::process::Memory) -> Self {
        Memory {
            rss: mem.rss().get::<heim::units::information::byte>(),
            vms: mem.vms().get::<heim::units::information::byte>(),

            shared: {
                cfg_if::cfg_if! {
                    if #[cfg(target_os = "linux")] {
                        Some(mem.shared().get::<heim::units::information::byte>())
                    } else {
                        None
                    }
                }
            },
            text: {
                cfg_if::cfg_if! {
                    if #[cfg(target_os = "linux")] {
                        Some(mem.text().get::<heim::units::information::byte>())
                    } else {
                        None
                    }
                }
            },
            data: {
                cfg_if::cfg_if! {
                    if #[cfg(target_os = "linux")] {
                        Some(mem.data().get::<heim::units::information::byte>())
                    } else {
                        None
                    }
                }
            },
        }
    }
}

impl Debug for Memory {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let to_string = |x: Byte| x.get_appropriate_unit(false).to_string();
        f.debug_struct("Memory")
            .field("vms", &to_string(Byte::from(self.vms)))
            .field("rss", &to_string(Byte::from(self.rss)))
            .field("shared", &self.shared.map(Byte::from).map(to_string))
            .field("text", &self.text.map(Byte::from).map(to_string))
            .field("data", &self.data.map(Byte::from).map(to_string))
            .finish()
    }
}
