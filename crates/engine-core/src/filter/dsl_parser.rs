use super::dsl_ast::*;

pub struct DslParser;

impl DslParser {
    pub fn parse(input: &str) -> Result<DslExpr, DslParseError> {
        let tokens = tokenize(input)?;
        if tokens.is_empty() {
            return Err(DslParseError { message: "空表达式".to_string(), position: 0 });
        }
        let (expr, pos) = parse_or(&tokens, 0)?;
        if pos < tokens.len() {
            return Err(DslParseError {
                message: format!("意外的标记: {:?}", tokens[pos]),
                position: pos,
            });
        }
        Ok(expr)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    FieldMatch { field: DslField, op: DslOp, value: DslValue },
    And,
    Or,
    Not,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Result<Vec<Token>, DslParseError> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some(&(i, ch)) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        if ch == '(' {
            tokens.push(Token::LParen);
            chars.next();
            continue;
        }
        if ch == ')' {
            tokens.push(Token::RParen);
            chars.next();
            continue;
        }

        let _word_start = i;
        let word = read_word(&mut chars, input);

        let upper = word.to_uppercase();
        if upper == "AND" || upper == "&&" {
            tokens.push(Token::And);
            continue;
        }
        if upper == "OR" || upper == "||" {
            tokens.push(Token::Or);
            continue;
        }
        if upper == "NOT" || word == "!" {
            tokens.push(Token::Not);
            continue;
        }

        if let Some((field, rest)) = try_parse_field_prefix(&word) {
            let (op, value) = parse_op_value(rest)?;
            tokens.push(Token::FieldMatch { field, op, value });
            continue;
        }

        let value = DslValue::String(word.to_string());
        tokens.push(Token::FieldMatch {
            field: DslField::Url,
            op: DslOp::Contains,
            value,
        });
    }

    Ok(tokens)
}

fn read_word<'a>(chars: &mut std::iter::Peekable<impl Iterator<Item = (usize, char)>>, input: &'a str) -> &'a str {
    let start = chars.peek().map(|&(i, _)| i).unwrap_or(0);
    let mut end = start;
    let mut depth = 0usize;
    while let Some(&(_, ch)) = chars.peek() {
        if ch == '(' { depth += 1; }
        if ch == ')' {
            if depth == 0 { break; }
            depth -= 1;
        }
        if ch.is_whitespace() && depth == 0 { break; }
        chars.next();
        end = chars.peek().map(|&(i, _)| i).unwrap_or(input.len());
    }
    &input[start..end]
}

fn try_parse_field_prefix(word: &str) -> Option<(DslField, &str)> {
    let colon_pos = word.find(':')?;
    let field_str = &word[..colon_pos];
    let rest = &word[colon_pos + 1..];
    if rest.is_empty() { return None; }

    let field = match field_str.to_lowercase().as_str() {
        "method" | "m" => DslField::Method,
        "url" | "u" => DslField::Url,
        "host" | "h" => DslField::Host,
        "path" => DslField::Path,
        "status" | "s" | "code" => DslField::Status,
        "proc" | "process" | "p" => DslField::ProcessName,
        "pid" => DslField::ProcessId,
        "body" | "b" => DslField::Body,
        "content-type" | "ct" | "contenttype" => DslField::ContentType,
        "scheme" => DslField::Scheme,
        "duration" | "dur" | "d" => DslField::Duration,
        "size" | "sz" => DslField::Size,
        other if other.starts_with("header.") || other.starts_with("hdr.") => {
            let header_name = if other.starts_with("header.") {
                &other[7..]
            } else {
                &other[4..]
            };
            DslField::Header(header_name.to_string())
        }
        _ => return None,
    };
    Some((field, rest))
}

