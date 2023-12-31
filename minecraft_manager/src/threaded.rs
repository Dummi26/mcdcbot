use std::{
    fmt::Display,
    io::{BufRead, BufReader, Write},
    process::{ExitStatus, Stdio},
};

use crate::{
    parse_line::{parse_line, ParseOutput},
    MinecraftServerType,
};

use {
    crate::tasks::MinecraftServerTask,
    crate::{
        events::{self as MinecraftServerEvents, MinecraftServerEvent, MinecraftServerEventType},
        MinecraftServerSettings,
    },
    std::sync::mpsc,
};

pub fn run(
    settings: MinecraftServerSettings,
) -> (
    mpsc::Sender<(MinecraftServerTask, mpsc::Sender<Result<u8, String>>)>,
    mpsc::Receiver<MinecraftServerEvent>,
    std::thread::JoinHandle<MinecraftServerStopReason>,
) {
    let (return_task_sender, tasks) =
        mpsc::channel::<(MinecraftServerTask, mpsc::Sender<Result<u8, String>>)>();
    let (events, return_events_receiver) = mpsc::channel();

    // thread
    let join_handle = std::thread::spawn(move || {
        let mut command = settings.get_command();
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        eprintln!("Spawning {command:?}");
        match command.spawn() {
            Ok(mut process) => {
                if let (Some(mut stdin), Some(stdout), Some(mut _stderr)) = (
                    process.stdin.take(),
                    process.stdout.take(),
                    process.stderr.take(),
                ) {
                    let stdout_lines = {
                        // the stdout reading thread
                        let (lines, stdout_lines) = mpsc::channel();
                        std::thread::spawn(move || {
                            let mut stdout = BufReader::new(stdout);
                            let mut line = String::new();
                            loop {
                                line.clear();
                                match stdout.read_line(&mut line) {
                                    Ok(_) if !line.trim().is_empty() => {
                                        eprintln!("> {}", line.trim());
                                        match lines.send(line.trim().to_owned()) {
                                            Ok(_) => (),
                                            Err(_) => return,
                                        }
                                    }
                                    Ok(0) => {
                                        eprintln!(
                                            " [ Stdout read thread ]    Reached EOF, stopping."
                                        );
                                        return;
                                    }
                                    Ok(_) => {} // empty line, but read newline char - ignore
                                    Err(e) => {
                                        eprintln!(
                                            " [ Stdout read thread ]    Read error, stopping. ({e:?})"
                                        );
                                        return;
                                    }
                                }
                            }
                        });
                        stdout_lines
                    };
                    loop {
                        while let Ok(task) = tasks.try_recv() {
                            eprintln!("[GOT TASK] {task:?}");
                            // iterate over all new tasks
                            match task.0 {
                                MinecraftServerTask::Stop => match writeln!(stdin, "stop") {
                                    Ok(_) => {
                                        task.1.send(Ok(0));
                                        while let Ok(None) = process.try_wait() {
                                            std::thread::sleep(std::time::Duration::from_millis(
                                                250,
                                            ));
                                        }
                                        task.1.send(Ok(100));
                                    }
                                    Err(e) => {
                                        events.send(MinecraftServerEvent {
                                            time: (),
                                            event: MinecraftServerEventType::Warning(
                                                MinecraftServerEvents::MinecraftServerWarning::CantWriteToStdin(e),
                                            ),
                                        });
                                    }
                                },
                                MinecraftServerTask::Kill => {
                                    process.kill();
                                    task.1.send(Ok(100));
                                    return MinecraftServerStopReason {
                                        time: (),
                                        reason: MinecraftServerStopReasons::KilledDueToTask,
                                    };
                                }
                                MinecraftServerTask::RunCommand(command) => {
                                    match writeln!(
                                        stdin,
                                        "{}",
                                        command.replace("\n", "\\n").replace("\r", "\\r")
                                    ) {
                                        Ok(_) => task.1.send(Ok(100)),
                                        Err(_) => task.1.send(Ok(101)),
                                    };
                                }
                            }
                        }
                        while let Ok(line) = stdout_lines.try_recv() {
                            // iterate over all new lines from stdout
                            // eprintln!(" [ server manager thread ]    Found line '{}'", line);
                            match parse_line(&line, &settings) {
                                ParseOutput::Event(event) => {
                                    events.send(MinecraftServerEvent { time: (), event });
                                }
                                ParseOutput::Error(_) => (),
                                ParseOutput::Nothing => (),
                            }
                        }
                        // stop the loop once the process exits
                        match process.try_wait() {
                            Ok(None) => (),
                            Ok(Some(exit_status)) => {
                                if let MinecraftServerType::Custom {
                                    line_parser_proc, ..
                                } = &settings.server_type
                                {
                                    if let Some(proc) = &mut *line_parser_proc.lock().unwrap() {
                                        _ = proc.0.kill();
                                    }
                                }
                                return MinecraftServerStopReason {
                                    time: (),
                                    reason: MinecraftServerStopReasons::ProcessEnded(exit_status),
                                };
                            }
                            Err(e) => {
                                return MinecraftServerStopReason {
                                    time: (),
                                    reason: MinecraftServerStopReasons::ProcessCouldNotBeAwaited(e),
                                }
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_millis(200));
                    }
                } else {
                    eprintln!("No stdin/out!");
                    events.send(MinecraftServerEvent {
                        time: (),
                        event: MinecraftServerEventType::Warning(
                            MinecraftServerEvents::MinecraftServerWarning::CouldNotGetServerProcessStdio,
                        ),
                    });
                    match process.wait() {
                        Ok(status) => MinecraftServerStopReason {
                            time: (),
                            reason: MinecraftServerStopReasons::ProcessEnded(status),
                        },
                        Err(e) => MinecraftServerStopReason {
                            time: (),
                            reason: MinecraftServerStopReasons::ProcessCouldNotBeAwaited(e),
                        },
                    }
                }
            }
            Err(e) => {
                eprintln!("Couldn't spawn server process: {e:?}");
                MinecraftServerStopReason {
                    time: (),
                    reason: MinecraftServerStopReasons::ProcessCouldNotBeSpawned(e),
                }
            }
        }
    });
    // return the mpsc channel parts
    (return_task_sender, return_events_receiver, join_handle)
}

pub struct MinecraftServerStopReason {
    time: (),
    reason: MinecraftServerStopReasons,
}
impl Display for MinecraftServerStopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

pub enum MinecraftServerStopReasons {
    KilledDueToTask,
    ProcessEnded(ExitStatus),
    ProcessCouldNotBeSpawned(std::io::Error),
    ProcessCouldNotBeAwaited(std::io::Error),
}
impl Display for MinecraftServerStopReasons {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KilledDueToTask => write!(f, "killed (due to task)"),
            Self::ProcessEnded(exit_status) => {
                if let Some(s) = exit_status.code() {
                    if s == 0 {
                        write!(f, "Stopped")
                    } else {
                        write!(f, "Stopped (Exited with status {s})!")
                    }
                } else {
                    write!(f, "Stopped!")
                }
            }
            Self::ProcessCouldNotBeSpawned(_e) => {
                write!(f, "Couldn't spawn process (check your paths!)")
            }
            Self::ProcessCouldNotBeAwaited(_e) => write!(
                f,
                "Couldn't wait for process to end (check console/log for errors)"
            ),
        }
    }
}
