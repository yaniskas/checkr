use std::{str::FromStr, collections::BTreeMap, io::{self, Write}};

use crate::{ast::{ParallelCommands, Target, Variable, Array}, model_checking::ModelCheckMemory};

pub fn ask_for<T: FromStr>(msg: &str, failmsg: &str) -> T {
    ask_for_with_parser(msg, failmsg, FromStr::from_str)
}

pub fn ask_for_with_parser<T, E>(msg: &str, failmsg: &str, parser: impl Fn(&str) -> Result<T, E>) -> T {
    loop {
        match parser(input_msg(msg).trim()) {
            Ok(val) => return val,
            Err(_) => {
                println!("{failmsg}");
                continue;
            }
        }
    }
}

pub fn ask_for_memory_assignment(commands: ParallelCommands) -> ModelCheckMemory {
    let assignment_targets = commands.fv();
    let mut var_map = BTreeMap::new();
    let mut arr_map = BTreeMap::new();

    for t in assignment_targets {
        match t {
            Target::Variable(wrapped_name) => {
                let Variable(name) = &wrapped_name;

                let desired_value = ask_for::<i64>(
                    &format!("Please input an initial assignment for variable {name}: "),
                    "Please enter a valid number"
                );

                var_map.insert(wrapped_name, desired_value);
            }
            Target::Array(wrapped_name, _) => {
                let Array(name) = &wrapped_name;

                let desired_value: Vec<i64> = loop {
                    let input = input_msg(&format!("Please input an initial assignment for array {name}, with values separated by commas: "));
                    match try_parse_array(&input) {
                        Ok(ass) => break ass,
                        Err(_) => {
                            println!("Please enter a valid array");
                            continue;
                        }
                    }
                };
                
                arr_map.insert(wrapped_name, desired_value);
            }
        }
    }

    ModelCheckMemory {
        variables: var_map,
        arrays: arr_map
    }
}

pub fn input() -> String {
    let mut str = String::new();
    io::stdin().read_line(&mut str).expect("Failed to read line");
    str
}

pub fn input_msg(message: &str) -> String {
    print!("{message}");
    io::stdout().flush().expect("Error flushing");
    input()
}

pub fn try_parse_array(string: &str) -> Result<Vec<i64>, ()> {
    let nums: Vec<Result<i64, _>> = string.split(",").map(|e| e.trim().parse()).collect();
    if nums.iter().all(|r| r.is_ok()) { Ok(nums.into_iter().map(|r| r.unwrap()).collect()) }
    else { Err(()) }
}