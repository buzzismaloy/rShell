use gethostname::gethostname;
use std::env;
use std::io::{Write, stdin, stdout};
use std::path::{Path, PathBuf};
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

    let mut parts: Vec<&str> = relative.split('/').collect();

    if parts.len() == 1 {
        return parts[0].to_string();
    }

    let last = parts.pop().unwrap();
    let abbrev: Vec<String> = parts
        .into_iter()
        .map(|s| s.chars().next().unwrap().to_string())
        .collect();

    format!("{}/{}", abbrev.join("/"), last)
}

fn main() {
    loop {
        let username = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        let hostname = gethostname().to_string_lossy().into_owned();
        let current_path = format_path();

        print!(
            "{}@{} ={}> ",
            username,
            hostname,
            if current_path.is_empty() {
                "".to_string()
            } else {
                format!("{}", current_path)
            }
        );
        stdout().flush().unwrap();

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
                    let new_dir = args.peekable().peek().map_or("/", |x| *x);
                    let new_path = Path::new(new_dir);
                    if let Err(e) = env::set_current_dir(&new_path) {
                        eprintln!("{}", e);
                    }
                    prev_command = None;
                }

                "pwd" => {
                    match env::current_dir() {
                        Ok(path) => {
                            println!("{}", path.display());
                        }
                        Err(e) => {
                            eprintln!("Error occured getting current path: {}", e);
                        }
                    }
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
