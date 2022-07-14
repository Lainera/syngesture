mod config;
mod events;

use config::Action;
use events::{EventLoop, Gesture};
use log::{info, trace, warn};
use regex::Regex;
use std::path::PathBuf;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

fn print_version<W: std::io::Write>(target: &mut W) {
    writeln!(
        target,
        "syngestures {} - Copyright NeoSmart Technologies 2020-2022",
        env!("CARGO_PKG_VERSION")
    )
    .ok();
    writeln!(
        target,
        "Developed by Mahmoud Al-Qudsi and other syngestures contributors"
    )
    .ok();
    writeln!(
        target,
        "Report bugs at <https://github.com/mqudsi/syngesture>"
    )
    .ok();
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
        "",
        "A valid syngestures config file must be installed to one of the",
        "following locations before executing syngestures:",
    ] {
        writeln!(target, "{}", line).ok();
    }

    for dir in config::config_dirs() {
        writeln!(target, "  * {}", dir).ok();
    }

    for line in [
        "",
        "A sample configuration file can be found in the package tarball or online at",
    ] {
        writeln!(target, "{}", line).ok();
    }

    let _ = writeln!(
        target,
        "<https://raw.githubusercontent.com/mqudsi/syngesture/{}/syngestures.toml>",
        env!("CARGO_PKG_VERSION")
    );
}

fn main() {
    let args = std::env::args();
    for arg in args.skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help(&mut std::io::stdout());
                std::process::exit(0);
            }
            "-V" | "--version" => {
                print_version(&mut std::io::stdout());
                std::process::exit(0);
            }
            _ => {
                eprintln!("{}: Invalid option!", arg);
                eprintln!("Try 'syngestures --help' for more info");
                std::process::exit(-1);
            }
        }
    }

    let config = config::load();

    if which("evtest").is_none() {
        eprintln!("Cannot find `evtest` - make sure it is installed and try again!");
        std::process::exit(-1);
    }

    if config.devices.is_empty() {
        eprintln!("No configured devices");
        std::process::exit(-1);
    }

    // Event: time 1593656931.323635, type 3 (EV_ABS), code 47 (ABS_MT_SLOT), value 0
    let event_regex = std::sync::Arc::new(
        Regex::new(r#"time (\d+\.\d+), type (\d+) .* code (\d+) .* value (\d+)"#).unwrap(),
    );

    let mut threads = Vec::new();
    for (device, gestures) in config.devices {
        let event_regex = event_regex.clone();
        let handle = std::thread::spawn(move || {
            let mut event_loop = EventLoop::new();

            let evtest = Command::new("evtest")
                .args(&[&device])
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();

            let reader = BufReader::new(evtest.stdout.unwrap());
            for line in reader.lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(_) => break,
                };

                // Event: time 1593656931.306879, -------------- SYN_REPORT ------------
                if line.contains("SYN_REPORT") {
                    if let Some(gesture) = event_loop.update() {
                        swipe_handler(&gestures, gesture);
                    }
                    continue;
                }

                if let Some(captures) = event_regex.captures(&line) {
                    let time: f64 = captures[1].parse().unwrap();
                    let event_type: u8 = captures[2].parse().unwrap();
                    let code: u16 = captures[3].parse().unwrap();
                    let value: i32 = captures[4].parse().unwrap();

                    trace!("{}", line);
                    event_loop.add_event(time, event_type, code, value);
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
        &Action::None => {}
        &Action::Execute(ref cmd) => {
            let mut shell = Command::new("sh");
            shell.args(&["-c", cmd]);
            let mut child = match shell.spawn() {
                Ok(child) => child,
                Err(e) => {
                    eprintln!("{}", e);
                    return;
                }
            };

            // Spawn a thread to wait on the process to finish executing.
            // TODO: Just have one thread wait on all launched processes.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }
}

fn which(target: &str) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::unix::prelude::OsStringExt;

    let mut cmd = Command::new("which");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());
    cmd.args(&[target]);
    let output = match cmd.output() {
        Err(_) => {
            warn!("Failed to find/execute `which`");
            return None;
        }
        Ok(output) => output,
    };

    if output.status.success() {
        let path = OsString::from_vec(output.stdout);
        return Some(PathBuf::from(path));
    }

    return None;
}

// fn xdotool(command: &'static str, actions: &'static str) {
//     use std::thread;
//
//     thread::spawn(move || {
//         Command::new("xdotool")
//             .args(&[command, actions])
//             .output()
//             .expect("Failed to run xdotool");
//     });
// }