fn parse_op_value(rest: &str) -> Result<(DslOp, DslValue), DslParseError> {
    if rest.starts_with("=~") {
        let val = &rest[2..];
        Ok((DslOp::Regex, DslValue::String(val.to_string())))
    } else if rest.starts_with('~') {
        let val = &rest[1..];
        Ok((DslOp::Regex, DslValue::String(val.to_string())))
    } else if rest.starts_with("!=") || rest.starts_with("<>") {
        let val = &rest[2..];
        Ok((DslOp::NotEquals, parse_value(val)))
    } else if rest.starts_with('>') {
        let val = &rest[1..];
        Ok((DslOp::GreaterThan, parse_value(val)))
    } else if rest.starts_with('<') {
        let val = &rest[1..];
        Ok((DslOp::LessThan, parse_value(val)))
    } else if rest.starts_with('=') {
        let val = &rest[1..];
        Ok((DslOp::Equals, parse_value(val)))
    } else if rest.contains("..") {
        let parts: Vec<&str> = rest.split("..").collect();
        if parts.len() == 2 {
            let lo: f64 = parts[0].parse().map_err(|_| DslParseError {
                message: format!("无效范围起始: {}", parts[0]),
                position: 0,
            })?;
            let hi: f64 = parts[1].parse().map_err(|_| DslParseError {
                message: format!("无效范围结束: {}", parts[1]),
                position: 0,
            })?;
            Ok((DslOp::Range, DslValue::Range(lo, hi)))
        } else {
            Ok((DslOp::Contains, DslValue::String(rest.to_string())))
        }
    } else if rest.contains('*') || rest.contains('?') {
        Ok((DslOp::Wildcard, DslValue::String(rest.to_string())))
    } else {
        Ok((DslOp::Contains, parse_value(rest)))
    }
}

fn parse_value(val: &str) -> DslValue {
    if let Ok(n) = val.parse::<f64>() {
        return DslValue::Number(n);
    }
    if val.ends_with("ms") {
        if let Ok(n) = val.trim_end_matches("ms").parse::<f64>() {
            return DslValue::DurationMs(n as u64);
        }
    }
    if val.ends_with('s') && !val.ends_with("ms") {
        if let Ok(n) = val.trim_end_matches('s').parse::<f64>() {
            return DslValue::DurationMs((n * 1000.0) as u64);
        }
    }
    if val.ends_with("KB") || val.ends_with("kb") {
        if let Ok(n) = val.trim_end_matches(|c: char| !c.is_ascii_digit()).parse::<f64>() {
            return DslValue::SizeBytes((n * 1024.0) as u64);
        }
    }
    if val.ends_with("MB") || val.ends_with("mb") {
        if let Ok(n) = val.trim_end_matches(|c: char| !c.is_ascii_digit()).parse::<f64>() {
            return DslValue::SizeBytes((n * 1024.0 * 1024.0) as u64);
        }
    }
    DslValue::String(val.to_string())
}

fn parse_or(tokens: &[Token], pos: usize) -> Result<(DslExpr, usize), DslParseError> {
    let (mut expr, mut pos) = parse_and(tokens, pos)?;
    while pos < tokens.len() && matches!(tokens[pos], Token::Or) {
        pos += 1;
        let (right, new_pos) = parse_and(tokens, pos)?;
        expr = DslExpr::Or(Box::new(expr), Box::new(right));
        pos = new_pos;
    }
    Ok((expr, pos))
}

fn parse_and(tokens: &[Token], pos: usize) -> Result<(DslExpr, usize), DslParseError> {
    let (mut expr, mut pos) = parse_not(tokens, pos)?;
    while pos < tokens.len() && matches!(tokens[pos], Token::And) {
        pos += 1;
        let (right, new_pos) = parse_not(tokens, pos)?;
        expr = DslExpr::And(Box::new(expr), Box::new(right));
        pos = new_pos;
    }
    Ok((expr, pos))
}

fn parse_not(tokens: &[Token], pos: usize) -> Result<(DslExpr, usize), DslParseError> {
    if pos < tokens.len() && matches!(tokens[pos], Token::Not) {
        let pos = pos + 1;
        let (expr, new_pos) = parse_primary(tokens, pos)?;
        Ok((DslExpr::Not(Box::new(expr)), new_pos))
    } else {
        parse_primary(tokens, pos)
    }
}

