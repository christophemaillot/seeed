use std::collections::HashMap;
use minijinja::Environment;

use crate::parser::{script_parser, Expression, Literal, Statement};
use crate::error::SeeedError;
use crate::built_in_functions;
use crate::sshclient::RemoteExecutor;
use regex::Regex;

/// Configuration extracted from script headers
#[derive(Debug, Default)]
pub struct ScriptConfig {
    pub target: Option<String>,
    pub sudo: Option<bool>,
}

/// Parses the script content to extract configuration headers
///
/// Headers are extracted from the initial comment block of the script.
/// The parsing stops at the first non-comment non-empty line.
///
/// Supported headers:
/// - `# @target: <user>@<host>:<port>`
/// - `# @sudo: <true|false>`
///
pub fn parse_script_headers(content: &str) -> ScriptConfig {
    let mut config = ScriptConfig::default();
    let re_target = Regex::new(r"^\s*#\s*@target:\s*(.+)$").unwrap();
    let re_sudo = Regex::new(r"^\s*#\s*@sudo:\s*(true|false)$").unwrap();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if !trimmed.starts_with('#') {
            break;
        }

        if let Some(captures) = re_target.captures(trimmed) {
            config.target = Some(captures.get(1).unwrap().as_str().trim().to_string());
        }

        if let Some(captures) = re_sudo.captures(trimmed) {
             let val = captures.get(1).unwrap().as_str();
             config.sudo = Some(val == "true");
        }
    }
    config
}

/// The script execution context
///
/// contains :
/// - the script content itself,
/// - a ssh client connected to the remote host,
/// - the defined variables and their values
/// and provides a set of utility methods
///
pub struct ScriptContext {
    target: Option<String>,
    use_sudo: bool,
    contents: String,
    variables: HashMap<String, Literal>,
    pub(crate) ssh_client: Box<dyn RemoteExecutor>,
    connected: bool,
}

impl ScriptContext {

    /// build a new script context with default parameters
    ///
    pub fn new(target: Option<String>, use_sudo: bool, contents: String, ssh_client: Box<dyn RemoteExecutor>) -> Self {
        Self {
            target,
            use_sudo,
            contents,
            variables: HashMap::new(),
            ssh_client,
            connected: false,
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
        let script = script_parser().parse(data).map_err(|e| {
            let position = match &e {
                pom::Error::Mismatch { position, .. } => *position,
                pom::Error::Conversion { position, .. } => *position,
                pom::Error::Expect { position, .. } => *position,
                pom::Error::Incomplete => self.contents.len(),
                pom::Error::Custom { position, .. } => *position,
            };
            
            let mut current_line = 1;
            let mut last_newline_pos = -1;
            for (i, c) in self.contents.char_indices() {
                if i >= position {
                    break;
                }
                if c == '\n' {
                    current_line += 1;
                    last_newline_pos = i as i64;
                }
            }
            let current_col = position as i64 - last_newline_pos;

            let line_content = self.contents.lines().nth(current_line - 1).unwrap_or("").to_string();
            let pointer = " ".repeat((current_col - 1) as usize) + "^";

            SeeedError::ParseError {
                message: format!("{:?}", e), // pom error usually has some info
                line: current_line,
                col: current_col as usize,
                line_content,
                pointer,
            }
        })?;


        // if debug flag is set,
        if debug {
            println!("script content :");
            script.statements.iter().for_each(|item| {
                println!("> {:?}", item);
            });
        }

        // instanciate the ssh client
        // self.ssh_client.connect(self.target.as_str())?; -> Moved to lazy connection

        
        // No need to create directory manually, sshclient handles temp files in /tmp/


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
                self.ensure_connected()?;
                let line = self.resolve_template(&line)?;
                self.ssh_client.run(line.as_str())?;
            }
            Statement::Remote(lines) => {
                self.ensure_connected()?;
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


    fn ensure_connected(&mut self) -> Result<(), SeeedError> {
        if self.connected {
            return Ok(());
        }

        // Try to find target in variables if not in struct
        let target = if let Some(target) = &self.target {
            target.clone()
        } else {
             return Err(SeeedError::BadTarget);
        };

        println!("Connecting to target: {}", target);
        self.ssh_client.connect(&target)?;
        self.connected = true;
        Ok(())
    }

}


