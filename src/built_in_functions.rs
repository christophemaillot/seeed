use crate::console;
use crate::error::SeeedError;
use crate::parser::Literal;
use crate::script::ScriptContext;

fn execute_echo(args:Vec<Literal>, _script_context: &mut ScriptContext) -> Result<(), SeeedError> {
    for arg in args {
        console::message(arg.to_string().as_str())
    }

    Ok(())
}

fn execute_upload(args:Vec<Literal>, script_context: &mut ScriptContext) -> Result<(), SeeedError> {

    if args.len() != 2 {
        return Err(SeeedError::WrongArgCount(2, args.len()));
    }

    let source = args.get(0).ok_or(SeeedError::BadArgument("missing source argument"))?;
    let target = args.get(1).ok_or(SeeedError::BadArgument("missing target argument"))?;

    // check source type
    match source {
        Literal::String(_) => {}
        Literal::HereDoc(_) => {}
        _ => return Err(SeeedError::BadArgType("first argument of upload must be a string or a heredoc".to_owned())),
    }

    match target {
        Literal::String(_) => {}
        _ =>  return Err(SeeedError::BadArgType("second argument of upload must be a string".to_owned()))
    }


    match source {
        Literal::HereDoc(content) => {
            script_context.ssh_client.upload(content.as_bytes(), target.to_string())?;
        },
        Literal::String(file_path) => {
            match std::fs::read(file_path) {
                Ok(contents) => {   
                    script_context.ssh_client.upload(&contents, target.to_string())?;
                },
                Err(e) => {
                    println!("could not load file content: {}", e);
                    return Err(SeeedError::BadArgument("loading failed"))
                }
            }
        },
        _ => return Err(SeeedError::BadArgument("could not load file content")),
    };

    Ok(())
}

fn execute_exec(args: Vec<Literal>, _script_context: &mut ScriptContext) -> Result<(), SeeedError> {
    if args.len() != 1 {
        return Err(SeeedError::WrongArgCount(1, args.len()));
    }

    let command = args[0].to_string();

    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .status()
        .map_err(|_| SeeedError::BadArgument("Failed to execute command"))?;

    if !status.success() {
        return Err(SeeedError::BadArgument("Command execution failed"));
    }

    Ok(())
}

pub fn execute_function(name: &str, args: Vec<Literal>, script_context: &mut ScriptContext) -> Result<(), SeeedError> {

    println!("Executing function: '{}'", name);
    match name {
        "echo" => execute_echo(args, script_context)?,
        "upload" => execute_upload(args, script_context)?,
        "exec" => execute_exec(args, script_context)?,
        &_ => {
            println!("Unknown function: {}", name);
            return Err(SeeedError::UnknownFunction())
        }
    }
    Ok(())
}