use std::collections::HashMap;
use crate::parser::{script_parser, Expr, ScriptItem};

use chumsky::Parser;
use crate::error::SeeedError;
use crate::built_in_functions::execute_function;
use crate::sshclient::SshClient;

pub struct ScriptContext {
    target: String,
    use_sudo: bool,
    contents: String,
    variables: HashMap<String, Expr>,
    pub(crate) ssh_client: SshClient,
}

impl ScriptContext {
    pub fn new(target: String, use_sudo: bool, contents: String) -> Self {
        Self {
            target,
            use_sudo,
            contents,
            variables: HashMap::new(),
            ssh_client: SshClient::new(use_sudo)
        }
    }

    pub(crate) fn run(&mut self, debug: bool) -> Result<(), SeeedError> {

        // parse the script
        let script = script_parser().parse(self.contents.as_str()).unwrap();

        if debug {
            println!("script content :");
            script.items.iter().for_each(|item| {
                println!("> {:?}", item);
            });
            return Ok(())
        }

        // instanciate the ssh client
        self.ssh_client.connect(self.target.as_str())?;
        self.ssh_client.command("mkdir -p /var/lib/seeed/")?;

        // execute the script
        for item in script.items {
            match item {
                ScriptItem::RemoteSingle(s) => {
                    self.ssh_client.run(s.as_str())?;
                },
                ScriptItem::Remote(s) => {
                    self.ssh_client.run(s.join("\n").as_str())?;
                },
                ScriptItem::Comment() => {
                    // ignore comments
                }
                ScriptItem::EmptyLine() => {
                    // ignore empty lines
                }
                ScriptItem::FnCall(name, args) => {
                    execute_function(&name, args, &self)?;
                },
                ScriptItem::VarAssign(name, value) => {
                    self.variables.insert(name, value);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn deref_vars(&self, args:&Vec<Expr>) -> Result<Vec<Expr>, SeeedError> {

        let mut resolved_args:Vec<Expr> = Vec::new();

        for arg in args {
            match arg {
                Expr::Variable(name) => {
                    let var = self.variables.get(name);
                    match var {
                        None => {
                            return Err(SeeedError::UndefinedVar(name.clone()))
                        }
                        Some(val) => {
                            resolved_args.push(val.clone());
                        }
                    }
                }
                _ => {
                    resolved_args.push(arg.clone());
                }
            }
        }

        Ok(resolved_args)
    }
}


