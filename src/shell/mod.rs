use std::env::Args;
use std::io::{self, BufRead};
use std::io::Write;
use std::ffi::{CStr, CString};
use libc::{O_CREAT, O_TRUNC, O_WRONLY, O_RDONLY, exit};
use nix::{unistd::{fork, ForkResult, execvp}, sys::wait::wait};
use std::fs::File;

mod tokens;

const SHELL_PROMPT : &str = "shell $ ";

// built-in commands with no additional arguments
const BUILT_IN_SINGLE_ARG : [&str; 2] = ["quit", "prev"];

// built-in commands with possible additional arguments
const BUILT_IN_MULT_ARGS : [&str; 2] = ["cd", "source"];

struct Prev {
    strings: Vec<String>
}

#[derive(PartialEq)]
#[derive(Debug)]
struct Redirect {
    input: bool,
    output: bool,
    src: String,
    dst: String,
    command_tokens: Vec<String>
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

pub fn run_shell(_args : Args) {

    let mut prev : Prev = Prev{strings: Vec::new()};

    loop {

        print!("{}", SHELL_PROMPT);
        io::stdout().flush().unwrap();

        let mut user_input : String = String::new();

        io::stdin()
            .read_line(&mut user_input)
            .expect("Failed to read line");

        let user_input = String::from(user_input.trim());
        let tokens: Vec<String> = tokens::get_tokens(user_input.clone()); 

        if BUILT_IN_SINGLE_ARG.contains(&user_input.to_ascii_lowercase().as_str()) {

            match user_input.to_ascii_lowercase().as_str() {
                "quit" => {
                    println!("Goodbye");
                    break;
                },
                "prev" => {
                    if prev.strings.is_empty() {
                        continue;
                    }
                    dispatch(&prev.strings);
                    continue;
                }
                _ => ()
            }
        }
        else {
            prev = Prev{strings: tokens.clone()}
        };

        dispatch(&tokens);
    }
}
// seperates sequences of tokens and runs commands based on redirections
fn dispatch<'a>(tokens: &'a Vec<String>) {

    let meta_tokens: Vec<Vec<String>> = sequence(&tokens);
    for toks in meta_tokens.iter() {

        let red: Redirect = Redirect::detect(&toks);
        handle_redirects(red);
    }
}

fn handle_redirects<'a>(red: Redirect) {

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
                        println!("Error closing stdin");
                        exit(0);
                    }

                    let open_result = libc::open(input_file, O_RDONLY);
                    if open_result == -1 {
                        println!("File not found: {}", &red.src);
                        exit(0);
                    }
                }
                if red.output {
                    let close_result = libc::close(1);
                    if close_result == -1 {
                        println!("Error closing stdout");
                        exit(0);
                    }

                    let open_result = libc::open(output_file, O_WRONLY | O_CREAT | O_TRUNC);
                    if open_result == -1 {
                        println!("File not found: {}", &red.dst);
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
        Err(e) => println!("fork failed with error: {}", e)
    }

}

fn sequence<'a>(tokens: &'a Vec<String>) -> Vec<Vec<String>> {

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

fn execute_within_child<'a, 'b>(filename : String, filename_c : &'a CStr, cstrs: &'b Vec<CString>) {

    match execvp(filename_c, cstrs) {
        Ok(_) => (),
        Err(_) => {
            println!("Program not found: {}", filename);
        }
    }
}

fn execute_cd<'a>(path : &'a String) {
    match std::env::set_current_dir(path) {
        Ok(_) => (),
        Err(e) => println!("cd failed with error: {}. Path was {}", e, path)
    }
}

fn execute_source<'a>(command_tokens : &Vec<String>) {

    let mut commands : Vec<Vec<String>> = Vec::new(); 

    let filename = command_tokens[1].clone();
    let file_result = File::open(&filename);
    let file: File;

    match file_result {
        Ok(t) => file = t,
        Err(e) => {
            println!("Error: {} in finding file: {}", e, &filename.clone());
            return;
        }
    }
    let buf_reader = io::BufReader::new(file);

    for line in buf_reader.lines() {
        match line {
            Ok(s) => {
                let tokens = tokens::get_tokens(s);
                commands.push(tokens);
            }
            Err(_) => println!("Error reading line")
        }
    }

    for command in commands.iter() {
        dispatch(command);
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