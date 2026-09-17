use std::env;

fn main() {
    let mut stdout = std::io::stdout().lock();
    let exit_code = match ams_gra_codegen_oms::run(env::args_os().skip(1), &mut stdout) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("error: {error}");
            error.exit_code()
        }
    };
    std::process::exit(exit_code);
}
