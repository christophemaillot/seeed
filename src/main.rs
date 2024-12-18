mod console;
mod parser;
mod error;
mod script;
mod sshclient;
mod built_in_functions;

use std::path::PathBuf;
use clap::Parser;


use crate::error::SeeedError;
use crate::script::ScriptContext;

#[derive(clap::Parser, Debug)]
#[clap(version, about, long_about = None)]
struct App {
    #[clap(long, short = 's', help = "use sudo to run the script", default_value_t = false, action)]
    sudo: bool,

    #[clap(long, short = 't', help = "The target host to run the script on (<user>@<host>:<port>)")]
    target: String,

    #[clap(long, short = 'e', help = "The shell to use for the script", default_value_t = String::from("/bin/bash"))]
    shell:String,

    #[clap(long, short = 'd', help = "use sudo to run the script", default_value_t = false, action)]
    debug: bool,

    /// Input files
    file: PathBuf,
}


fn main() -> Result<(), SeeedError> {

    // display a welcome message
    console::log(format!("{} version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")).as_str());

    // parse the command line arguments
    let app = App::parse();

    // read the input file contents
    let contents = std::fs::read_to_string(app.file)?;
    let mut script_context = ScriptContext::new(app.target, app.sudo, contents);
    script_context.run(app.debug)?;

    Ok(())
}
