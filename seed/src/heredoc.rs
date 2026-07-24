//! Heredoc expansion (ADR 0020): squiggly `<<~SQL` only.
//!
//! Runs before the lexer, rewriting each heredoc into an ordinary string
//! literal on the opener's line and dropping the body lines. Crude on
//! purpose: seed tokens borrow the source, and a dedented heredoc body is
//! not a slice of it — so the rewrite happens in source, once, up front.
//!
//! `<<EOS` and `<<-EOS` are deliberately absent, which is what keeps `<<`
//! unambiguously the append operator (ADR 0015).

/// Rewrite every heredoc in `source` into an inline string literal.
pub fn expand(source: &str) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut output: Vec<String> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let (rewritten, consumed) = expand_line(&lines, index);
        output.push(rewritten);
        index += 1 + consumed;
    }
    output.join("\n")
}

/// Rewrite one line, returning it plus how many following lines its
/// heredoc bodies swallowed.
fn expand_line(lines: &[&str], index: usize) -> (String, usize) {
    let line = lines[index];
    let bytes = line.as_bytes();
    let mut result = String::with_capacity(line.len());
    let mut consumed = 0;
    let mut position = 0;
    while position < bytes.len() {
        let character = bytes[position];
        // Comments end the line; a `<<~` inside one is just prose.
        if character == b'#' {
            result.push_str(&line[position..]);
            return (result, consumed);
        }
        if character == b'"' || character == b'\'' {
            let end = skip_string(bytes, position);
            result.push_str(&line[position..end]);
            position = end;
            continue;
        }
        if let Some((terminator, interpolating, opener_length)) = read_opener(line, position) {
            let (body, used) = read_body(lines, index + 1 + consumed, &terminator);
            consumed += used;
            result.push_str(&encode(&dedent(&body), interpolating));
            position += opener_length;
            continue;
        }
        result.push(character as char);
        position += 1;
    }
    (result, consumed)
}

/// Walk past a string literal so its contents can't be mistaken for code.
fn skip_string(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let mut position = start + 1;
    while position < bytes.len() {
        match bytes[position] {
            b'\\' => position += 2,
            character if character == quote => return position + 1,
            _ => position += 1,
        }
    }
    bytes.len()
}

/// A `<<~TERMINATOR` opener at `position`, if one starts there.
fn read_opener(line: &str, position: usize) -> Option<(String, bool, usize)> {
    let rest = &line[position..];
    let after = rest.strip_prefix("<<~")?;
    let (quote, name_start) = match after.as_bytes().first() {
        Some(b'\'') => (Some('\''), 1),
        Some(b'"') => (Some('"'), 1),
        _ => (None, 0),
    };
    let name: String = after[name_start..]
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect();
    if name.is_empty() {
        // `<<~ SQL` and friends: Ruby rejects these too.
        panic!("expected a heredoc terminator right after `<<~`, with no space");
    }
    if !name.starts_with(|character: char| character.is_ascii_uppercase())
        || name.chars().any(|character| character.is_ascii_lowercase())
    {
        panic!(
            "heredoc terminator {name} must be SCREAMING_CAPS — write {}",
            name.to_uppercase()
        );
    }
    let mut length = 3 + name_start + name.len();
    if let Some(quote) = quote {
        if !after[name_start + name.len()..].starts_with(quote) {
            panic!("unterminated heredoc terminator quote in `<<~{quote}{name}`");
        }
        length += 1;
    }
    Some((name, quote != Some('\''), length))
}

/// Body lines from `start` up to the terminator line, which is consumed.
fn read_body(lines: &[&str], start: usize, terminator: &str) -> (Vec<String>, usize) {
    let mut body = Vec::new();
    let mut index = start;
    while index < lines.len() {
        if lines[index].trim() == terminator {
            return (body, index - start + 1);
        }
        body.push(lines[index].to_string());
        index += 1;
    }
    panic!("unterminated heredoc — expected a line reading {terminator}");
}

