use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use futures::stream::StreamExt;
use heim::process::{CpuUsage as HeimCpuUsage, Pid, Process};

use crate::data::{CpuTime, CpuUsage, Data, Io, Memory};
use crate::shutdown_notify::ShutdownGuard;
use crate::store::StoreStream;

pub struct Monitor {
    process: Process,
    last_cpu_usage: Option<HeimCpuUsage>,
    store_stream: StoreStream,
    _shutdown_guard: ShutdownGuard,
}

impl Monitor {
    pub async fn new<P>(
        process: Process,
        out_dir: P,
        _shutdown_guard: ShutdownGuard,
    ) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let store_stream = StoreStream::create(out_dir.as_ref().join(format!(
            "{}-{}.clairvoyance",
            process.pid(),
            chrono::Local::now().format("%F_%H-%M-%S")
        )))
        .await?;

        Ok(Monitor {
            process,
            last_cpu_usage: None,
            store_stream,
            _shutdown_guard,
        })
    }

    pub async fn from_pid<P>(
        pid: Pid,
        out_dir: P,
        _shutdown_guard: ShutdownGuard,
    ) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let process = heim::process::get(pid).await?;
        Monitor::new(process, out_dir, _shutdown_guard).await
    }

    // todo:
    pub async fn from_name<P>(
        name: &str,
        out_dir: P,
        _shutdown_guard: ShutdownGuard,
    ) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let processes = heim::process::processes().await?;
        pin_utils::pin_mut!(processes);
        let mut process = None;
        while let Some(Ok(p)) = processes.next().await {
            if let Ok(proc_name) = p.name().await {
                if proc_name == name {
                    process = Some(p);
                    break;
                }
            }
        }

        if let Some(process) = process {
            Monitor::new(process, out_dir, _shutdown_guard).await
        } else {
            anyhow::bail!("no process named {} was found", name)
        }
    }

    pub async fn run(&mut self, frequency: Duration) {
        let mut interval = tokio::time::interval(frequency);
        let mut tick = 0;
        loop {
            tokio::select! {
                biased;

                _ = self._shutdown_guard.notified() => {
                    break
                }

                _ = interval.tick() => {
                    tick += 1;
                    if tick % 3 == 0 {
                        self.store_stream.flush().await.expect("an error occurred while flushing to the store stream");
                    }

                    if !self.process.is_running().await.unwrap_or(false) {
                        break
                    }

                    match self.collect().await {
                        Err(err) => {
                            log::error!("an error occurred during collection: {:?}", err);
                            break
                        }
                        Ok(data) => {
                            log::info!("recording {}...", self.process.pid());
                            self.store_stream.write(&data).await.expect("an error occurred while writing to the store stream");
                        }
                    }
                }
            }
        }

        log::info!("stopping recording {}", self.process.pid());
        self.store_stream
            .flush()
            .await
            .expect("an error occurred while flushing to the store stream");
    }

    async fn collect(&mut self) -> anyhow::Result<Data> {
        let mem = self.process.memory().await?;
        let cpu_time = self.process.cpu_time().await?;
        let now_cpu_usage = self.process.cpu_usage().await?;
        let cpu_usage = self
            .last_cpu_usage
            .take()
            .map(|last_cpu_usage| {
                (now_cpu_usage.clone() - last_cpu_usage).get::<heim::units::ratio::percent>()
            })
            .unwrap_or(0f32);
        self.last_cpu_usage = Some(now_cpu_usage);

        cfg_if::cfg_if! {
            if #[cfg(target_os = "linux")] {
                use heim::process::os::linux::IoCountersExt;

                let io = self.process.io_counters().await?;
                let io = Io {
                    bytes_written: io.bytes_written().get::<heim::units::information::byte>(),
                    bytes_read: io.bytes_read().get::<heim::units::information::byte>(),
                    disk_written: Some(io.chars_written().get::<heim::units::information::byte>()),
                    disk_read: Some(io.chars_read().get::<heim::units::information::byte>()),
                    syscall_written: Some(io.write_syscalls()),
                    syscall_read: Some(io.read_syscalls()),
                };
            } else {
                let io = self.process.io_counters().await?;
            }
        }

        cfg_if::cfg_if! {
            if #[cfg(target_os = "linux")] {
                use heim::process::os::linux::ProcessExt;
                use crate::data::NetIo;

                let mut net_io_stream = self.process.net_io_counters().await?;
                let mut net_io = HashMap::default();

                while let Some(Ok(io)) = net_io_stream.next().await {
                    net_io.insert(io.interface().to_owned(), NetIo::from(io));
                }
            } else {
                let net_io = HashMap::default();
            }
        }

        let data = Data {
            memory: Memory::from(mem),
            cpu_time: CpuTime::from(cpu_time),
            cpu_usage: CpuUsage(cpu_usage),
            io: Io::from(io),
            net_io,
        };

        Ok(data)
    }
}
