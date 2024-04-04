use std::env::Args;
use std::io::{self, BufRead};
use std::io::Write;
use std::ffi::{CStr, CString};
use libc::{O_CREAT, O_TRUNC, O_WRONLY, O_RDONLY};
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
            unsafe {
                match fork() {
                    Ok(ForkResult::Child) => {
                        // close stdin
                        let close_result = libc::close(0);
                        if close_result == -1 {
                            output("Error closing stdin", true);
                        }

                        // redirect input to src file
                        let open_result = libc::open(src.as_ptr() as *const i8, O_RDONLY);
                        if open_result == -1 {
                            output(format!("File not found: {}", src).as_str(), true);
                        }
                        dispatch_command(c);
                        exit(0);
                    },
                    _ =>{},
                }   
            }
        },
        Command::OutputRedirect(dst, c) => {
            unsafe {
                match fork() {
                    Ok(ForkResult::Child) => {
                        // close stdout
                        let close_result = libc::close(1);
                        if close_result == -1 {
                            output("Error closing stdin", true);
                        }

                        // redirect output to dst file
                        let open_result = libc::open(dst.as_ptr() as *const i8, O_WRONLY | O_CREAT | O_TRUNC);
                        if open_result == -1 {
                            output(format!("Problem with creating or writing to: {}", dst).as_str(), true);
                        }
                        dispatch_command(c);
                        exit(0);
                    },
                    _ =>{},
                }
            }
        }
    }
}

fn convert_string_to_cstring(s: &String) -> CString {
    return CString::new(s.as_str()).unwrap();
}

// fn handle_redirects(red: Redirect) {

//     let filename: String = red.command_tokens[0].clone();   
//     let filename_c: CString = CString::new(filename.as_str()).expect("CString conversion failed");

//     let cstrs: Vec<CString> = red.command_tokens.iter()
//     .map(|s| CString::new(s as &str).expect("failure converting str to CString")).collect::<Vec<CString>>();

//     let output_file = red.dst.trim().as_ptr() as *const i8;
//     let input_file = red.src.trim().as_ptr() as *const i8;

//     if BUILT_IN_MULT_ARGS.contains(&red.command_tokens[0].as_str()) && red.command_tokens.len() > 1 {

//         match red.command_tokens[0].clone().as_str() {
//             "cd" => execute_cd(&red.command_tokens[1].clone()),
//             "source" => execute_source(&red.command_tokens),
//             _ => ()
//         }
//         return;
//     }
    
//     match unsafe{fork()} {

//         Ok(ForkResult::Child) => {

//             unsafe {
//                 //bit of C trickery to replace stdin or stdout with a new file
//                 if red.input {
//                     let close_result = libc::close(0);
//                     if close_result == -1 {
//                         output("Error closing stdin", true);
//                         exit(0);
//                     }

//                     let open_result = libc::open(input_file, O_RDONLY);
//                     if open_result == -1 {
//                         output(format!("File not found: {}", &red.src).as_str(), true);
//                         exit(0);
//                     }
//                 }
//                 if red.output {
//                     let close_result = libc::close(1);
//                     if close_result == -1 {
//                         output("Error closing stdout", true);
//                         exit(0);
//                     }

//                     let open_result = libc::open(output_file, O_WRONLY | O_CREAT | O_TRUNC);
//                     if open_result == -1 {
//                         output(format!("File not found: {}", &red.dst).as_str(), true);
//                         exit(0);
//                     }
//                 }
//             }
//             execute_within_child(filename, &filename_c, &cstrs);
//             unsafe{exit(0)}
//         }
//         Ok(ForkResult::Parent{child: _, ..}) => {
//             wait().expect("Child process failed");
//         }
//         Err(e) => output(format!("fork failed with error: {}", e).as_str(), true)
//     }

// }

// fn sequence(tokens: &Vec<String>) -> Vec<Vec<String>> {

//     let mut v2: Vec<Vec<String>> = Vec::new();
//     let mut v: Vec<String> = Vec::new();

//     for s in tokens.iter() {
        
//         match s.as_str() {
//             ";" => {
//                 v2.push(v.clone());
//                 v = Vec::new();
//             }
//             _ => v.push(s.to_string())
//         }
//     }
//     if !v.is_empty() {
//         v2.push(v);
//     }

//     return v2;
// }

fn execute_within_child(filename: String, filename_c: &CStr, cstrs: &Vec<CString>) {

    match execvp(filename_c, cstrs) {
        Ok(_) => (),
        Err(_) => {
            output(format!("Program not found: {}", filename).as_str(), true);
        }
    }
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

// #[cfg(test)]
// mod redirect_tests {
//     use super::*;

//     #[test]
//     fn redirect_test1() {

//         let sample_tokens: Vec<String> = vec!["ex", "howdy", ">", "list"].iter().map(|&s| s.into()).collect();

//         let input = false;
//         let output = true;
//         let src = String::new();
//         let dst = String::from("list");
//         let command_tokens = vec![String::from("ex"), String::from("howdy")];
//         let expected_result = Redirect{input, output, src, dst, command_tokens};

//         assert_eq!(Redirect::detect(&sample_tokens), expected_result);
//     }


    
//     #[test]
//     fn redirect_test2() {

//         let sample_tokens: Vec<String> = vec!["ex", "howdy", "<", "in", ">", "out"].iter().map(|&s| s.into()).collect();

//         let input = true;
//         let output = true;
//         let src = String::from("in");
//         let dst = String::from("out");
//         let command_tokens = vec![String::from("ex"), String::from("howdy")];
//         let expected_result = Redirect{input, output, src, dst, command_tokens};

//         assert_eq!(Redirect::detect(&sample_tokens), expected_result);
//     }



//     #[test]
//     fn redirect_test3() {

//         let sample_tokens: Vec<String> = vec!["ex", "howdy", "<", "in", "in2", ">", "out"].iter().map(|&s| s.into()).collect();

//         let input = true;
//         let output = true;
//         let src = String::from("in");
//         let dst = String::from("out");
//         let command_tokens = vec![String::from("ex"), String::from("howdy"), String::from("in2")];
//         let expected_result = Redirect{input, output, src, dst, command_tokens};

//         assert_eq!(Redirect::detect(&sample_tokens), expected_result);
//     }
// }