/// Strip the indentation common to every non-blank line, keeping relative
/// structure. This is the whole point of the squiggly form.
fn dedent(body: &[String]) -> Vec<String> {
    let common = body
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    body.iter()
        .map(|line| {
            if line.len() < common {
                String::new()
            } else {
                line[common..].to_string()
            }
        })
        .collect()
}

/// Encode the body as a string literal the lexer already understands.
/// Interpolating heredocs become double-quoted (so `#{...}` and escapes
/// keep working); `<<~'SQL'` becomes single-quoted, where neither does.
fn encode(body: &[String], interpolating: bool) -> String {
    let mut result = String::new();
    if interpolating {
        result.push('"');
        for line in body {
            result.push_str(&line.replace('"', "\\\""));
            result.push_str("\\n");
        }
        result.push('"');
    } else {
        result.push('\'');
        for line in body {
            result.push_str(&line.replace('\\', "\\\\").replace('\'', "\\'"));
            result.push('\n');
        }
        result.push('\'');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::expand;

    #[test]
    fn strips_the_common_indentation_and_keeps_relative_structure() {
        let source = "db = <<~SQL\n  select *\n    from orders\nSQL\n";
        assert_eq!(expand(source), "db = \"select *\\n  from orders\\n\"\n");
    }

    #[test]
    fn leaves_interpolation_for_the_parser() {
        let source = "x = <<~TEXT\n  hi #{name}\nTEXT\n";
        assert_eq!(expand(source), "x = \"hi #{name}\\n\"\n");
    }

    /// A single-quoted terminator suppresses interpolation, so the body
    /// becomes a single-quoted literal where `#{...}` means nothing.
    #[test]
    fn single_quoted_terminators_do_not_interpolate() {
        let source = "x = <<~'TEXT'\n  hi #{name}\nTEXT\n";
        assert_eq!(expand(source), "x = 'hi #{name}\n'\n");
    }

    #[test]
    fn keeps_the_rest_of_the_opener_line() {
        let source = "x = <<~TEXT.upcase\n  shout\nTEXT\n";
        assert_eq!(expand(source), "x = \"shout\\n\".upcase\n");
    }

    #[test]
    fn allows_an_indented_terminator() {
        let source = "def show\n  x = <<~TEXT\n    hi\n  TEXT\nend\n";
        assert_eq!(expand(source), "def show\n  x = \"hi\\n\"\nend\n");
    }

    #[test]
    fn expands_two_heredocs_opened_on_one_line() {
        let source = "pair = [<<~A, <<~B]\n  first\nA\n  second\nB\n";
        assert_eq!(expand(source), "pair = [\"first\\n\", \"second\\n\"]\n");
    }

    /// `<<` stays the append operator (ADR 0015) — that is the whole point
    /// of shipping only the squiggly opener.
    #[test]
    fn leaves_the_append_operator_alone() {
        assert_eq!(expand("list << 2\n"), "list << 2\n");
    }

    #[test]
    fn ignores_a_heredoc_looking_thing_in_a_comment() {
        assert_eq!(expand("# see <<~FAKE\n"), "# see <<~FAKE\n");
    }

    #[test]
    fn ignores_a_heredoc_looking_thing_inside_a_string() {
        assert_eq!(expand("x = \"<<~FAKE\"\n"), "x = \"<<~FAKE\"\n");
    }

    #[test]
    #[should_panic(expected = "must be SCREAMING_CAPS")]
    fn panics_on_a_lowercase_terminator() {
        expand("x = <<~sql\n  a\nsql\n");
    }

    #[test]
    #[should_panic(expected = "with no space")]
    fn panics_on_a_space_after_the_squiggle() {
        expand("x = <<~ SQL\n  a\nSQL\n");
    }

    #[test]
    #[should_panic(expected = "unterminated heredoc")]
    fn panics_when_the_terminator_never_arrives() {
        expand("x = <<~SQL\n  a\n");
    }
}
