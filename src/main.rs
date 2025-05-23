use colored::*;
use dirs::home_dir;
use gethostname::gethostname;
use rustyline::Editor;
use rustyline::error::ReadlineError;
use rustyline::history::{DefaultHistory, History};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

const HISTSIZE: usize = 1500;
const HISTFILESIZE: usize = 2200;

fn format_path() -> String {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("?"));
    let home = env::var("HOME").unwrap_or_else(|_| "/".to_string());

    let path_str = cwd.to_string_lossy();
    if path_str == home {
        return "".to_string();
    }

    let relative = if let Some(stripped) = path_str.strip_prefix(&home) {
        stripped.trim_start_matches('/').to_string()
    } else {
        path_str.to_string()
    };

    let mut parts: Vec<&str> = relative.split('/').filter(|p| !p.is_empty()).collect();

    if parts.is_empty() {
        return "/".to_string();
    }

    if parts.len() == 1 {
        return format!("/{}", parts[0]);
    }

    let last = parts.pop().unwrap();
    let abbrev: Vec<String> = parts
        .into_iter()
        .map(|s| s.chars().next().unwrap_or('?').to_string())
        .collect();

    format!("/{}/{}", abbrev.join("/"), last)
}

fn get_prompt() -> String {
    let username = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let hostname = gethostname().to_string_lossy().into_owned();
    let current_path = format_path();

    format!(
        "{}@{} {}{}> ",
        username.cyan().bold(),
        hostname,
        "=".to_string().cyan(),
        current_path.cyan()
    )
}

fn expand_env_variables(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            let mut var_name = String::new();

            while let Some(&next) = chars.peek() {
                if next.is_alphanumeric() || next == '_' {
                    var_name.push(next);
                    chars.next();
                } else {
                    break;
                }
            }

            let rep = match var_name.as_str() {
                "HOME" => env::var("HOME").unwrap_or_default(),
                "PATH" => env::var("PATH").unwrap_or_default(),
                "USER" => env::var("USER").unwrap_or_default(),
                "PWD" => env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                "LANG" => env::var("LANG").unwrap_or_default(),

                _ => format!("UNKNOWN YET VARIABLE: ${}", var_name),
            };

            result.push_str(&rep);
        } else {
            result.push(c);
        }
    }

    result
}

fn main() {
    rshell_loop();
}

fn rshell_loop() {
    let mut oldpwd: Option<PathBuf> = None;
    let mut rl = Editor::<(), DefaultHistory>::new().unwrap();
    let history_path = get_history_path();
    let _ = rl.load_history(&history_path);

    loop {
        let prompt = get_prompt();
        let line = rl.readline(&prompt);

        match line {
            Ok(input) => {
                let trimmed = input.trim();
                if !trimmed.is_empty() {
                    let starts_with_space = input.starts_with(' ');

                    //HISTCONTROL=ignorespace
                    if !starts_with_space {
                        rl.add_history_entry(trimmed).ok();
                        trim_shell_history(&mut rl);
                    }
                }

                let expand_input = expand_env_variables(trimmed);
                let mut commands = expand_input.split('|').peekable();
                let mut prev_command: Option<Child> = None;

                while let Some(command) = commands.next() {
                    let mut parts = command.trim().split_whitespace();
                    let Some(command) = parts.next() else {
                        eprintln!("Error!!! Empty command in pipeline segment!");
                        continue;
                    };
                    let args = parts.collect::<Vec<_>>();

                    match command {
                        "cd" => {
                            run_builtin_cd(args.into_iter().map(|s| s.to_string()), &mut oldpwd);
                            prev_command = None;
                        }

                        "pwd" => {
                            run_builtin_pwd(args.into_iter().map(|s| s.to_string()));
                            prev_command = None;
                        }

                        "help" => {
                            run_buitlin_help();
                            prev_command = None;
                        }

                        "exit" => {
                            save_shell_history(&rl);
                            return;
                        }

                        "history" => {
                            run_builtin_history(&args, &mut rl);
                        }

                        _ => {
                            let stdin = match prev_command {
                                Some(mut output) => match output.stdout.take() {
                                    Some(out) => Stdio::from(out),
                                    None => Stdio::inherit(),
                                },
                                None => Stdio::inherit(),
                            };

                            let stdout = if commands.peek().is_some() {
                                Stdio::piped()
                            } else {
                                Stdio::inherit()
                            };

                            let output = Command::new(command)
                                .args(args)
                                .stdin(stdin)
                                .stdout(stdout)
                                .spawn();

                            match output {
                                Ok(output) => {
                                    prev_command = Some(output);
                                }
                                Err(e) => {
                                    prev_command = None;
                                    eprintln!("{}", e);
                                }
                            };
                        }
                    }
                }

                if let Some(mut fin_command) = prev_command {
                    fin_command.wait().unwrap();
                }
            }

            Err(e) => match e {
                ReadlineError::Interrupted => {
                    println!("SIGINT captured!");
                    continue;
                }

                ReadlineError::Eof => {
                    println!("Captured Ctrl + D (EOF), terminating...");
                    save_shell_history(&rl);
                    break;
                }

                _ => {
                    println!("Error: {}", e);
                    save_shell_history(&rl);
                    break;
                }
            },
        }
    }
}

