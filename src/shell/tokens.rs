use std::str::Chars;

use super::Command;

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

fn convert_vec_to_command(vec: Vec<String>) -> Command {
    return Command::Tokens(Box::new(vec));
}

pub fn get_command(input: &str) -> Command {
    let vec = get_tokens(input);

    let seq = sequence(&vec);
    if seq.len() == 1 {
        return Command::Tokens(Box::new(seq.get(0).unwrap().to_vec()));
    } 
    else {
        let mut com_list = Vec::new();
        for sub in seq {
            com_list.push(convert_vec_to_command(sub));
        }
        return Command::Commands(Box::new(com_list)); 
    }
}

pub fn get_tokens(input : &str) -> Vec<String> {

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

        assert_eq!(get_tokens(slice), result);
        
    }

    #[test]
    fn test_with_2_tokens() {

        let slice = "input1 input2";

        let result = vec![String::from("input1"), String::from("input2")];

        assert_eq!(get_tokens(slice), result);
        
    }

    #[test]
    fn test_with_2_tokens_and_end_space() {

        let slice = "input1 input2 ";

        let result = vec![String::from("input1"), String::from("input2")];

        assert_eq!(get_tokens(slice), result);
        
    }

    #[test]
    fn test_with_special_character_tokens() {

        let slice = "input1 < input2";

        let result = vec![String::from("input1"), String::from("<"), String::from("input2")];

        assert_eq!(get_tokens(slice), result);
        
    }


    #[test]
    fn test_with_quotation_tokens() {

        let slice = r#"input1 "filename" input2 "#;
        let result = vec![String::from("input1"), String::from("filename"), String::from("input2")];

        assert_eq!(get_tokens(slice), result);

    }

    #[test]
    fn test_with_quotation_tokens2() {

        let slice = r#"input1 "file < ... name" input2 "#;
        let result = vec![String::from("input1"), String::from("file < ... name"), String::from("input2")];

        assert_eq!(get_tokens(slice), result);

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

        assert_eq!(get_tokens(slice), result);
        
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