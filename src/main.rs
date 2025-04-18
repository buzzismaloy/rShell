use colored::*;
use gethostname::gethostname;
use std::env;
use std::io::{Write, stdin, stdout};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

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

fn print_prompt() {
    let username = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let hostname = gethostname().to_string_lossy().into_owned();
    let current_path = format_path();

    print!(
        "{}@{} {}{}> ",
        username.cyan().bold(),
        hostname,
        "=".to_string().cyan(),
        current_path.cyan()
    );
    stdout().flush().unwrap();
}

fn main() {
    let mut oldpwd: Option<PathBuf> = None;
    loop {
        print_prompt();

        let mut input = String::new();
        if let Err(e) = stdin().read_line(&mut input) {
            eprintln!("Failed to read input due to the following error: {}", e);
            continue;
        }

        let mut commands = input.trim().split(" | ").peekable();
        let mut prev_command: Option<Child> = None;

        while let Some(command) = commands.next() {
            let mut parts = command.trim().split_whitespace();
            let Some(command) = parts.next() else {
                eprintln!("Error!!! Empty command in pipeline segment!");
                continue;
            };
            let args = parts;

            match command {
                "cd" => {
                    run_builtin_cd(args.map(|s| s.to_string()), &mut oldpwd);
                    prev_command = None;
                }

                "pwd" => {
                    run_builtin_pwd(args.map(|s| s.to_string()));
                    prev_command = None;
                }

                "help" => {
                    run_buitlin_help();
                    prev_command = None;
                }

                "exit" => return,

                command => {
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
            let path_suf = path.trim_start_matches('~');
            PathBuf::from(home.clone()).join(path_suf)
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
    let builtins = ["cd", "pwd", "help", "exit"];
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
