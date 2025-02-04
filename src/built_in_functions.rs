use crate::console;
use crate::error::SeeedError;
use crate::parser::{Expression, Literal};
use crate::script::ScriptContext;

fn execute_echo(args:Vec<Literal>, script_context: &mut ScriptContext) -> Result<(), SeeedError> {
    for arg in args {
        console::message(arg.to_string().as_str())
    }

    Ok(())
}

fn execute_upload(args:Vec<Literal>, script_context: &mut ScriptContext) -> Result<(), SeeedError> {

    if args.len() != 2 {
        return Err(SeeedError::WrongArgCount(2, args.len()));
    }

    let source = args.get(0).unwrap();  // unwrap because args length was checked previously


    //let source = script_context.expand_expr(&source)?;

    let target = args.get(1).unwrap();  // unwrap because args length was checked previously

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
            script_context.ssh_client.upload(content.as_str(), target.to_string())?;
        },
        Literal::String(file_path) => {
            let contents = std::fs::read_to_string(file_path)?;
            script_context.ssh_client.upload(contents.as_str(), target.to_string())?;
        },
        _ => return Err(SeeedError::BadArgument("could not load file content")),
    };

    Ok(())
}

pub fn execute_function(name: &str, args: Vec<Literal>, script_context: &mut ScriptContext) -> Result<(), SeeedError> {

    match name {
        "echo" => execute_echo(args, script_context)?,
        "upload" => execute_upload(args, script_context)?,
        &_ => return Err(SeeedError::UnknownFunction())
    }
    Ok(())
}