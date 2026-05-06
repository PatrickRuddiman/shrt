use std::env;
use std::fmt::Write as FmtWrite;
use std::io::{self, Read, Write};
use std::process;

fn main() {
    let exit_code: i32 = env::var("EXIT_CODE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let argv: Vec<String> = env::args().skip(1).collect();

    let stdin_capture = if env::var("READ_STDIN").as_deref() == Ok("1") {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).ok();
        Some(buf)
    } else {
        None
    };

    let mut out = String::from("{\"argv\":");
    out.push_str(&render_array(&argv));
    if let Some(s) = &stdin_capture {
        out.push_str(",\"stdin\":");
        out.push_str(&render_string(s));
    }
    out.push('}');

    let _ = writeln!(io::stdout(), "{}", out);
    let _ = io::stdout().flush();

    process::exit(exit_code);
}

fn render_array(items: &[String]) -> String {
    let mut s = String::from("[");
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&render_string(item));
    }
    s.push(']');
    s
}

fn render_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_array_renders() {
        let v: Vec<String> = vec![];
        assert_eq!(render_array(&v), "[]");
    }

    #[test]
    fn array_with_strings() {
        let v = vec!["a".to_string(), "two words".to_string()];
        assert_eq!(render_array(&v), "[\"a\",\"two words\"]");
    }

    #[test]
    fn escapes_double_quote() {
        assert_eq!(render_string("\""), "\"\\\"\"");
    }

    #[test]
    fn escapes_backslash() {
        assert_eq!(render_string("\\"), "\"\\\\\"");
    }

    #[test]
    fn escapes_newline_and_tab() {
        assert_eq!(render_string("\n"), "\"\\n\"");
        assert_eq!(render_string("\t"), "\"\\t\"");
    }

    #[test]
    fn escapes_low_control_char() {
        assert_eq!(render_string("\u{0001}"), "\"\\u0001\"");
    }

    #[test]
    fn plain_string_passes_through() {
        assert_eq!(render_string("hello world"), "\"hello world\"");
    }
}
