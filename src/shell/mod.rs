use std::env::Args;
use std::io;
use std::io::Write;
use std::ffi::{CStr, CString};
use libc::{O_CREAT, O_TRUNC, O_WRONLY};
use nix::{unistd::{fork, ForkResult, execvp}, sys::wait::wait};

mod tokens;

const SHELL_PROMPT : &str = "shell $ ";
const BUILT_INS : [&str; 2] = ["quit", "prev"];

struct Prev {
    strings: Vec<String>
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

        if BUILT_INS.contains(&user_input.to_ascii_lowercase().as_str()) {

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
// runs tokens through functions to find sequencing or output redirect
fn dispatch<'a>(tokens: &'a Vec<String>) {

    let meta_tokens: Vec<Vec<String>> = sequence(&tokens);
        for toks in meta_tokens.iter() {
            if toks[0].eq("cd") {
                run_cd(&toks);
                continue;
            }
            let (file_output, index) = filter_redirects(&toks);
            if file_output.is_empty() {
                run_command(&toks);
            }
            else {
                println!("redirect detected: {}", file_output);
                redirect_output_and_run(&file_output, &toks, index);
            }
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

fn run_cd<'a>(user_input: &'a Vec<String>) {

    std::env::set_current_dir(user_input[1].clone()).expect("failed to cd");
}

fn filter_redirects<'a>(user_input: &'a Vec<String>) -> (String, usize) {

    for (i, s) in user_input.iter().enumerate() {
        match s.as_str() {
            ">" => {
                if !user_input[i + 1].is_empty() {
                    return (user_input[i + 1].clone(), i);
                } 
            }
            _ => ()
        }
    }
    return (String::new(), 0);
}

fn redirect_output_and_run<'a>(dst: &String, user_input: &'a Vec<String>, index: usize) {  

    let (filename, filename_c, cstrs) = produce_c_strings(user_input);
    let execute_args = cstrs.get(0..index).unwrap().to_vec();

    let file: *const u8 = dst.as_ptr();

    let file = file as *const i8;

    match unsafe{fork()} {

        Ok(ForkResult::Child) => {
            unsafe {
                libc::close(1);
                libc::open(file, O_WRONLY | O_CREAT | O_TRUNC);
            }
            execute_within_child(filename, &filename_c, &execute_args);
        }
        Ok(ForkResult::Parent{child: _, ..}) => {
            wait().expect("Child process failed");
        }
        Err(e) => println!("fork failed with error: {}", e)
    }
}

fn convert_str_to_cstring<'a>(s: &'a str) -> CString{

    let cstring = CString::new(s).expect("failure converting str to CString");
    return CString::from(cstring);
}

fn map_strings_to_cstrings<'a>(user_input: &'a Vec<String>) -> Vec<CString> {

    return user_input.iter().map(|s| convert_str_to_cstring(s)).collect::<Vec<CString>>();
}


fn produce_c_strings<'a>(user_input: &'a Vec<String>) -> (String, CString, Vec<CString>) {

    let filename: String = user_input[0].clone();   
    let filename_c: CString = CString::new(filename.as_str()).expect("CString conversion failed");

    let cstrs: Vec<CString> = map_strings_to_cstrings(&user_input);

    return (filename, filename_c, cstrs);
}

fn run_command<'a>(user_input: &'a Vec<String>) {

    let (filename, filename_c, cstrs) = produce_c_strings(user_input);

    match unsafe{fork()} {

        Ok(ForkResult::Child) => {
            execute_within_child(filename, &filename_c, &cstrs);
        }
        Ok(ForkResult::Parent{child: _, ..}) => {
            wait().expect("Child process failed");
        }

        Err(e) => println!("fork failed with error: {}", e)
    }
}

fn execute_within_child<'a, 'b>(filename : String, filename_c : &'a CStr, cstrs: &'b Vec<CString>) {

    match execvp(filename_c, cstrs) {
        Ok(_) => (),
        Err(_) => {
            println!("Program not found: {}", filename);
        }
    }
}

#[cfg(test)]
mod run_tests {
    use super::*;

    #[test]
    fn test_convert_str_to_cstr() {

        let test_str = "testing";

        let converted_c_str = convert_str_to_cstring(test_str);

        let reconverted_str = converted_c_str.to_str().expect("failed reconversion");
        assert_eq!(test_str, reconverted_str);
    }

    #[test]
    fn test_convert_str_to_cstr2() {

        let test_str = "testing 1 2 3 \n";

        let converted_c_str = convert_str_to_cstring(test_str);

        let reconverted_str = converted_c_str.to_str().expect("failed reconversion");
        assert_eq!(test_str, reconverted_str);
    }

    #[test]
    fn test_map_to_cstrings() {

        let test_str_vec = vec!["testing", "1", "2", "3"];
        let test_string_vec = test_str_vec.iter().map(|&s|String::from(s)).collect::<Vec<String>>();        

        let result = map_strings_to_cstrings(&test_string_vec);

        let expected_result = vec![CString::new("testing").expect("fail"),
                                                CString::new("1").expect("fail"),
                                                CString::new("2").expect("fail"), 
                                                CString::new("3").expect("fail")];

        for i in 0..result.len() {
            assert_eq!(result[i], expected_result[i]);
        } 
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
mod output_redirect_tests {
    use super::*;

    #[test]
    fn filter_redirects_test() {

        let sample_tokens: Vec<String> = vec!["ex", "howdy", ">", "list"].iter().map(|&s| s.into()).collect();

        let expected_result = String::from("list");

        assert_eq!(filter_redirects(&sample_tokens).0, expected_result);
    }


    #[test]
    fn filter_redirects_test2() {

        let sample_tokens: Vec<String> = vec!["ex", ";", "howdy", ">", "list"]
        .iter().map(|&s| s.into()).collect();

        let expected_result = String::from("list");

        assert_eq!(filter_redirects(&sample_tokens).0, expected_result);
    }
}