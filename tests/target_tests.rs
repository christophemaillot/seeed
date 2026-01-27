use seeed::script::ScriptContext;
use seeed::sshclient::RemoteExecutor;
use seeed::error::SeeedError;
use std::sync::{Arc, Mutex};

// Mock Executor that records commands and connection targets
#[derive(Clone)]
struct MockExecutor {
    commands: Arc<Mutex<Vec<String>>>,
    connection_target: Arc<Mutex<Option<String>>>,
}

impl MockExecutor {
    fn new() -> Self {
        Self {
            commands: Arc::new(Mutex::new(Vec::new())),
            connection_target: Arc::new(Mutex::new(None)),
        }
    }
}

impl RemoteExecutor for MockExecutor {
    fn connect(&mut self, target: &str) -> Result<(), SeeedError> {
        let mut conn = self.connection_target.lock().unwrap();
        *conn = Some(target.to_string());
        Ok(())
    }

    fn command(&self, command: &str) -> Result<(), SeeedError> {
        self.commands.lock().unwrap().push(command.to_string());
        Ok(())
    }

    fn run(&self, script: &str) -> Result<(), SeeedError> {
        self.commands.lock().unwrap().push(format!("RUN: {}", script));
        Ok(())
    }

    fn upload(&self, _content: &str, _dst_path: String) -> Result<(), SeeedError> {
        Ok(())
    }
}

#[test]
fn test_target_from_cli() {
    let script_content = "| echo \"hello\"\n";
    let mock = MockExecutor::new();
    let executor = Box::new(mock.clone());
    
    // Provide target via CLI arg (simulated)
    let mut context = ScriptContext::new(
        Some("cli_user@cli_host".to_string()), 
        false, 
        script_content.to_string(), 
        executor
    );

    context.run(false).unwrap();

    let conn = mock.connection_target.lock().unwrap();
    assert_eq!(conn.as_ref().unwrap(), "cli_user@cli_host");
}

#[test]
fn test_target_from_script_variable() {
    let script_content = "let target = \"script_user@script_host\"\n| echo \"hello\"\n";
    let mock = MockExecutor::new();
    let executor = Box::new(mock.clone());
    
    // No target via CLI arg
    let mut context = ScriptContext::new(
        None, 
        false, 
        script_content.to_string(), 
        executor
    );

    context.run(false).unwrap();

    let conn = mock.connection_target.lock().unwrap();
    assert_eq!(conn.as_ref().unwrap(), "script_user@script_host");
}

#[test]
fn test_missing_target_error() {
    let script_content = "| echo \"hello\"\n";
    let mock = MockExecutor::new();
    let executor = Box::new(mock.clone());
    
    // No target anywhere
    let mut context = ScriptContext::new(
        None, 
        false, 
        script_content.to_string(), 
        executor
    );

    let result = context.run(false);
    
    assert!(result.is_err());
    // matching on specific error would be better if possible, assuming BadTarget
    match result {
        Err(SeeedError::BadTarget) => (),
        _ => panic!("Expected BadTarget error"),
    }
}
