//! Name formats: free text with placeholders, like `{diff} {rate}`, that the
//! trainer and the pack creator fill to name the diffs they make.
//!
//! A placeholder is `{name}` with a name of `fields`; anything else is text,
//! unknown `{...}` included. Empty placeholders vanish, and so do the `(...)`
//! and `[...]` around them when every placeholder inside is empty: `({bpm}bpm)`
//! without a BPM. Spaces are tidied after.

/// A part of a name format.
enum Part {
    Text(String),
    Field(String),
    /// `(...)` or `[...]`.
    Group(char, Vec<Part>, char),
}

fn parse(format: &str, fields: &[&str]) -> Vec<Part> {
    // The groups being read, innermost last; the bottom one is the whole format.
    let mut stack: Vec<(Option<char>, Vec<Part>)> = vec![(None, Vec::new())];
    let push_text = |parts: &mut Vec<Part>, text: &str| match parts.last_mut() {
        Some(Part::Text(last)) => last.push_str(text),
        _ => parts.push(Part::Text(text.to_string())),
    };

    let mut chars = format.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let closes = match c {
            ')' => Some('('),
            ']' => Some('['),
            _ => None,
        };
        match c {
            '(' | '[' => stack.push((Some(c), Vec::new())),
            _ if closes.is_some() && stack.len() > 1 && stack.last().unwrap().0 == closes => {
                let (open, parts) = stack.pop().unwrap();
                stack.last_mut().unwrap().1.push(Part::Group(open.unwrap(), parts, c));
            }
            '{' => {
                let rest = &format[i + 1..];
                match rest.find('}').map(|end| &rest[..end]).filter(|name| fields.contains(name)) {
                    Some(name) => {
                        stack.last_mut().unwrap().1.push(Part::Field(name.to_string()));
                        for _ in 0..=name.len() {
                            chars.next();
                        }
                    }
                    // Not a placeholder: the user's own text.
                    None => push_text(&mut stack.last_mut().unwrap().1, "{"),
                }
            }
            _ => push_text(&mut stack.last_mut().unwrap().1, c.encode_utf8(&mut [0; 4])),
        }
    }

    // A group that is never closed was text.
    while stack.len() > 1 {
        let (open, parts) = stack.pop().unwrap();
        let parent = &mut stack.last_mut().unwrap().1;
        push_text(parent, open.unwrap().encode_utf8(&mut [0; 4]));
        for part in parts {
            match part {
                Part::Text(text) => push_text(parent, &text),
                other => parent.push(other),
            }
        }
    }
    stack.pop().unwrap().1
}

/// The text of `parts`, how many placeholders they have and how many of
/// those are filled.
fn render_parts(parts: &[Part], value: &dyn Fn(&str) -> Option<String>) -> (String, usize, usize) {
    let mut out = String::new();
    let (mut fields, mut filled) = (0, 0);
    for part in parts {
        match part {
            Part::Text(text) => out.push_str(text),
            Part::Field(name) => {
                fields += 1;
                if let Some(v) = value(name) {
                    filled += 1;
                    out.push_str(&v);
                }
            }
            Part::Group(open, inner, close) => {
                let (text, f, ok) = render_parts(inner, value);
                fields += f;
                filled += ok;
                if f == 0 || ok > 0 {
                    out.push(*open);
                    out.push_str(&text);
                    out.push(*close);
                }
            }
        }
    }
    (out, fields, filled)
}

/// `format` with its placeholders, the names in `fields`, filled by `value`.
pub fn render(format: &str, fields: &[&str], value: &dyn Fn(&str) -> Option<String>) -> String {
    let (text, _, _) = render_parts(&parse(format, fields), value);

    // One space between words, and none right inside the brackets that are left.
    let mut out = String::new();
    for word in text.split_whitespace() {
        if !out.is_empty() && !out.ends_with(['(', '[']) && !word.starts_with([')', ']']) {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}
