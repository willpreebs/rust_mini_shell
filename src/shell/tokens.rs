use std::str::Chars;

use super::{output, Command};

fn sequence(tokens: Vec<String>) -> Vec<Command> {

    if tokens.len() == 0 {
        return vec![Command::Empty];
    }

    let r: Vec<&[String]> = tokens.split(|t| t.eq(";")).collect();

    if r.len() == 1 {
        let v = r[0].to_vec();
        return vec![split_on_redirection(v)];
    } else {
        let mut c_vec = Vec::new();
        for command_arr in tokens.split(|token| token.eq(";")) {
            let v = command_arr.to_vec();
            if !v.is_empty() {
                let a = split_on_redirection(v);
                c_vec.push(a);
            }
        }
    
        return c_vec;
    }
}

fn split_on_redirection(v: Vec<String>) -> Command {

    let split_output: Vec<&[String]> = v.split(|e| e.eq(">")).collect();
    if split_output.len() == 1 {
        return split_on_input(v);
    }
    else if split_output.len() != 2 {
        output("Output redirection failed, cannot redirect output more than once", true);
        return Command::Empty;
    }
    else if split_output[0].is_empty() {
        output("Output redirection failed, command is required prior to <", true);
        return Command::Empty;
    }
    else if split_output[1].is_empty() {
        output("Output redirection failed, must include filename to get send output", true);
        return Command::Empty;
    }
    else {
        let mut command: Vec<String> = split_output[0].to_vec();
        let mut command_snd: Vec<String> = split_output[1].iter().skip(1).map(|b| b.to_owned()).collect();
        command.append(&mut command_snd);
        let dst = split_output[1][0].clone();
        return Command::OutputRedirect(dst, Box::new(split_on_input(command)));
    }
}

fn split_on_input(v: Vec<String>) -> Command {
    // split_input = {[tee, file1], [example.txt, >, file2]
    let split_input: Vec<&[String]> = v.split(|e| e.eq("<")).collect();

    if split_input.len() == 1 {
        // no input redirect
        return Command::Tokens(Box::new(v));
    }
        
    else if split_input.len() != 2 {
        output("Input redirection failed, cannot redirect input more than once", true);
        return Command::Empty;
    }

    else if split_input[0].is_empty() {
        output("Input redirection failed, command is required prior to <", true);
        return Command::Empty;
    }
    else if split_input[1].is_empty() {
        output("Input redirection failed, must include filename to get input from", true);
        return Command::Empty;
    }
    else {
        return Command::InputRedirect(split_input[1][0].clone(), Box::new(Command::Tokens(Box::new(split_input[0].to_vec()))));
    }

}

pub fn get_command(input: &str) -> Vec<Command> {
    let vec = tokenize(input);
    return sequence(vec);
}

pub fn tokenize(input : &str) -> Vec<String> {

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


#[cfg(test)]
mod token_tests{
    use super::*;

    #[test]
    fn test_with_1_token() {

        let slice = "example_input";

        // let input : String = String::from(slice);
        let result = vec![String::from(slice)];

        assert_eq!(tokenize(slice), result);
        
    }

    #[test]
    fn test_with_2_tokens() {

        let slice = "input1 input2";

        let result = vec![String::from("input1"), String::from("input2")];

        assert_eq!(tokenize(slice), result);
        
    }

    #[test]
    fn test_with_2_tokens_and_end_space() {

        let slice = "input1 input2 ";

        let result = vec![String::from("input1"), String::from("input2")];

        assert_eq!(tokenize(slice), result);
        
    }

    #[test]
    fn test_with_special_character_tokens() {

        let slice = "input1 < input2";

        let result = vec![String::from("input1"), String::from("<"), String::from("input2")];

        assert_eq!(tokenize(slice), result);
        
    }


    #[test]
    fn test_with_quotation_tokens() {

        let slice = r#"input1 "filename" input2 "#;
        let result = vec![String::from("input1"), String::from("filename"), String::from("input2")];

        assert_eq!(tokenize(slice), result);

    }

    #[test]
    fn test_with_quotation_tokens2() {

        let slice = r#"input1 "file < ... name" input2 "#;
        let result = vec![String::from("input1"), String::from("file < ... name"), String::from("input2")];

        assert_eq!(tokenize(slice), result);

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
        let result = vec![String::from("input1"), 
                                        String::from("<"), 
                                        String::from(">"), 
                                        String::from(">"), 
                                        String::from("input2")];

        assert_eq!(tokenize(slice), result);
        
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