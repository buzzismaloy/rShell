use std::io::{Write, stdin, stdout};
use std::process::{Child, Command, Stdio};

fn main() {
    loop {
        print!("=> ");
        stdout().flush();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        let command = input.trim();

        let mut child_process = Command::new(command).spawn().unwrap();

        child_process.wait();
    }
}
