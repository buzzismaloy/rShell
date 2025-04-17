use std::env;
use std::io::{Write, stdin, stdout};
use std::path::Path;
use std::process::{Child, Command, Stdio};

fn main() {
    loop {
        print!("=> ");
        stdout().flush();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        let mut parts = input.trim().split_whitespace();
        let command = parts.next().unwrap();
        let args = parts;

        match command {
            "cd" => {
                let new_dir = args.peekable().peek().map_or("/", |x| *x);
                let new_path = Path::new(new_dir);
                if let Err(e) = env::set_current_dir(&new_path) {
                    eprintln!("{}", e);
                }
            }

            "exit" => return,

            command => {
                let child_process = Command::new(command).args(args).spawn();

                match child_process {
                    Ok(mut child_process) => {
                        child_process.wait();
                    }
                    Err(e) => eprintln!("{}", e),
                };
            }
        }
    }
}
