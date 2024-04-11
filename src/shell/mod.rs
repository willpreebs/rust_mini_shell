use std::env::Args;
use std::fmt::Octal;
use std::io::{self, BufRead};
use std::io::Write;
use std::ffi::{CStr, CString};
use libc::{c_int, O_CREAT, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY};
use nix::{unistd::{fork, ForkResult, execvp}, sys::wait::wait};
use std::fs::File;

use std::process::exit;

mod scanner;
mod tokens;

const SHELL_PROMPT : &str = "shell $ ";

// built-in commands with no additional arguments
const BUILT_IN_SINGLE_ARG : [&str; 3] = ["quit", "prev", "help"];

// built-in commands with possible additional arguments
const BUILT_IN_MULT_ARGS : [&str; 2] = ["cd", "source"];

#[derive(Debug)]
#[derive(PartialEq)]
pub enum Command {
    Empty,
    Tokens(Box<Vec<String>>),
    // Commands(Box<Vec<Command>>),
    InputRedirect(String, Box<Command>),
    OutputRedirect(String, Box<Command>),
}

fn output(out: &str, newline: bool) {
    if newline {
        println!("{}", out);
    }
    else {
        print!("{}", out);
    }
}

fn get_help_message() -> String {
    return scanner::scan_in_text("help.txt");
}

pub fn run_shell(_args : Args) {

    output(get_help_message().as_str(), false);

    let mut prev : Command = Command::Empty;

    loop {

        output(SHELL_PROMPT, false);
        io::stdout().flush().unwrap();

        let mut user_input : String = String::new();

        io::stdin()
            .read_line(&mut user_input)
            .expect("Failed to read line");

        let user_input = user_input.trim();

        let command_sequence: Vec<Command> = tokens::get_command(user_input);

        println!("{:?}", command_sequence);

        match handle_single_arg_built_in(user_input, &prev) {
            LoopBehavior::BREAK => break,
            LoopBehavior::CONTINUE => continue,
            LoopBehavior::SKIP => ()
        }

        for command in command_sequence {
            dispatch_command(&command);
            prev = command;
        }
    }
}

enum LoopBehavior {
    BREAK,
    CONTINUE,
    SKIP
}

fn handle_single_arg_built_in(input: &str, prev: &Command) -> LoopBehavior {
    match input.to_ascii_lowercase().as_str() {
        "quit" => {
            output("Goodbye", true);
            return LoopBehavior::BREAK;
        },
        "prev" => {
            dispatch_command(prev);
            return LoopBehavior::CONTINUE;
        },
        "help" => {
            output(get_help_message().as_str(), true);
            return LoopBehavior::CONTINUE;
        }
        _ => return LoopBehavior::SKIP
    }
}

fn dispatch_command(command: &Command) {

    match command {
        Command::Empty => {},
        Command::Tokens(boxed_vec) => {
            let fst: String = boxed_vec[0].clone();
            // output(&fst, true);

            if BUILT_IN_MULT_ARGS.contains(&fst.as_str()) {
                match fst.as_str() {
                    "cd" => {
                        execute_cd(&boxed_vec[1]);
                    }
                    "source" => {
                        execute_source(&boxed_vec);
                    }
                    _ => {
                        output("Branch missing for handling of built-in multi-arg", true);
                    }
                }
                return;
            }

            match unsafe{fork()} {
                Ok(ForkResult::Child) => {

                    let exec_file = CString::new(boxed_vec[0].clone()).unwrap();
                    let cstrings: Vec<CString> = boxed_vec.iter().map(|s| convert_string_to_cstring(s)).collect();
                    match execvp(&exec_file, &cstrings) {
                        Ok(_) => (),
                        Err(e) => output(format!("execvp failed with error: {}", e).as_str(), true)
                    }
                    
                    exit(0);
                },
                Ok(ForkResult::Parent { child: pid }) => {
                    wait().expect(format!("Execution of program within child process {} failed", pid).as_str());
                },
                Err(e) => panic!("Creation of child process failed with error {}", e)
            }
        },
        Command::InputRedirect(src, c) => {
            match unsafe{fork()} {
                Ok(ForkResult::Child) => {
                    // close stdin
                    let close_result = unsafe{libc::close(0)};
                    if close_result == -1 {
                        output("Error closing stdin", true);
                    }

                    // redirect input to src file
                    let open_result = unsafe{libc::open(src.as_ptr() as *const i8, O_RDONLY)};
                    if open_result == -1 {
                        output(format!("File not found: {}", src).as_str(), true);
                    }
                    dispatch_command(c);
                    exit(0);
                },
                Ok(ForkResult::Parent { child: pid }) => {
                    wait().expect(format!("Execution of program within child process {} failed", pid).as_str());
                }
                Err(e) => panic!("Creation of child process failed with error {}", e)
            }   
        },
        Command::OutputRedirect(dst, c) => {

            match unsafe{fork()} {
                Ok(ForkResult::Child) => {
                    // close stdout
                    let close_result = unsafe{libc::close(1)};
                    if close_result == -1 {
                        output("Error closing stdin", true);
                    }

                    let mut clone = dst.clone(); 
                    clone.push('\0');

                    // redirect output to dst file
                    // decimal(438) = octal(666): create file in mode 666
                    let open_result = unsafe{libc::open(clone.as_ptr() as *const i8, O_RDWR | O_CREAT | O_TRUNC, 438)};
                    if open_result == -1 {
                        output(format!("Problem with creating or writing to: {}", dst).as_str(), true);
                    }
                    dispatch_command(c);
                    exit(0);
                },
                Ok(ForkResult::Parent { child: pid }) => {
                    wait().expect(format!("Execution of program within child process {} failed", pid).as_str());
                }
                Err(e) => panic!("Creation of child process failed with error {}", e)
            }
        }
    }
}

