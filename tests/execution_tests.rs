use seeed::script::ScriptContext;
use seeed::sshclient::RemoteExecutor;
use seeed::error::SeeedError;
use std::sync::{Arc, Mutex};

// Mock Executor that records commands
#[derive(Clone)]
struct MockExecutor {
    commands: Arc<Mutex<Vec<String>>>,
    uploads: Arc<Mutex<Vec<(String, String)>>>
}

impl MockExecutor {
    fn new() -> Self {
        Self {
            commands: Arc::new(Mutex::new(Vec::new())),
            uploads: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl RemoteExecutor for MockExecutor {
    fn connect(&mut self, _target: &str) -> Result<(), SeeedError> {
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

    fn upload(&self, content: &str, dst_path: String) -> Result<(), SeeedError> {
        self.uploads.lock().unwrap().push((content.to_string(), dst_path));
        Ok(())
    }
}

#[test]
fn test_execution_simple_remote() {
    let script_content = "| echo \"hello\"\n";

    let mock = MockExecutor::new();
    let executor = Box::new(mock.clone());
    let mut context = ScriptContext::new("user@host".to_string(), false, script_content.to_string(), executor);

    context.run(false).unwrap();

    let commands = mock.commands.lock().unwrap();
    // 0: RUN:  echo "hello"
    assert!(commands.len() >= 1);
    assert_eq!(commands[0], "RUN:  echo \"hello\"");
}

#[test]
fn test_loop_execution() {
    let script_content = "let users = [\"alice\", \"bob\"]\nfor user in $users {\n|echo {{ user }}\n}\n";

    let mock = MockExecutor::new();
    let executor = Box::new(mock.clone());
    let mut context = ScriptContext::new("user@host".to_string(), false, script_content.to_string(), executor);

    context.run(false).unwrap();

    let commands = mock.commands.lock().unwrap();
    // 0: echo alice
    // 1: echo bob
    assert!(commands.len() >= 2);
    assert!(commands.contains(&"RUN: echo alice".to_string()));
    assert!(commands.contains(&"RUN: echo bob".to_string()));
}
