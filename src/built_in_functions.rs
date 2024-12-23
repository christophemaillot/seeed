use crate::console;
use crate::error::SeeedError;
use crate::parser::{Expr};
use crate::script::ScriptContext;

fn execute_echo(args:Vec<Expr>, script_context: &mut ScriptContext) -> Result<(), SeeedError> {
    for arg in args {
        let expr = script_context.expand_expr(&arg)?;
        console::message(expr.to_string().as_str())
    }

    Ok(())
}

fn execute_upload(args:Vec<Expr>, script_context: &mut ScriptContext) -> Result<(), SeeedError> {

    if args.len() != 2 {
        return Err(SeeedError::WrongArgCount(2, args.len()));
    }

    let source = args.get(0).unwrap();  // unwrap because args length was checked previously

    let source = script_context.expand_expr(&source)?;

    let target = args.get(1).unwrap();  // unwrap because args length was checked previously

    // check source type
    match source {
        Expr::String(_) => {}
        Expr::HereDoc(_) => {}
        _ => return Err(SeeedError::BadArgType("first argument of upload must be a string or a heredoc".to_owned())),
    }

    match target {
        Expr::String(_) => {}
        _ =>  return Err(SeeedError::BadArgType("second argument of upload must be a string".to_owned()))
    }

    let source = script_context.expand_expr(&source)?;

    match source {
        Expr::HereDoc(content) => {
            script_context.ssh_client.upload(content.as_str(), target.to_string())?;
        },
        Expr::String(file_path) => {
            let contents = std::fs::read_to_string(file_path)?;
            script_context.ssh_client.upload(contents.as_str(), target.to_string())?;
        },
        _ => return Err(SeeedError::BadArgument("could not load file content")),
    };

    Ok(())
}

pub fn execute_function(name: &str, args: Vec<Expr>, script_context: &mut ScriptContext) -> Result<(), SeeedError> {

    match name {
        "echo" => execute_echo(args, script_context)?,
        "upload" => execute_upload(args, script_context)?,
        &_ => return Err(SeeedError::UnknownFunction())
    }
    Ok(())
}