fn parse_primary(tokens: &[Token], pos: usize) -> Result<(DslExpr, usize), DslParseError> {
    if pos >= tokens.len() {
        return Err(DslParseError { message: "意外的表达式结尾".to_string(), position: pos });
    }
    match &tokens[pos] {
        Token::LParen => {
            let (expr, pos) = parse_or(tokens, pos + 1)?;
            if pos >= tokens.len() || !matches!(tokens[pos], Token::RParen) {
                return Err(DslParseError { message: "缺少右括号".to_string(), position: pos });
            }
            Ok((expr, pos + 1))
        }
        Token::FieldMatch { field, op, value } => {
            Ok((DslExpr::FieldMatch { field: field.clone(), op: *op, value: value.clone() }, pos + 1))
        }
        _ => Err(DslParseError {
            message: format!("意外的标记: {:?}", tokens[pos]),
            position: pos,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_field() {
        let expr = DslParser::parse("method:GET").unwrap();
        assert!(matches!(expr, DslExpr::FieldMatch { field: DslField::Method, op: DslOp::Contains, .. }));
    }

    #[test]
    fn test_and_expression() {
        let expr = DslParser::parse("method:GET AND host:example.com").unwrap();
        assert!(matches!(expr, DslExpr::And(_, _)));
    }

    #[test]
    fn test_or_expression() {
        let expr = DslParser::parse("method:GET OR method:POST").unwrap();
        assert!(matches!(expr, DslExpr::Or(_, _)));
    }

    #[test]
    fn test_not_expression() {
        let expr = DslParser::parse("NOT status:404").unwrap();
        assert!(matches!(expr, DslExpr::Not(_)));
    }

    #[test]
    fn test_parenthesized() {
        let expr = DslParser::parse("(method:GET OR method:POST) AND host:api.com").unwrap();
        assert!(matches!(expr, DslExpr::And(_, _)));
    }

    #[test]
    fn test_status_range() {
        let expr = DslParser::parse("status:200..299").unwrap();
        if let DslExpr::FieldMatch { value: DslValue::Range(lo, hi), .. } = expr {
            assert_eq!(lo, 200.0);
            assert_eq!(hi, 299.0);
        } else {
            panic!("Expected Range value");
        }
    }

    #[test]
    fn test_regex() {
        let expr = DslParser::parse("url:~/api/v[0-9]+").unwrap();
        assert!(matches!(expr, DslExpr::FieldMatch { op: DslOp::Regex, .. }));
    }

    #[test]
    fn test_greater_than() {
        let expr = DslParser::parse("duration:>1000").unwrap();
        if let DslExpr::FieldMatch { op: DslOp::GreaterThan, value: DslValue::Number(n), .. } = expr {
            assert_eq!(n, 1000.0);
        } else {
            panic!("Expected GreaterThan with Number");
        }
    }

    #[test]
    fn test_bare_text() {
        let expr = DslParser::parse("example.com").unwrap();
        assert!(matches!(expr, DslExpr::FieldMatch { field: DslField::Url, op: DslOp::Contains, .. }));
    }

    #[test]
    fn test_header_field() {
        let expr = DslParser::parse("header.authorization:Bearer").unwrap();
        if let DslExpr::FieldMatch { field: DslField::Header(name), .. } = expr {
            assert_eq!(name, "authorization");
        } else {
            panic!("Expected Header field");
        }
    }

    #[test]
    fn test_empty_expression() {
        assert!(DslParser::parse("").is_err());
    }

    #[test]
    fn test_complex_expression() {
        let expr = DslParser::parse("method:POST AND host:api.example.com AND NOT status:500..599").unwrap();
        assert!(matches!(expr, DslExpr::And(_, _)));
    }
}
