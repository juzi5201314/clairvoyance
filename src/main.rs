use std::path::PathBuf;
use std::time::Duration;

use argh::FromArgs;
use heim::process::Pid;

use clairvoyance::monitor::Monitor;
use clairvoyance::shutdown_notify::ShutdownNotify;

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
        SubCommandEnum::Render(_) => {
            todo!()
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
struct SubCommandRender {}

struct ParseDuration(Duration);

impl argh::FromArgValue for ParseDuration {
    fn from_arg_value(value: &str) -> Result<Self, String> {
        parse_duration::parse(value)
            .map(ParseDuration)
            .map_err(|_| value.to_string())
    }
}