fn convert_string_to_cstring(s: &String) -> CString {
    return CString::new(s.as_str()).unwrap();
}


fn execute_cd(path: &String) {
    match std::env::set_current_dir(path) {
        Ok(_) => (),
        Err(e) => output(format!("cd failed with error: {}. Path was {}", e, path).as_str(), true)
    }
}

fn execute_source(command_tokens : &Vec<String>) {

    let filename = command_tokens[1].as_str();
    let file_result = File::open(&filename);
    let file: File;

    match file_result {
        Ok(t) => file = t,
        Err(e) => {
            output(format!("Error: {} in finding file: {}", e, filename).as_str(), true);
            return;
        }
    }
    let buf_reader = io::BufReader::new(file);
    let mut commands_vec: Vec<Command> = Vec::new();

    for line in buf_reader.lines() {
        match line {
            Ok(s) => {
                let mut command_sequence: Vec<Command> = tokens::get_command(s.as_str());
                commands_vec.append(&mut command_sequence);
            }
            Err(_) => output("Error reading line", true)
        }
    }
    for c in commands_vec {
        dispatch_command(&c);
    }
}

// #[cfg(test)]
// mod sequence_tests {
//     use super::*;

//     #[test]
//     fn sequence_test1() {
//         let sample_tokens: Vec<String> = ["ex", "token", "list"].map(String::from).to_vec();
//         let expected_meta_tokens: Vec<Vec<String>> = vec![["ex", "token", "list"].map(String::from).to_vec()];

//         assert_eq!(sequence(&sample_tokens), expected_meta_tokens);
//     }

//     #[test]
//     fn sequence_test2() {
//         let sample_tokens: Vec<String> = ["ex", ";", "list"].map(String::from).to_vec();

//         let expected_meta_tokens: Vec<Vec<String>> = vec![vec![String::from("ex")], vec![String::from("list")]];

//         assert_eq!(sequence(&sample_tokens), expected_meta_tokens);
//     }


//     #[test]
//     fn sequence_test3() {
//         let sample_tokens: Vec<String> = vec!["ex", "howdy", ";", "list"].iter().map(|&s| s.into()).collect();

//         let expected_meta_tokens: Vec<Vec<String>> = vec![vec![String::from("ex"), String::from("howdy")], vec![String::from("list")]];

//         assert_eq!(sequence(&sample_tokens), expected_meta_tokens);
//     }
// }

#[cfg(test)]
mod redirect_tests {

    use super::*;

    fn into_owned_string_vec(str_vec: Vec<&str>) -> Vec<String> {
        str_vec.iter().map(|s| String::from(*s)).collect()
    }


    #[test]
    fn redirect_test1() {

        let input = "exe input > outputfile";

        let token_command = Command::Tokens(Box::new(into_owned_string_vec(vec!["exe", "input"])));
        let expected_command = Command::OutputRedirect(String::from("outputfile"), Box::new(token_command));

        
        let actual_command = tokens::get_command(input);

        assert_eq!(expected_command, actual_command[0]);
    }


    
    #[test]
    fn redirect_test2() {

        let input = "exe < inputfile > outputfile";

        let token_command = Command::Tokens(Box::new(into_owned_string_vec(vec!["exe"])));
        let expected_command = Command::OutputRedirect(String::from("outputfile"), 
            Box::new(Command::InputRedirect(String::from("inputfile"), Box::new(token_command))));

        let actual_command = tokens::get_command(input);
        assert_eq!(expected_command, actual_command[0]);

    }

    #[test]
    fn redirect_test3() {

        let input = "exe > outputfile < inputfile ";

        let token_command = Command::Tokens(Box::new(into_owned_string_vec(vec!["exe"])));
        let expected_command = Command::OutputRedirect(String::from("outputfile"), 
            Box::new(Command::InputRedirect(String::from("inputfile"), Box::new(token_command))));

        let actual_command = tokens::get_command(input);
        assert_eq!(expected_command, actual_command[0]);

    }
}