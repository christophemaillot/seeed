use seeed::parser::{script_parser, Statement, Expression, Literal};
use seeed::script::ScriptContext;
use seeed::sshclient::RemoteExecutor;
use seeed::error::SeeedError;

// Mock executor for parser tests (though parser itself doesn't use executor, 
// ScriptContext might if we test evaluation)
struct MockExecutor;
impl RemoteExecutor for MockExecutor {
    fn connect(&mut self, _target: &str) -> Result<(), SeeedError> { Ok(()) }
    fn command(&self, _command: &str) -> Result<(), SeeedError> { Ok(()) }
    fn run(&self, _script: &str) -> Result<(), SeeedError> { Ok(()) }
    fn upload(&self, _content: &str, _dst_path: String) -> Result<(), SeeedError> { Ok(()) }
}

#[test]
fn test_parse_simple_assignment() {
    let script = "let x = 10";
    let ast = script_parser().parse(script.as_bytes()).unwrap();
    assert_eq!(ast.statements.len(), 1);
    match &ast.statements[0] {
        Statement::Assign(name, expr) => {
            assert_eq!(name, "x");
            match expr {
                Expression::Literal(Literal::Integer(val)) => assert_eq!(val, &10),
                _ => panic!("Expected integer literal"),
            }
        }
        _ => panic!("Expected assignment statement"),
    }
}

#[test]
fn test_parse_and_evaluate_string_interpolation() {
    let script_content = "let name = \"world\"\nlet msg = \"hello {{ name }}\"\n";
    
    let executor = Box::new(MockExecutor);
    let mut context = ScriptContext::new("user@host".to_string(), false, script_content.to_string(), executor);
    
    context.run(false).unwrap();
    
    // We can't easily inspect context.variables because they are private.
    // Ideally we should add a getter for testing, or use reflection/debug output.
    // For now, let's verify it doesn't crash.
}
