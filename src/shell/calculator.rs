use std::io;
use std::io::Write;

use super::tokens;

const OPERANDS : [&str; 4] = ["+", "-", "*", "/"];

// 3 + 2 - 9 = (3 + 2) - 9

// tokens: vec(3, +, 2, -, 9)
// groupings: (3, +, 2), (_, -, 9)
// step 1: (5), (5, -, 9)

/**
 *     2 + 2 * 4
 *     /      \
 *    2 +    2 * 4
 *     |       |          
 *     2 +     8
 *      \     / 
 *         10   
 * 
 *       
 *      4 * 2 + 3 + 2 * 7
 * 
 *      (4 * 2) + 3 + (2 * 7)
 *         8    + 3 +   14
 * 
 * 
 *  PEMDAS hierarchy
 * 
 *  {} : ignore
 *  [] : ignore
 *  () : ignore
 *  ^    
 *  *, /                 
 *  +, -
 * 
 * for level v in hierarchy 
 * if operand exists
 * evaluate expression and replace
 * 
 * 
 * 
 *  
 */ 

enum Operands {
    Plus,
    Minus,
    Times,
    Divide,
}

enum Op<T> {
    Add,
    Sub,
    Div,
    Mul,
    Id(T)
}

type ChildNode<T> = Option<Box<BTNode<T>>>;

struct BTNode<T> {
    left: ChildNode<T>,
    op: Op<T>,
    right: ChildNode<T>,
}

struct BinaryTree<T> {
    head : Option<BTNode<T>>
}

impl BTNode<i32> {

    fn new(left: BTNode<i32>, op: Op<i32>, right: BTNode<i32>) -> Self {
        BTNode::<i32> {left: Some(Box::new(left)), op: op, right: Some(Box::new(right))}
    }

}

struct Grouping {
    op1: i32,
    operand: Operands,
    op2: i32,
}

trait GetResult {

    fn parse(op1 : i32, pontential_operand: &str, op2 : i32) -> Self;
    fn evaluate(g : Grouping) -> i32;

}

impl GetResult for Grouping {

    fn parse(op1 : i32, pontential_operand: &str, op2 : i32) -> Self {
        
        match pontential_operand {

            "+" => return Grouping {op1 : op1, operand: Operands::Plus, op2: op2},
            "-" => return Grouping {op1 : op1, operand: Operands::Minus, op2: op2},
            "*" => return Grouping {op1 : op1, operand: Operands::Times, op2: op2},
            "/" => return Grouping {op1 : op1, operand: Operands::Divide, op2: op2},
            _ => panic!("Unknown operand")
        }
    }

    fn evaluate(g : Grouping) -> i32 {
        
        match g.operand {
            Operands::Plus => return g.op1 + g.op2,
            Operands::Minus => return g.op1 - g.op2,
            Operands::Times => return g.op1 * g.op2,
            Operands::Divide => return g.op1 / g.op2,
        }
    }
}

pub fn start_calculator() {

    println!("Begin calculator mode");
    

    loop {

        print!("Calculator $ ");
        io::stdout().flush().unwrap();
        

        let mut user_input : String = String::new();

        io::stdin()
            .read_line(&mut user_input)
            .expect("Failed to read line");


        if user_input.to_lowercase() == "quit".to_owned() {

        }

        let tokens = tokens::get_tokens(user_input.clone());
        calculate(&tokens);

    }
}

pub fn calculate(input : &Vec<String>) {

    let mut results : Vec<i32> = Vec::new();

    for (i, token) in input.iter().enumerate() {

        if OPERANDS.contains(&token.clone().as_str()) {
            let mut op1: i32 = 0;

            if i != 0 {
                op1 = input[i-1].clone().parse().expect("op1 not a number");
            }

            let op2: i32 = input[i+1].clone().parse().expect("op1 not a number");

            let g = Grouping::parse(op1, &token.clone().as_str(), op2);

            results.push(Grouping::evaluate(g));

        }

        // if token is a number, save as potential operand
    }

    println!();
}
