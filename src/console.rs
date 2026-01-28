use colored::Colorize;

/// log a message to the console, with a green color, and a 🌱 emoji
/// to indicate that it is a standard log message, either from the
/// scripting or from the system.
pub fn log(msg: &str) {
    println!("🌱 {}", msg.green());
}

#[allow(dead_code)]
pub fn error(msg: &str) {
    println!("🚨 {}", msg.red());
}

pub fn message(msg: &str) {
    println!("🖥  - {}", msg.green());
}