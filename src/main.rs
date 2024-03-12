mod shell;

fn main() {

    let args = std::env::args();
    
    shell::run_shell(args);
    
}