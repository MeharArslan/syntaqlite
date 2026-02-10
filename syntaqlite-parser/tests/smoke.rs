use syntaqlite_parser::{NodeTag, Parser, Tokenizer, TokenType};

#[test]
fn parse_select_1() {
    let mut parser = Parser::new();
    let mut session = parser.parse("SELECT 1;");

    let root_id = session.next_statement().unwrap().unwrap();
    let node = session.node(root_id).unwrap();
    assert_eq!(node.tag(), NodeTag::SelectStmt);

    let select = node.as_select_stmt().unwrap();
    assert_eq!(select.tag, NodeTag::SelectStmt as u32);

    // No more statements.
    assert!(session.next_statement().is_none());
}

#[test]
fn parse_multiple_statements() {
    let mut parser = Parser::new();
    let mut session = parser.parse("SELECT 1; SELECT 2;");

    let root1 = session.next_statement().unwrap().unwrap();
    assert_eq!(session.node(root1).unwrap().tag(), NodeTag::SelectStmt);

    let root2 = session.next_statement().unwrap().unwrap();
    assert_eq!(session.node(root2).unwrap().tag(), NodeTag::SelectStmt);

    assert!(session.next_statement().is_none());
}

#[test]
fn parse_error() {
    let mut parser = Parser::new();
    let mut session = parser.parse("SELECT");

    let result = session.next_statement().unwrap();
    assert!(result.is_err());
}

#[test]
fn tokenize_select() {
    let mut tokenizer = Tokenizer::new();
    let tokens: Vec<_> = tokenizer.tokenize("SELECT 1").collect();

    assert_eq!(tokens[0].token_type, TokenType::Select);
    assert_eq!(tokens[0].text, "SELECT");

    assert_eq!(tokens[1].token_type, TokenType::Space);
    assert_eq!(tokens[1].text, " ");

    assert_eq!(tokens[2].token_type, TokenType::Integer);
    assert_eq!(tokens[2].text, "1");
}

#[test]
fn parser_reuse() {
    let mut parser = Parser::new();

    // First parse
    {
        let mut session = parser.parse("SELECT 1");
        let root = session.next_statement().unwrap().unwrap();
        assert_eq!(session.node(root).unwrap().tag(), NodeTag::SelectStmt);
    }

    // Reuse with different input
    {
        let mut session = parser.parse("DELETE FROM t");
        let root = session.next_statement().unwrap().unwrap();
        assert_eq!(session.node(root).unwrap().tag(), NodeTag::DeleteStmt);
    }
}
