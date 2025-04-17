use std::env;
use std::io::{Write, stdin, stdout};
use std::path::Path;
use std::process::{Child, Command, Stdio};

fn main() {
    loop {
        print!("=> ");
        stdout().flush().unwrap();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        let mut commands = input.trim().split(" | ").peekable();
        let mut prev_command = None;

        while let Some(command) = commands.next() {
            let mut parts = command.trim().split_whitespace();
            let command = parts.next().unwrap();
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

                "exit" => return,

                command => {
                    let stdin = prev_command.map_or(Stdio::inherit(), |output: Child| {
                        Stdio::from(output.stdout.unwrap())
                    });

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
