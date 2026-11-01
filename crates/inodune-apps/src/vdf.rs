use std::collections::BTreeMap;
#[derive(Debug)]
pub(crate) enum Value {
    Text(String),
    Object(BTreeMap<String, Value>),
}
impl Value {
    pub fn text(&self) -> Option<&str> {
        if let Self::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }
    pub fn object(&self) -> Option<&BTreeMap<String, Value>> {
        if let Self::Object(o) = self {
            Some(o)
        } else {
            None
        }
    }
}
pub(crate) fn parse(text: &str) -> Result<BTreeMap<String, Value>, String> {
    if text.len() > crate::files::MAX_FILE {
        return Err("VDF excede limite".into());
    }
    struct Parser<'a> {
        bytes: &'a [u8],
        at: usize,
        nodes: usize,
    }
    impl Parser<'_> {
        fn skip(&mut self) {
            loop {
                while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
                    self.at += 1;
                }
                if self.bytes.get(self.at..self.at + 2) == Some(b"//") {
                    while self.bytes.get(self.at).is_some_and(|b| *b != b'\n') {
                        self.at += 1;
                    }
                } else {
                    break;
                }
            }
        }
        fn string(&mut self) -> Result<String, String> {
            self.skip();
            if self.bytes.get(self.at) != Some(&b'"') {
                return Err("String VDF esperada".into());
            }
            self.at += 1;
            let mut result = vec![];
            loop {
                let b = *self.bytes.get(self.at).ok_or("String VDF incompleta")?;
                self.at += 1;
                match b {
                    b'"' => return String::from_utf8(result).map_err(|_| "UTF-8 inválido".into()),
                    b'\\' => {
                        let b = *self.bytes.get(self.at).ok_or("Escape incompleto")?;
                        self.at += 1;
                        result.push(match b {
                            b'\\' | b'"' => b,
                            b'n' => b'\n',
                            b't' => b'\t',
                            _ => return Err("Escape VDF inválido".into()),
                        });
                    }
                    0..=31 => return Err("Controle VDF inválido".into()),
                    _ => result.push(b),
                }
                if result.len() > 16384 {
                    return Err("String VDF enorme".into());
                }
            }
        }
        fn object(&mut self, depth: usize) -> Result<BTreeMap<String, Value>, String> {
            if depth > 16 {
                return Err("Profundidade VDF excedida".into());
            }
            let mut result = BTreeMap::new();
            loop {
                self.skip();
                match self.bytes.get(self.at) {
                    None if depth == 0 => return Ok(result),
                    Some(b'}') if depth > 0 => {
                        self.at += 1;
                        return Ok(result);
                    }
                    None | Some(b'}') => return Err("Chaves VDF desbalanceadas".into()),
                    _ => {}
                }
                let key = self.string()?;
                self.skip();
                self.nodes += 1;
                if self.nodes > 30000 {
                    return Err("Muitos nós VDF".into());
                }
                let value = if self.bytes.get(self.at) == Some(&b'{') {
                    self.at += 1;
                    Value::Object(self.object(depth + 1)?)
                } else {
                    Value::Text(self.string()?)
                };
                if result.insert(key, value).is_some() {
                    return Err("Chave VDF duplicada".into());
                }
            }
        }
    }
    Parser {
        bytes: text.as_bytes(),
        at: 0,
        nodes: 0,
    }
    .object(0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_comments_unicode_escapes_and_nested() {
        assert!(parse("// comment\n\"root\" { \"name\" \"Jogo ç \\\"X\\\"\" }").is_ok());
    }
    #[test]
    fn refuses_malformed_huge_deep_duplicate() {
        for s in [
            "\"a\" {",
            "\"a\" \"b",
            "}",
            "\"a\" \"b\" \"a\" \"c\"",
            "\"a\" \"b\" garbage",
        ] {
            assert!(parse(s).is_err());
        }
        assert!(parse(&" ".repeat(crate::files::MAX_FILE + 1)).is_err());
        assert!(parse(&format!("{}{}", "\"x\" {".repeat(18), "}".repeat(18))).is_err());
    }
}
