use std::collections::HashMap;
use crate::parser::{script_parser, Expr, ScriptItem};

use chumsky::Parser;
use minijinja::Environment;
use crate::error::SeeedError;
use crate::built_in_functions::execute_function;
use crate::sshclient::SshClient;

/// The script execution context
///
/// contains :
/// - the script content itself,
/// - a ssh client connected to the remote host,
/// - the defined variables and their values
/// and provides a set of utility methods
///
pub struct ScriptContext {
    target: String,
    use_sudo: bool,
    contents: String,
    variables: HashMap<String, Expr>,
    pub(crate) ssh_client: SshClient,
}

impl ScriptContext {

    /// build a ne script context with default parameters
    ///
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
                ScriptItem::Remote(lines) => {
                    let content = self.resolve_template(
                        lines.join("\n").as_str()
                    )?;
                    self.ssh_client.run(&content)?;
                },
                ScriptItem::Comment() => {
                    // ignore comments
                }
                ScriptItem::EmptyLine() => {
                    // ignore empty lines
                }
                ScriptItem::FnCall(name, args) => {
                    execute_function(&name, args, self)?;
                },
                ScriptItem::VarAssign(name, value) => {
                    self.variables.insert(name, value);
                }
            }
        }
        Ok(())
    }

    /// expand an expression item to a litteral expression
    ///
    /// If the exp is a variable, replace it by its actual value,
    /// if the exp is a string or here doc : apply the template engine
    pub(crate) fn expand_expr(&mut self, expr: &Expr) -> Result<Expr, SeeedError> {

        let expr = match expr {
            Expr::Variable(name) => {
                let var = self.variables.get(name);
                match var {
                    None => {
                        return Err(SeeedError::UndefinedVar(name.clone()))
                    }
                    Some(val) => {
                        val.clone()
                    }
                }
            }
            _ => expr.clone(),
        };

        let expr: Result<Expr, SeeedError> = match expr {
            Expr::String(source) => {
                Ok(Expr::String(self.resolve_template(&source)?))
            }
            Expr::HereDoc(doc) => {
                Ok(Expr::HereDoc(self.resolve_template(&doc)?))
            }
            _ => Ok(expr)
        };

        Ok(expr?)
    }


    pub(crate) fn resolve_template(&self, source:&str) -> Result<String, SeeedError> {
        let mut env = Environment::new();
        env.add_template("template", source)?;
        let tmpl = env.get_template("template")?;
        let result = tmpl.render(&self.variables)?;
        Ok(result)
    }
}


