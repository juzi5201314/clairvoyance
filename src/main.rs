use std::fs::OpenOptions;
use std::path::PathBuf;
use std::time::Duration;

use argh::FromArgs;
use heim::process::Pid;

use clairvoyance::draw::{render_cpu_time, render_cpu_usage, render_memory};
use clairvoyance::monitor::Monitor;
use clairvoyance::shutdown_notify::ShutdownNotify;
use clairvoyance::store::StoreStream;

#[tokio::main]
async fn main() {
    setup_logger();

    let args = argh::from_env::<Arguments>();

    match args.sub_cmd {
        SubCommandEnum::Record(args) => {
            if args.pid.is_empty() {
                log::warn!("no process that needs to record");
                return;
            }

            let shutdown_handle = ShutdownNotify::new();

            for pid in args.pid {
                let mut monitor = Monitor::from_pid(pid, &args.out_dir, shutdown_handle.start())
                    .await
                    .unwrap();
                tokio::spawn(async move {
                    monitor.run(args.frequency.0).await;
                });
            }

            shutdown_handle.wait_shutdown(args.shutdown_timeout.0).await;
        }
        SubCommandEnum::Render(args) => {
            let mut stream = StoreStream::open(args.file)
                .await
                .expect("failed to open store stream");
            let mut data = Vec::new();
            while let Some(d) = stream.read().await.unwrap() {
                data.push(d);
            }

            if args.json {
                serde_json::to_writer(
                    OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .create(true)
                        .open(args.out_dir.join("result.json"))
                        .unwrap(),
                    &data,
                )
                .unwrap();
            }

            if args.memory {
                render_memory(&data, args.out_dir.join("memory.svg")).unwrap();
            }
            if args.cpu {
                render_cpu_time(&data, args.out_dir.join("cpu_time.svg")).unwrap();
                render_cpu_usage(&data, args.out_dir.join("cpu_usage.svg")).unwrap();
            }
        }
    }
}

fn setup_logger() {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()
        .unwrap()
}

#[derive(FromArgs)]
/// Clairvoyance Arguments
struct Arguments {
    #[argh(subcommand)]
    sub_cmd: SubCommandEnum,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum SubCommandEnum {
    Record(SubCommandRecord),
    Render(SubCommandRender),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "record")]
/// record process
struct SubCommandRecord {
    #[argh(positional)]
    /// process pid
    pid: Vec<Pid>,

    #[argh(
        option,
        short = 'f',
        default = "ParseDuration(Duration::from_millis(500))"
    )]
    /// scanning frequency. default: 500ms
    frequency: ParseDuration,

    #[argh(option, default = "ParseDuration(Duration::from_secs(3))")]
    /// shutdown timeout. default: 3s
    shutdown_timeout: ParseDuration,

    #[argh(option, short = 'o', default = "PathBuf::new().join(\".\")")]
    /// output directory. default: "."
    out_dir: PathBuf,
}

#[derive(FromArgs)]
#[argh(subcommand, name = "render")]
/// render result
struct SubCommandRender {
    #[argh(positional)]
    /// the intermediate file obtained by record
    file: PathBuf,

    #[argh(option, short = 'o', default = "PathBuf::new().join(\".\")")]
    /// output directory. default: "."
    out_dir: PathBuf,

    #[argh(switch, short = 'm')]
    /// render memory result
    memory: bool,

    #[argh(switch, short = 'c')]
    /// render cpu result
    cpu: bool,

    #[argh(switch, short = 'j')]
    /// convert intermediate files to json format
    json: bool,
}

struct ParseDuration(Duration);

impl argh::FromArgValue for ParseDuration {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        parse_duration::parse(value)
            .map(ParseDuration)
            .map_err(|_| value.to_string())
    }
}