fn trim_shell_history(rl: &mut Editor<(), DefaultHistory>) {
    let history_entries: Vec<String> = rl.history().iter().map(|s| s.to_string()).collect();
    let start = if history_entries.len() > HISTSIZE {
        history_entries.len() - HISTSIZE
    } else {
        0
    };

    let trimmed = &history_entries[start..];

    rl.clear_history().ok();
    for entry in trimmed {
        rl.add_history_entry(entry.as_str()).ok();
    }
}

fn save_shell_history(rl: &Editor<(), DefaultHistory>) {
    let path = get_history_path();
    let history_entries: Vec<String> = rl.history().iter().map(|s| s.to_string()).collect();
    let total = history_entries.len();
    let start = if total > HISTFILESIZE {
        total - HISTFILESIZE
    } else {
        0
    };

    let trimmed = &history_entries[start..];

    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&path)
        .unwrap();

    let mut writer = std::io::BufWriter::new(file);
    for entry in trimmed {
        writeln!(writer, "{}", entry).ok();
    }
}

fn get_history_path() -> PathBuf {
    let mut path = home_dir().unwrap();
    path.push(".rshell_history");
    path
}

fn run_builtin_history(args: &[&str], rl: &mut Editor<(), DefaultHistory>) {
    match args {
        ["-c"] => {
            rl.clear_history().ok();
            save_shell_history(rl);
            println!("History cleared.");
        }

        ["-w"] => {
            save_shell_history(rl);
            println!("History was written to file.")
        }

        [] => {
            let hist = rl.history();
            let start = if hist.len() > HISTSIZE {
                hist.len() - HISTSIZE
            } else {
                0
            };
            for (i, cmd) in hist.iter().skip(start).enumerate() {
                println!("{:>5} {}", start + i + 1, cmd);
            }
        }

        _ => {
            eprintln!("Usage: history [-c] or [-w]");
        }
    }
}

fn run_builtin_cd<I: Iterator<Item = String>>(mut args: I, oldpwd: &mut Option<PathBuf>) {
    let target = args.next();
    let home = env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

    let dest_path = match target.as_deref() {
        Some("-") => {
            if let Some(prev_dir) = oldpwd.as_ref() {
                println!("Previous path is: {}", prev_dir.display());
                prev_dir.clone()
            } else {
                eprintln!("cd: previous directory is not set!");
                return;
            }
        }

        Some(path) if path.starts_with("~") => {
            if path == "~" || path == "~/" {
                PathBuf::from(home.clone())
            } else {
                let path_suf = path.trim_start_matches("~/");
                PathBuf::from(home.clone()).join(path_suf)
            }
        }

        Some(path) => PathBuf::from(path),

        None => {
            let current_str = current_dir.to_string_lossy();
            if current_str == home {
                return;
            }
            PathBuf::from(home)
        }
    };

    if let Err(e) = env::set_current_dir(&dest_path) {
        eprintln!("Occured error in builtin cd: {}", e);
    } else {
        *oldpwd = Some(current_dir);
    }
}

fn run_buitlin_help() {
    let builtins = ["cd", "pwd", "help", "history", "exit"];
    println!(
        "This is a {} - Shell written in Rust.",
        "rShell".to_string().cyan().bold()
    );
    println!("Here is the list of built-in functions:");

    for i in builtins {
        println!("\t{}", i);
    }

    println!("\nUse man command for more information on other programs");
}

fn run_builtin_pwd(args: impl Iterator<Item = String>) {
    let mut physical = true;
    let mut show_help = false;

    for arg in args {
        match arg.as_str() {
            "-P" | "--physical" | "-p" => physical = true,
            "-L" | "--logical" | "-l" => physical = false,
            "-h" | "--help" => show_help = true,
            _ => {
                eprintln!("built-in pwd: unrecognized option '{}'", arg);
                return;
            }
        }
    }

    if show_help {
        println!("Name\n\tpwd - output the current working directory\n");
        println!("Usage: pwd [OPTION]");
        println!("  -L, --logical     use PWD from environment, even if it contains symlinks");
        println!("  -P, --physical    avoid all symlinks");
        println!("  -h, --help        display this help and exit");

        return;
    }

    let path = if physical {
        env::current_dir().unwrap_or_default()
    } else {
        env::var("PWD")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().unwrap_or_default())
    };

    println!("{}", path.display());
}
