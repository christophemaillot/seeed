use std::collections::HashMap;
use minijinja::Environment;

use crate::parser::{script_parser, Expression, Literal, Statement};
use crate::error::SeeedError;
use crate::built_in_functions;
use crate::sshclient::RemoteExecutor;

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
    variables: HashMap<String, Literal>,
    pub(crate) ssh_client: Box<dyn RemoteExecutor>,
}

impl ScriptContext {

    /// build a new script context with default parameters
    ///
    pub fn new(target: String, use_sudo: bool, contents: String, ssh_client: Box<dyn RemoteExecutor>) -> Self {
        Self {
            target,
            use_sudo,
            contents,
            variables: HashMap::new(),
            ssh_client
        }
    }

    /// Loads a environment file and sets the corresponding variables
    pub fn load_env(&mut self, filename: &str) -> Result<(), SeeedError> {
        let env_variables = env_file_reader::read_file(filename)?;

        env_variables.iter().for_each(|(name, value)| {
            self.variables.insert(name.clone(), Literal::String(value.clone()));
        });

        Ok(())
    }

    /// Main method that runs the script
    ///
    pub fn run(&mut self, debug: bool) -> Result<(), SeeedError> {

        // parse the script
        let data = self.contents.as_bytes();
        let script = script_parser().parse(data)?;


        // if debug flag is set,
        if debug {
            println!("script content :");
            script.statements.iter().for_each(|item| {
                println!("> {:?}", item);
            });
        }

        // instanciate the ssh client
        self.ssh_client.connect(self.target.as_str())?;
        if self.use_sudo {
            self.ssh_client.command("sudo mkdir -p /var/lib/seeed/ && sudo chown $(whoami) /var/lib/seeed/ ")?;
        } else {
            self.ssh_client.command("mkdir -p /var/lib/seeed/")?;
        }


        // execute the script
        for statement in script.statements {
            self.execute_statement(&statement)?;
        }

        Ok(())
    }

    fn execute_statement(&mut self, statement: &Statement) -> Result<(), SeeedError> {
        match statement {

            Statement::Comment() => {
                // nothing to do
            }

            Statement::EmptyLine() => {
                // nothing to do
            }

            Statement::Assign(name, expression) => {
                let literal = self.evaluate(expression)?;
                self.variables.insert(name.clone(), literal);
            }
            Statement::RemoteSingle(line) => {
                let line = self.resolve_template(&line)?;
                self.ssh_client.run(line.as_str())?;
            }
            Statement::Remote(lines) => {
                let line = lines.join("\n");
                let line = self.resolve_template(&line)?;

                self.ssh_client.run(line.as_str())?;
            }
            Statement::FnCall(name, args) => {

                let dst_args: Vec<Result<Literal, SeeedError>> = args
                    .iter()
                    .map(|arg| self.evaluate(arg))
                    .collect();

                let dst_args = dst_args.into_iter().collect::<Result<Vec<_>, _>>()?;

                self.call_builtin_function(name, dst_args)?;
            }
            Statement::ForLoop(varname, expression, statements) => {

                let literal = self.evaluate(expression)?;

                if let Literal::Array(literals) = literal {
                    for literal in literals {
                        self.variables.insert(varname.clone(), literal.clone());
                        for statement in statements {
                            self.execute_statement(statement)?;
                        }
                    }
                } else {
                    println!("error : {:?}", expression);
                    return Err(SeeedError::IterateOverArray)
                }
            }
        }
        Ok(())
    }

    fn evaluate(&mut self, expression: &Expression) -> Result<Literal, SeeedError> {
        match expression {
            Expression::Literal(literal) => {
                match literal {
                    Literal::HereDoc(content) => {
                        Ok(Literal::HereDoc(self.resolve_template(content)?))
                    }
                    Literal::String(content) => {
                        Ok(Literal::String(self.resolve_template(content)?))
                    }
                    lit => Ok(lit.clone())
                }
            }
            Expression::Variable(name) => {
                let value = self.variables.get(name);
                match value {
                    Some(value) => Ok(value.clone()),
                    None => Err(SeeedError::UndefinedVar(name.clone()))
                }

            }
            Expression::FnCall(name, src_args) => {
                let args = src_args.iter().map(|arg| self.evaluate(arg)).collect::<Result<Vec<_>, _>>()?;

                let result = self.call_builtin_function(name, args)?;
                Ok(result)

            }
            Expression::Array(src_array) => {
                let mut result:Vec<Literal> = vec![];
                for exp in src_array {
                    let literal = self.evaluate(exp)?;
                    result.push(literal);
                }

                Ok(Literal::Array(result))
            }
            Expression::HereDoc(content) => {
                Ok(Literal::HereDoc(self.resolve_template(content)?))
            }
        }
    }

    fn call_builtin_function(&mut self, name: &str, args: Vec<Literal>) -> Result<Literal, SeeedError> {
        built_in_functions::execute_function(name, args, self)?;
        Ok(Literal::Void)
    }


    pub(crate) fn resolve_template(&self, source:&str) -> Result<String, SeeedError> {
        let mut env = Environment::new();
        env.add_template("template", source)?;
        let tmpl = env.get_template("template")?;
        let result = tmpl.render(&self.variables)?;
        Ok(result)
    }
}


