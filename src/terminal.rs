use crate::println;
use crate::print;
use crate::get_words;
use crate::clear_screen;
use crate::set_color;
use crate::update_color;
use crate::reset_command_offset;
use crate::xhci::XHCI_DRIVER;

use alloc::{vec::Vec, string::String};

struct Command {
    name: &'static str,
    function: fn(Vec<String>),
    description: &'static str,
}

static COMMANDS: &[Command] = &[
    Command {
        name: "",
        function: help,
        description: "Every command must start with $."
    },
    Command {
        name: "help",
        function: help,
        description: "It tells you the name of the commands and a short description."
    },
    Command {
        name: "echo",
        function: echo,
        description: "It will print anything you put in. Args will be separated by a ' ' when   displaying it"
    },
    Command {
        name: "clear",
        function: clear,
        description: "It clears the screen"
    },
    Command {
        name: "set_color",
        function: change_color,
        description: "It changes the color of the text and or background"
    },
    Command {
        name: "update_color",
        function: update_color,
        description: "it changes all of the text colors to be the one you are currently using"
    },
    Command {
        name: "xhci_log",
        function: xhci_logs,
        description: "It shows xHCI's logs"
    },
    Command {
        name: "xhci_log_cap_register",
        function: xhci_cap_logs,
        description: "It shows xHCI's log capability registers"
    },
    Command {
        name: "xhci_log_op_register",
        function: xhci_op_logs,
        description: "It shows xHCI's log operational registers"
    },
];

pub fn command_runner(){
    let mut cmd = get_words!();


    if cmd.is_empty() {
        return;
    }

    let first_char = cmd[0].chars().nth(0).unwrap();

    if first_char != '$' {
        return;
    }

    if cmd.len() == 1 && cmd[0].len() == 1 { 
        println!("> Need to specify command");
        return;
    } 

    if first_char == '$' && cmd[0].len() != 1 {
        cmd[0] = cmd[0].chars().skip(1).collect::<String>();
    } else {
        cmd.remove(0);
    };

    for cmd_entry in COMMANDS.iter() {
        if cmd_entry.name == cmd[0].to_lowercase() {
            (cmd_entry.function)(cmd);
            reset_command_offset!();
            return;
        }
    }
    println!("> Command not found");
}

fn echo(cmd: Vec<String>) {
    print!("> ");

    for col in 1..=cmd.len() - 1 {
        print!("{} ", cmd[col]);
    }
    println!();
}

fn help(_cmd : Vec<String>) {
    println!();
    for command in COMMANDS.iter() {
        print!("{}: ", command.name);
        println!("{}", command.description);
    }
}

fn clear(_cmd: Vec<String>) {
    clear_screen!();
}

fn xhci_cap_logs(_cmd: Vec<String>) {
    #[allow(static_mut_refs)]
    let xhci_driver = unsafe { 
        XHCI_DRIVER.as_ref().expect("xHCI not initialized!") 
    };

    xhci_driver.log_capability_registers();
}

fn xhci_op_logs(_cmd: Vec<String>) {
    #[allow(static_mut_refs)]
    let xhci_driver = unsafe { 
        XHCI_DRIVER.as_ref().expect("xHCI not initialized!") 
    };

    xhci_driver.log_operational_registers();
}

fn xhci_logs(_cmd: Vec<String>) {
    #[allow(static_mut_refs)]
    let xhci_driver = unsafe { 
        XHCI_DRIVER.as_ref().expect("xHCI not initialized!") 
    };

    xhci_driver.log_capability_registers();
    xhci_driver.log_operational_registers();
}

fn change_color(cmd: Vec<String>){
    if cmd.len() <= 1 {
        println!("Not enough args for set_color");
        return;
    } else if cmd.len() > 3 {
        println!("To many args for set_color");
        return;
    } 

    set_color!(cmd);
}

fn update_color(_cmd: Vec<String>){
    update_color!();
}