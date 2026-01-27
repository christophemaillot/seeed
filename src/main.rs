use seeed::console;
use seeed::parser;
use seeed::error;
use seeed::script;
use seeed::sshclient;
use seeed::built_in_functions;

use std::path::PathBuf;
use clap::Parser;



use seeed::error::SeeedError;
use seeed::script::ScriptContext;

#[derive(clap::Parser, Debug)]
#[clap(version, about, long_about = None)]
struct App {
    #[clap(long, short = 's', help = "use sudo to run the script", default_value_t = false, action)]
    sudo: bool,

    #[clap(long, short = 't', help = "The target host to run the script on (<user>@<host>:<port>)")]
    target: String,

    #[clap(long, short = 'e', help = "The shell to use for the script", default_value_t = String::from("/bin/bash"))]
    shell:String,

    #[clap(long, short = 'd', help = "print debug information", default_value_t = false, action)]
    debug: bool,

    #[clap(long,  help = "load environment variables",)]
    env: Option<String>,
    
    /// Input files
    file: PathBuf,
}

fn main() -> Result<(), SeeedError> {

    // display a welcome message
    console::log(format!("{} version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")).as_str());

    // parse the command line arguments
    let app = App::parse();

    console::log(format!("target is {}", app.target).as_str());
    if app.sudo {
        console::log("using sudo");
    }

    // read the input file contents
    let contents = std::fs::read_to_string(app.file)?;
    let ssh_client = Box::new(seeed::sshclient::SshClient::new(app.sudo));
    let mut script_context = ScriptContext::new(app.target, app.sudo, contents, ssh_client);

    if let Some(env_file) = app.env {
        script_context.load_env(&env_file)?;
    }


    let result = script_context.run(app.debug);
    match result {
        Ok(_) => console::log("script completed successfully"),
        Err(seeed_error)  => {
            console::log(format!("script execution failed : {}", seeed_error).as_str());
        }
    }

    Ok(())
}
