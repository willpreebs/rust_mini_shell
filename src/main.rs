mod shell;

fn main() {

    let args = std::env::args();

    println!("Welcome to the rust minishell");

    shell::run_shell(args);

}

