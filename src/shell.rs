use std::env::Args;
use std::io;
use std::io::Write;
use std::str::Chars;
use std::ffi::CStr;
use std::ffi::CString;

use nix::{unistd::{fork, ForkResult, execvp}, sys::wait::wait};
const SHELL_PROMPT : &str = "shell $ ";
const BUILT_INS : [&str; 2] = ["quit", "prev"];
//const MAX_LENGTH : usize = 256;

struct Prev {
    //line: String,
    strings: Vec<String>
}

fn get_tokens(input : String) -> Vec<String> {

    let iter = input.chars();
    let mut tokens: Vec<String> = Vec::new();
    let mut buf : Vec<char> = Vec::new();
    let mut quote_escape: i32 = 0;

    for (s, c) in iter.clone().enumerate() {

        if quote_escape > 0 {
            quote_escape -= 1;
            continue;
        }

        match c {
            ' '|'\n'|'\t' => {
                if buf.len() > 0 {  
                    add_token(&mut tokens, buf);
                    buf = Vec::new();
                }
            }
            '>'|'<'|'|'|';' => {
                if buf.len() > 0 {
                    add_token(&mut tokens, buf);
                    buf = Vec::new();
                }

                buf.push(c);
                add_token(&mut tokens, buf);
                buf = Vec::new();
            }
            '"' => {
                
                if buf.len() > 0 {
                    add_token(&mut tokens, buf);
                    buf = Vec::new();
                }
                // read_string will update buf with the chars within the quotes
                // and also initialize quote_escape to the length of the chars between the quotes
                quote_escape = read_string(s + 1, input.chars(), &mut buf) + 1;
                add_token(&mut tokens, buf);
                buf = Vec::new();
            } 
            _ => {
                buf.push(c);
            }
        }
    }

    if buf.len() > 0 {
    add_token(&mut tokens, buf);
    }

    return tokens;
}

fn add_token(tokens : &mut Vec<String>, buf : Vec<char>) {
    tokens.push(buf.iter().collect::<String>());
}

/**
 * Inputs
 * index: usize - the index where the quote starts
 * iter: Chars - the input line as an iterable
 * buf: &mut Vec<char> - where the string within the quote will go
 * Outputs
 * length 
 */ 
fn read_string(index : usize, iter: Chars, buf: &mut Vec<char>) -> i32 {

    let after_quote = iter.skip(index);
    for c in after_quote {

        match c {

            '"' => {
                break;
            }
            _ => {
                buf.push(c);
                //length += 1;
            }
        }
    }

    assert!(buf.len() != 0);
    return buf.len() as i32;
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
        let tokens: Vec<String> = get_tokens(user_input.clone()); 

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
                    run_command(&prev.strings);
                    continue;
                }
                _ => ()
            }
        }
        else {
            prev = Prev{strings: tokens.clone()}
        };

        if tokens[0].eq("cd") {
            run_cd(&tokens);
            continue;
        }

        let meta_tokens = sequence(&tokens);
        for toks in meta_tokens.iter() {
            run_command(&toks);
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

fn convert_str_to_cstring<'a>(s: &'a str) -> CString{

    let cstring = CString::new(s).expect("failure converting str to CString");
    return CString::from(cstring);
}

fn map_strings_to_cstrings<'a>(user_input: &'a Vec<String>) -> Vec<CString> {

    return user_input.iter().map(|s| convert_str_to_cstring(s)).collect::<Vec<CString>>();

}


fn run_command<'a>(user_input: &'a Vec<String>) {

    let filename: String = user_input[0].clone();   
    let filename_c: CString = CString::new(filename.as_str()).expect("CString conversion failed");
    let filename_c: &CStr = &filename_c;

    let cstrs: Vec<CString> = map_strings_to_cstrings(&user_input);
    
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
mod token_tests{
    use super::*;

    #[test]
    fn test_with_1_token() {

        let slice = "example_input";

        let input : String = String::from(slice);
        let result = vec![String::from(slice)];

        assert_eq!(get_tokens(input), result);
        
    }

    #[test]
    fn test_with_2_tokens() {

        let slice = "input1 input2";

        let input : String = String::from(slice);
        let result = vec![String::from("input1"), String::from("input2")];

        assert_eq!(get_tokens(input), result);
        
    }

    #[test]
    fn test_with_2_tokens_and_end_space() {

        let slice = "input1 input2 ";

        let input : String = String::from(slice);
        let result = vec![String::from("input1"), String::from("input2")];

        assert_eq!(get_tokens(input), result);
        
    }

    #[test]
    fn test_with_special_character_tokens() {

        let slice = "input1 < input2";

        let input : String = String::from(slice);
        let result = vec![String::from("input1"), String::from("<"), String::from("input2")];

        assert_eq!(get_tokens(input), result);
        
    }


    #[test]
    fn test_with_quotation_tokens() {

        let slice = r#"input1 "filename" input2 "#;

        let input : String = String::from(slice);

        let result = vec![String::from("input1"), String::from("filename"), String::from("input2")];

        assert_eq!(get_tokens(input), result);

    }

    #[test]
    fn test_with_quotation_tokens2() {

        let slice = r#"input1 "file < ... name" input2 "#;

        let input : String = String::from(slice);

        let result = vec![String::from("input1"), String::from("file < ... name"), String::from("input2")];

        assert_eq!(get_tokens(input), result);

    }

    #[test]
    fn read_string_test() {

        // fn read_string(index : usize, iter: Chars, buf: &mut Vec<char>)

        let slice = r#"input1 "file < name" input2 "#;


        let input : String = String::from(slice);

        let iter: Chars = input.chars();
        let index : usize = 8;
        let mut buf: Vec<char> = Vec::new();

        //let length = read_string(index, iter, &mut buf);

        let expected_buf: Vec<char> = vec!['f', 'i', 'l', 'e', ' ', '<', ' ', 'n', 'a', 'm', 'e'];
        let expected_length = 11; 

        //let result = vec![String::from("input1"), String::from("file < ... name"), String::from("input2")];

        assert_eq!(read_string(index, iter, &mut buf), expected_length);
        assert_eq!(buf, expected_buf);
    }


    #[test]
    fn test_with_special_character_tokens2() {

        let slice = "input1< >>input2";

        let input : String = String::from(slice);
        let result = vec![String::from("input1"), 
                                        String::from("<"), 
                                        String::from(">"), 
                                        String::from(">"), 
                                        String::from("input2")];

        assert_eq!(get_tokens(input), result);
        
    }

    #[test]
    fn add_token_test() {

        let mut tokens : Vec<String> = vec![String::from("tokenAlreadyHere")];
        let buf : Vec<char> = vec!['e', 'x', 'a', 'm', 'p', 'l', 'e'];
        let expected_tokens = vec![String::from("tokenAlreadyHere"), String::from("example")];
        
        add_token(&mut tokens, buf);
        assert_eq!(tokens, expected_tokens);
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

