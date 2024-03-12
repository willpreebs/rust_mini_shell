use std::env::Args;
use std::io::{self, BufRead};
use std::io::Write;
use std::ffi::{CStr, CString};
use libc::{O_CREAT, O_TRUNC, O_WRONLY, O_RDONLY, exit};
use nix::{unistd::{fork, ForkResult, execvp}, sys::wait::wait};
use std::fs::File;

mod scanner;
mod tokens;

const SHELL_PROMPT : &str = "shell $ ";

// built-in commands with no additional arguments
const BUILT_IN_SINGLE_ARG : [&str; 3] = ["quit", "prev", "help"];

// built-in commands with possible additional arguments
const BUILT_IN_MULT_ARGS : [&str; 2] = ["cd", "source"];

// struct Prev {
//     strings: Vec<String>
// }

#[derive(PartialEq)]
#[derive(Debug)]
struct Redirect {
    input: bool,
    output: bool,
    src: String,
    dst: String,
    command_tokens: Vec<String>
}

pub enum Command {
    Tokens(Box<Vec<String>>),
    Commands(Box<Vec<Command>>)
}

trait Detect {
    fn detect<'a>(user_input: &'a Vec<String>) -> Self;
}

impl Detect for Redirect {

    fn detect<'a>(user_input: &'a Vec<String>) -> Redirect {

        let mut input = false;
        let mut output = false;
        let mut src = String::new();
        let mut dst = String::new();
        let mut command_tokens: Vec<String> = Vec::new();
        
        let mut skip = 0;
        
        for (i, s) in user_input.iter().enumerate() {
            match s.as_str() {
                ">" => {
                    if !user_input[i + 1].is_empty() {
                        output = true;
                        dst = user_input[i + 1].clone();
                        skip = 1;
                    } 
                }
                "<" => {
                    if !user_input[i + 1].is_empty() {
                        input = true;
                        src = user_input[i + 1].clone();
                        skip = 1;
                    }
                }
                _ => {
                    if skip == 0 {
                        command_tokens.push(String::from(s));
                    }
                    else {
                        skip -= 1;
                    }
                    
                }
            }
        }

        if !(input || output) {
            command_tokens = user_input.clone();
        }

        return Redirect {input, output, src, dst, command_tokens};
    }
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

    let mut prev : Box<Vec<String>> = Box::new(Vec::new());

    loop {

        output(SHELL_PROMPT, false);
        io::stdout().flush().unwrap();

        let mut user_input : String = String::new();

        io::stdin()
            .read_line(&mut user_input)
            .expect("Failed to read line");

        let user_input = user_input.trim();

        
        let tokens: Vec<String> = tokens::get_tokens(user_input); 
        let command: Command = tokens::get_command(user_input);

        if is_single_arg_built_in(user_input) {
            let behavior = handle_single_arg_built_in(user_input, &prev);
            match behavior {
                LoopBehavior::BREAK => break,
                LoopBehavior::CONTINUE => continue,
                LoopBehavior::SKIP => ()
            }
        }
        else {
            prev = Box::new(tokens.clone());
        }

        // dispatch(&tokens);
        dispatch_command(&command);
    }
}

fn is_single_arg_built_in(input: &str) -> bool {
    return BUILT_IN_SINGLE_ARG.contains(&input.to_ascii_lowercase().as_str());
}

enum LoopBehavior {
    BREAK,
    CONTINUE,
    SKIP
}

fn handle_single_arg_built_in(input: &str, prev: &Box<Vec<String>>) -> LoopBehavior {
    match input.to_ascii_lowercase().as_str() {
        "quit" => {
            output("Goodbye", true);
            return LoopBehavior::BREAK;
        },
        "prev" => {
            if prev.is_empty() {
                return LoopBehavior::CONTINUE;
            }
            dispatch(prev);
            return LoopBehavior::CONTINUE;
        },
        "help" => {
            output(get_help_message().as_str(), true);
            return LoopBehavior::CONTINUE;
        }
        _ => return LoopBehavior::SKIP
    }
}
// seperates sequences of tokens and runs commands based on redirections
fn dispatch(tokens: &Vec<String>) {

    let meta_tokens: Vec<Vec<String>> = sequence(&tokens);

    for toks in meta_tokens.iter() {

        let red: Redirect = Redirect::detect(&toks);
        handle_redirects(red);
    }
}

fn dispatch_command(command: &Command) {

    match command {
        Command::Tokens(boxed_vec) => {
            let red: Redirect = Redirect::detect(&boxed_vec);
            handle_redirects(red);
        }
        Command::Commands(boxed_vec) => {
            for c in boxed_vec.iter() {
                dispatch_command(c);
            }
        }
    }
}

