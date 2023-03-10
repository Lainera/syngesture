mod config;
mod epoll;
#[cfg(not(feature = "logging"))]
mod errorlog;
mod events;

use config::Action;
use epoll::Epoll;
use evdev_rs::Device as EvDevice;
use events::{EventLoop, Gesture};
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use std::io::ErrorKind;
use std::os::fd::AsRawFd;
use std::process::Command;

fn print_version<W: std::io::Write>(target: &mut W) {
    let _ = writeln!(
        target,
        "syngestures {} - Copyright NeoSmart Technologies 2020-2022",
        env!("CARGO_PKG_VERSION")
    );

    for line in [
        "Developed by Mahmoud Al-Qudsi and other syngestures contributors",
        "Report bugs at <https://github.com/mqudsi/syngesture>",
    ] {
        writeln!(target, "{line}").ok();
    }
}

fn print_help<W: std::io::Write>(target: &mut W) {
    print_version(&mut *target);
    for line in [
        "",
        "Usage: syngestures [OPTIONS]",
        "",
        "Options:",
        "  -h --help     Print this help message",
        "  -V --version  Print version info",
        "  -f $CONFIG_FILE_PATH Load config file",
        "",
        "A valid syngestures config file must be installed to one of the",
        "following locations before executing syngestures:",
    ] {
        writeln!(target, "{line}").ok();
    }

    for dir in config::config_dirs() {
        writeln!(target, "  * {dir}").ok();
    }

    for line in [
        "",
        "A sample configuration file can be found in the package tarball or online at",
    ] {
        writeln!(target, "{line}").ok();
    }

    let _ = writeln!(
        target,
        "<https://raw.githubusercontent.com/mqudsi/syngesture/{}/syngestures.toml>",
        env!("CARGO_PKG_VERSION")
    );
}

#[cfg(feature = "logging")]
fn init_logger() {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "trace");
    }
    pretty_env_logger::init();
}

#[cfg(not(feature = "logging"))]
fn init_logger() {
    errorlog::init();
}

fn main() {
    init_logger();

    let mut args = std::env::args();
    let config = args
        .nth(1)
        .map(|arg| match arg.as_str() {
            "-f" | "--file" => {
                let path = args
                    .next()
                    .expect("config file path must follow the file flag");
                config::load_path(path)
            }
            "-h" | "--help" => {
                print_help(&mut std::io::stdout());
                std::process::exit(0);
            }
            "-v" | "--version" => {
                print_version(&mut std::io::stdout());
                std::process::exit(0);
            }
            _ => {
                eprintln!("{arg}: Invalid option!");
                eprintln!("Try 'syngestures --help' for more info");
                std::process::exit(-1);
            }
        })
        .unwrap_or_else(config::load);

    if config.devices.is_empty() {
        error!("No configured devices");
        std::process::exit(-1);
    }

    let mut threads = Vec::new();
    for (device_path, gestures) in config.devices {
        let device = match EvDevice::new_from_path(&device_path) {
            Ok(device) => device,
            Err(e) => {
                error!("{device_path}: {e}");
                continue;
            }
        };
        let device_fd = device.file().as_raw_fd();
        let handle = std::thread::spawn(move || {
            use evdev_rs::enums::*;
            use evdev_rs::{InputEvent, ReadFlag, ReadStatus};

            let mut epoll = Epoll::new().unwrap();
            epoll.register_read(device_fd, false).unwrap();

            let mut event_loop = EventLoop::new();
            let mut read_flag = ReadFlag::NORMAL;
            'device: loop {
                let event = match device.next_event(read_flag) {
                    Ok((ReadStatus::Success, event)) => event,
                    Ok((
                        ReadStatus::Sync,
                        InputEvent {
                            event_code: EventCode::EV_SYN(EV_SYN::SYN_DROPPED),
                            ..
                        },
                    )) => {
                        read_flag = ReadFlag::SYNC;
                        continue;
                    }
                    Ok((ReadStatus::Sync, event)) => event,
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        read_flag = ReadFlag::NORMAL;
                        loop {
                            match epoll.wait(None) {
                                Ok(()) => continue 'device,
                                Err(e) => {
                                    if e.kind() == ErrorKind::Interrupted {
                                        continue;
                                    }
                                    error!("epoll_wait: {e}");
                                    break 'device;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("{device_path}: {e}");
                        break;
                    }
                };

                let result = event_loop.add_event(event.time, event.event_code, event.value);
                if let Some(gesture) = result {
                    swipe_handler(&gestures, gesture);
                }
            }
        });
        threads.push(handle);
    }

    for thread in threads {
        thread.join().unwrap();
    }
}

fn swipe_handler(gestures: &config::GestureMap, gesture: Gesture) {
    info!("{:?}", gesture);

    let action = match gestures.get(&gesture) {
        Some(action) => action,
        None => return,
    };

    match action {
        Action::None => {}
        Action::Execute(cmd) => {
            let mut shell = Command::new("sh");
            shell.args(["-c", cmd]);
            let mut child = match shell.spawn() {
                Ok(child) => child,
                Err(e) => {
                    error!("{e}");
                    return;
                }
            };

            // Spawn a thread to wait on the process to finish executing.
            // This is only here to avoid zombie processes from piling up.
            // TODO: Just have one thread wait on all launched processes.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }
}
