extern crate lalrpop;

fn main() {
    let mut lalrpop_config = lalrpop::Configuration::new();
    lalrpop_config.use_cargo_dir_conventions();
    lalrpop_config.process_file("src/gcl.lalrpop").unwrap();
}