fn handle_redirects(red: Redirect) {

    let filename: String = red.command_tokens[0].clone();   
    let filename_c: CString = CString::new(filename.as_str()).expect("CString conversion failed");

    let cstrs: Vec<CString> = red.command_tokens.iter()
    .map(|s| CString::new(s as &str).expect("failure converting str to CString")).collect::<Vec<CString>>();

    let output_file = red.dst.trim().as_ptr() as *const i8;
    let input_file = red.src.trim().as_ptr() as *const i8;

    if BUILT_IN_MULT_ARGS.contains(&red.command_tokens[0].as_str()) && red.command_tokens.len() > 1 {

        match red.command_tokens[0].clone().as_str() {
            "cd" => execute_cd(&red.command_tokens[1].clone()),
            "source" => execute_source(&red.command_tokens),
            _ => ()
        }
        return;
    }
    
    match unsafe{fork()} {

        Ok(ForkResult::Child) => {

            unsafe {
                //bit of C trickery to replace stdin or stdout with a new file
                if red.input {
                    let close_result = libc::close(0);
                    if close_result == -1 {
                        output("Error closing stdin", true);
                        exit(0);
                    }

                    let open_result = libc::open(input_file, O_RDONLY);
                    if open_result == -1 {
                        output(format!("File not found: {}", &red.src).as_str(), true);
                        exit(0);
                    }
                }
                if red.output {
                    let close_result = libc::close(1);
                    if close_result == -1 {
                        output("Error closing stdout", true);
                        exit(0);
                    }

                    let open_result = libc::open(output_file, O_WRONLY | O_CREAT | O_TRUNC);
                    if open_result == -1 {
                        output(format!("File not found: {}", &red.dst).as_str(), true);
                        exit(0);
                    }
                }
            }
            execute_within_child(filename, &filename_c, &cstrs);
            unsafe{exit(0)}
        }
        Ok(ForkResult::Parent{child: _, ..}) => {
            wait().expect("Child process failed");
        }
        Err(e) => output(format!("fork failed with error: {}", e).as_str(), true)
    }

}

fn sequence(tokens: &Vec<String>) -> Vec<Vec<String>> {

    let mut v2: Vec<Vec<String>> = Vec::new();
    let mut v: Vec<String> = Vec::new();

    for s in tokens.iter() {
        
        match s.as_str() {
            ";" => {
                v2.push(v.clone());
                v = Vec::new();
            }
            _ => v.push(s.to_string())
        }
    }
    if !v.is_empty() {
        v2.push(v);
    }

    return v2;
}

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
                let command: Command = tokens::get_command(s.as_str());
                let tokens = tokens::get_tokens(s.as_str());
                commands_vec.push(command);
            }
            Err(_) => output("Error reading line", true)
        }
    }

    if commands_vec.len() == 1 {
        dispatch_command(&commands_vec[0]);
    }
    else {
        let c = Command::Commands(Box::new(commands_vec));
        dispatch_command(&c);
    }
}

#[cfg(test)]
mod sequence_tests {
    use super::*;

    #[test]
    fn sequence_test1() {
        let sample_tokens: Vec<String> = ["ex", "token", "list"].map(String::from).to_vec();
        let expected_meta_tokens: Vec<Vec<String>> = vec![["ex", "token", "list"].map(String::from).to_vec()];

        assert_eq!(sequence(&sample_tokens), expected_meta_tokens);
    }

    #[test]
    fn sequence_test2() {
        let sample_tokens: Vec<String> = ["ex", ";", "list"].map(String::from).to_vec();

        let expected_meta_tokens: Vec<Vec<String>> = vec![vec![String::from("ex")], vec![String::from("list")]];

        assert_eq!(sequence(&sample_tokens), expected_meta_tokens);
    }


    #[test]
    fn sequence_test3() {
        let sample_tokens: Vec<String> = vec!["ex", "howdy", ";", "list"].iter().map(|&s| s.into()).collect();

        let expected_meta_tokens: Vec<Vec<String>> = vec![vec![String::from("ex"), String::from("howdy")], vec![String::from("list")]];

        assert_eq!(sequence(&sample_tokens), expected_meta_tokens);
    }
}

#[cfg(test)]
mod redirect_tests {
    use super::*;

    #[test]
    fn redirect_test1() {

        let sample_tokens: Vec<String> = vec!["ex", "howdy", ">", "list"].iter().map(|&s| s.into()).collect();

        let input = false;
        let output = true;
        let src = String::new();
        let dst = String::from("list");
        let command_tokens = vec![String::from("ex"), String::from("howdy")];
        let expected_result = Redirect{input, output, src, dst, command_tokens};

        assert_eq!(Redirect::detect(&sample_tokens), expected_result);
    }


    
    #[test]
    fn redirect_test2() {

        let sample_tokens: Vec<String> = vec!["ex", "howdy", "<", "in", ">", "out"].iter().map(|&s| s.into()).collect();

        let input = true;
        let output = true;
        let src = String::from("in");
        let dst = String::from("out");
        let command_tokens = vec![String::from("ex"), String::from("howdy")];
        let expected_result = Redirect{input, output, src, dst, command_tokens};

        assert_eq!(Redirect::detect(&sample_tokens), expected_result);
    }



    #[test]
    fn redirect_test3() {

        let sample_tokens: Vec<String> = vec!["ex", "howdy", "<", "in", "in2", ">", "out"].iter().map(|&s| s.into()).collect();

        let input = true;
        let output = true;
        let src = String::from("in");
        let dst = String::from("out");
        let command_tokens = vec![String::from("ex"), String::from("howdy"), String::from("in2")];
        let expected_result = Redirect{input, output, src, dst, command_tokens};

        assert_eq!(Redirect::detect(&sample_tokens), expected_result);
    }
}