use std::str;

use profiler_macro::instrument;

#[derive(Debug, PartialEq)]
pub enum JsonValue<'a> {
    Object{ pairs: Vec<(&'a str, JsonValue<'a>)> },
    Array{ elements: Vec<JsonValue<'a>> },
    String(&'a str),
    Number(f64),
    Boolean(bool),
    Null,
}

#[derive(Debug, PartialEq)]
enum JsonToken<'a> {
    CurlyStart,
    CurlyEnd,
    Colon,
    SquareStart,
    SquareEnd,
    String(&'a str),
    Number(f64),
    Boolean(bool),
    Null,
}

impl<'a> JsonToken<'a> {
    fn parse_token(data: &'a [u8]) -> (Self, usize) {
        let mut ptr = 0;

        while data[ptr].is_ascii_whitespace() || data[ptr] == b',' {
            ptr += 1 ;
        }

        match data[ptr] {
            b'{' => (JsonToken::CurlyStart, ptr + 1),
            b'}' => (JsonToken::CurlyEnd, ptr + 1),

            b'[' => (JsonToken::SquareStart, ptr + 1),
            b']' => (JsonToken::SquareEnd, ptr + 1),

            b':' => (JsonToken::Colon, ptr + 1),

            b'"' => {
                let size = data[ptr + 1..].iter().position(|x| *x == b'"').expect("Expected closing quote for JSON string");

                let s = unsafe { str::from_utf8_unchecked(&data[ptr + 1..ptr + 1 + size]) };
                (JsonToken::String(s), ptr + 2 + size)
            }
            x if x.is_ascii_digit() || x == b'-' => {
                let mut num_size = 0;

                if x == b'-' {
                    num_size += 1;
                }

                num_size += data[ptr + num_size..].iter().take_while(|x| x.is_ascii_digit()).count();

                if data.len() > ptr + num_size && data[ptr + num_size] == b'.' {
                    num_size += 1;
                    num_size += data[ptr + num_size..].iter().take_while(|x| x.is_ascii_digit()).count();
                }

                let num_str = unsafe {
                    str::from_utf8_unchecked(&data[ptr..ptr + num_size])
                };
                let num = num_str.parse().unwrap_or_else(|_| panic!("Couldn't parse '{num_str}' as f64"));

                (JsonToken::Number(num), ptr + num_size)
            }
            b't' => {
                if data[ptr..ptr + 4] == *b"true" {
                    (JsonToken::Boolean(true), ptr + 4)
                } else {
                    panic!("Expected JSON token starting with 't' to be 'true'");
                }
            },

            b'f' => {
                if data[ptr..ptr + 5] == *b"false" {
                    (JsonToken::Boolean(false), ptr + 5)
                } else {
                    panic!("Expected JSON token starting with 'f' to be 'false'");
                }
            }

            b'n' => {
                if data[ptr..ptr + 4] == *b"null" {
                    (JsonToken::Null, ptr + 4)
                } else {
                    panic!("Expected JSON token starting with 'n' to be 'null'");
                }
            }
            _ => panic!("Unexpected JSON token '{}...'", data[ptr..].iter().take(25).map(|x| *x as char).collect::<String>()),
        }
    }
}

impl<'a> JsonValue<'a> {
    #[instrument]
    pub fn parse(data: &'a str) -> Self {
        Self::parse_rec(data.as_bytes()).0
    }

    fn parse_rec(data: &'a [u8]) -> (Self, &'a[u8]) {
        let (token, ptr) = JsonToken::parse_token(data);
        let mut data = &data[ptr..];
        
        let res = match token {
            JsonToken::CurlyStart => {
                let mut pairs = Vec::new();
                loop {
                    let (curr, ptr) = JsonToken::parse_token(data);
                    data = &data[ptr..];

                    let key = match curr {
                        JsonToken::String(s) => s,
                        JsonToken::CurlyEnd => break,
                        _ => panic!("Found non-string object key!")
                    };

                    let (curr, ptr) = JsonToken::parse_token(data);
                    data = &data[ptr..];

                    assert_eq!(curr, JsonToken::Colon, "Expected colon between kv pair");
                    
                    let (val, d) = Self::parse_rec(data);
                    data = d;

                    pairs.push((key, val));
                };


                JsonValue::Object { pairs }
            },
            JsonToken::SquareStart => {
                let mut elements = Vec::new();
                loop {
                    let (curr, ptr) = JsonToken::parse_token(data);
                    if curr == JsonToken::SquareEnd {
                        data = &data[ptr..];
                        break;
                    }

                    let (element, d) = Self::parse_rec(data);
                    data = d;

                    elements.push(element);
                };

                JsonValue::Array { elements }
            },
            JsonToken::Number(n) => JsonValue::Number(n),
            JsonToken::String(s) => JsonValue::String(s),
            JsonToken::Boolean(b) => JsonValue::Boolean(b),
            JsonToken::Null => JsonValue::Null,
            _ => panic!("Unexpected token {token:?}"),
        };

        (res, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use JsonValue::*;

    #[test]
    fn test_parse_null() {
        assert_eq!(JsonValue::parse("null"), Null);
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(JsonValue::parse("true"), Boolean(true));
        assert_eq!(JsonValue::parse("false"), Boolean(false));
 
    }

    #[test]
    fn test_parse_string() {
        assert_eq!(JsonValue::parse("\"hello world\""), String("hello world"));
    }

    #[test]
    fn test_parse_num() {
        assert_eq!(JsonValue::parse("12345.12345"), Number(12345.12345));
        assert_eq!(JsonValue::parse("10"), Number(10.0));
        assert_eq!(JsonValue::parse("-100"), Number(-100.0));
        assert_eq!(JsonValue::parse("-3.1415"), Number(-3.1415));
    }

    #[test]
    fn test_parse_array() {
        let arr = Array { elements: vec![Null, Boolean(true), Number(1.2), String("hello")] };
        assert_eq!(JsonValue::parse("[null, true, 1.2, \"hello\"]"), arr);
    }

    #[test]
    fn test_parse_object() {
        let json = r#"{
            "name": "Bob",
            "age": 24,
            "happy": true,
            "wife": null
        }"#;

        let expected = Object { pairs: vec![
            ("name", String("Bob")),
            ("age", Number(24.0)),
            ("happy", Boolean(true)),
            ("wife", Null),
        ] };

        assert_eq!(JsonValue::parse(json), expected);
    }

    #[test]
    fn test_parse_nested() {
        let json = r#"{
            "name": "Bob",
            "age": 24,
            "happy": true,
            "cars": [
                {
                    "size": "big"
                },
                {
                    "size": "smallish"
                }  
            ] 
        }"#;

        let expected = Object { pairs: vec![
            ("name", String("Bob")),
            ("age", Number(24.0)),
            ("happy", Boolean(true)),
            ("cars", Array { elements: vec![
                Object { pairs: vec![
                    ("size", String("big"))
                ] },
                Object { pairs: vec![
                    ("size", String("smallish"))
                ] }
            ]
            }),
        ] };

        assert_eq!(JsonValue::parse(json), expected);
    }
}
