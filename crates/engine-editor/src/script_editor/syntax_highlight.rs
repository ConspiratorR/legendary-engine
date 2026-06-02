use egui::text::LayoutJob;
use egui::{Color32, FontId, TextFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptLanguage {
    Lua,
    Wasm,
}

pub fn highlight(code: &str, language: ScriptLanguage, font_id: FontId) -> LayoutJob {
    match language {
        ScriptLanguage::Lua => highlight_lua(code, font_id),
        ScriptLanguage::Wasm => highlight_wasm(code, font_id),
    }
}

fn color_for_token(kind: TokenKind) -> Color32 {
    match kind {
        TokenKind::Keyword => Color32::from_rgb(204, 120, 220),
        TokenKind::String => Color32::from_rgb(152, 195, 121),
        TokenKind::Number => Color32::from_rgb(209, 154, 102),
        TokenKind::Comment => Color32::from_rgb(92, 130, 102),
        TokenKind::Operator => Color32::from_rgb(198, 120, 82),
        TokenKind::Builtin => Color32::from_rgb(86, 182, 194),
        TokenKind::Normal => Color32::from_rgb(212, 212, 216),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Keyword,
    String,
    Number,
    Comment,
    Operator,
    Builtin,
    Normal,
}

fn push_segment(job: &mut LayoutJob, text: &str, kind: TokenKind, font_id: &FontId) {
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font_id.clone(),
            color: color_for_token(kind),
            ..Default::default()
        },
    );
}

fn highlight_lua(code: &str, font_id: FontId) -> LayoutJob {
    let mut job = LayoutJob::default();
    let keywords: &[&str] = &[
        "function", "end", "if", "then", "else", "elseif", "for", "while", "do", "return", "local",
        "and", "or", "not", "true", "false", "nil", "in", "repeat", "until", "break", "goto",
    ];
    let builtins: &[&str] = &[
        "print",
        "type",
        "tostring",
        "tonumber",
        "error",
        "assert",
        "pcall",
        "xpcall",
        "require",
        "module",
        "setmetatable",
        "getmetatable",
        "rawget",
        "rawset",
        "rawequal",
        "rawlen",
        "select",
        "ipairs",
        "pairs",
        "next",
        "load",
        "loadstring",
        "dofile",
        "collectgarbage",
        "table",
        "string",
        "math",
        "io",
        "os",
        "coroutine",
        "debug",
        "utf8",
    ];

    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Block comment --[[ ... ]]
        if i + 3 < len
            && chars[i] == '-'
            && chars[i + 1] == '-'
            && chars[i + 2] == '['
            && chars[i + 3] == '['
        {
            let start = i;
            i += 4;
            while i + 1 < len {
                if chars[i] == ']' && chars[i + 1] == ']' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            push_segment(&mut job, &s, TokenKind::Comment, &font_id);
            continue;
        }

        // Line comment --
        if i + 1 < len && chars[i] == '-' && chars[i + 1] == '-' {
            let start = i;
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            push_segment(&mut job, &s, TokenKind::Comment, &font_id);
            continue;
        }

        // String "..." or '...'
        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            let start = i;
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < len {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            push_segment(&mut job, &s, TokenKind::String, &font_id);
            continue;
        }

        // Long string [[ ... ]]
        if i + 1 < len && chars[i] == '[' && chars[i + 1] == '[' {
            let start = i;
            i += 2;
            while i + 1 < len {
                if chars[i] == ']' && chars[i + 1] == ']' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            push_segment(&mut job, &s, TokenKind::String, &font_id);
            continue;
        }

        // Number
        if chars[i].is_ascii_digit()
            || (chars[i] == '.' && i + 1 < len && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            if i + 1 < len && chars[i] == '0' && (chars[i + 1] == 'x' || chars[i + 1] == 'X') {
                i += 2;
                while i < len && (chars[i].is_ascii_hexdigit() || chars[i] == '_') {
                    i += 1;
                }
            } else {
                while i < len
                    && (chars[i].is_ascii_digit()
                        || chars[i] == '.'
                        || chars[i] == '_'
                        || chars[i] == 'e'
                        || chars[i] == 'E'
                        || chars[i] == '+'
                        || chars[i] == '-')
                {
                    i += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            push_segment(&mut job, &s, TokenKind::Number, &font_id);
            continue;
        }

        // Identifier or keyword
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let kind = if keywords.contains(&word.as_str()) {
                TokenKind::Keyword
            } else if builtins.contains(&word.as_str()) {
                TokenKind::Builtin
            } else {
                TokenKind::Normal
            };
            push_segment(&mut job, &word, kind, &font_id);
            continue;
        }

        // Operators
        if "+-*/%^#=<>~".contains(chars[i]) {
            let op: String = chars[i..=i].iter().collect();
            push_segment(&mut job, &op, TokenKind::Operator, &font_id);
            i += 1;
            continue;
        }

        // Everything else (whitespace, punctuation)
        let s: String = chars[i..=i].iter().collect();
        push_segment(&mut job, &s, TokenKind::Normal, &font_id);
        i += 1;
    }

    job
}

fn highlight_wasm(code: &str, font_id: FontId) -> LayoutJob {
    let mut job = LayoutJob::default();
    let keywords: &[&str] = &[
        "module",
        "func",
        "param",
        "result",
        "local",
        "export",
        "import",
        "memory",
        "data",
        "type",
        "table",
        "elem",
        "start",
        "global",
        "mut",
        "offset",
        "block",
        "loop",
        "if",
        "then",
        "else",
        "end",
        "br",
        "br_if",
        "br_table",
        "return",
        "call",
        "call_indirect",
        "drop",
        "select",
        "unreachable",
        "nop",
    ];
    let builtins: &[&str] = &[
        "i32.const",
        "i64.const",
        "f32.const",
        "f64.const",
        "i32.add",
        "i32.sub",
        "i32.mul",
        "i32.div_s",
        "i32.div_u",
        "i32.rem_s",
        "i32.rem_u",
        "i32.and",
        "i32.or",
        "i32.xor",
        "i32.shl",
        "i32.shr_s",
        "i32.shr_u",
        "i32.rotl",
        "i32.rotr",
        "i32.eqz",
        "i32.eq",
        "i32.ne",
        "i32.lt_s",
        "i32.lt_u",
        "i32.gt_s",
        "i32.gt_u",
        "i32.le_s",
        "i32.le_u",
        "i32.ge_s",
        "i32.ge_u",
        "i32.wrap_i64",
        "i32.trunc_f32_s",
        "i32.trunc_f32_u",
        "i32.trunc_f64_s",
        "i32.trunc_f64_u",
        "i32.reinterpret_f32",
        "i64.add",
        "i64.sub",
        "i64.mul",
        "i64.div_s",
        "i64.div_u",
        "i64.rem_s",
        "i64.rem_u",
        "i64.and",
        "i64.or",
        "i64.xor",
        "i64.shl",
        "i64.shr_s",
        "i64.shr_u",
        "i64.rotl",
        "i64.rotr",
        "i64.eqz",
        "i64.eq",
        "i64.ne",
        "i64.lt_s",
        "i64.lt_u",
        "i64.gt_s",
        "i64.gt_u",
        "i64.le_s",
        "i64.le_u",
        "i64.ge_s",
        "i64.ge_u",
        "i64.extend_i32_s",
        "i64.extend_i32_u",
        "i64.trunc_f32_s",
        "i64.trunc_f32_u",
        "i64.trunc_f64_s",
        "i64.trunc_f64_u",
        "f32.add",
        "f32.sub",
        "f32.mul",
        "f32.div",
        "f32.abs",
        "f32.neg",
        "f32.copysign",
        "f32.ceil",
        "f32.floor",
        "f32.trunc",
        "f32.nearest",
        "f32.sqrt",
        "f32.min",
        "f32.max",
        "f32.eq",
        "f32.ne",
        "f32.lt",
        "f32.gt",
        "f32.le",
        "f32.ge",
        "f32.convert_i32_s",
        "f32.convert_i32_u",
        "f32.convert_i64_s",
        "f32.convert_i64_u",
        "f32.reinterpret_i32",
        "f32.demote_f64",
        "f64.add",
        "f64.sub",
        "f64.mul",
        "f64.div",
        "f64.abs",
        "f64.neg",
        "f64.copysign",
        "f64.ceil",
        "f64.floor",
        "f64.trunc",
        "f64.nearest",
        "f64.sqrt",
        "f64.min",
        "f64.max",
        "f64.eq",
        "f64.ne",
        "f64.lt",
        "f64.gt",
        "f64.le",
        "f64.ge",
        "f64.convert_i32_s",
        "f64.convert_i32_u",
        "f64.convert_i64_s",
        "f64.convert_i64_u",
        "f64.promote_f32",
        "f64.reinterpret_i64",
        "i32.load",
        "i64.load",
        "f32.load",
        "f64.load",
        "i32.store",
        "i64.store",
        "f32.store",
        "f64.store",
        "i32.load8_s",
        "i32.load8_u",
        "i32.load16_s",
        "i32.load16_u",
        "i64.load8_s",
        "i64.load8_u",
        "i64.load16_s",
        "i64.load16_u",
        "i64.load32_s",
        "i64.load32_u",
        "i32.store8",
        "i32.store16",
        "i64.store8",
        "i64.store16",
        "i64.store32",
        "memory.size",
        "memory.grow",
        "memory.copy",
        "memory.fill",
        "global.get",
        "global.set",
        "local.get",
        "local.set",
        "local.tee",
        "table.get",
        "table.set",
        "table.size",
        "table.grow",
        "table.copy",
        "table.init",
        "elem.drop",
        "data.drop",
        "memory.copy",
        "memory.fill",
        "memory.init",
    ];

    let chars: Vec<char> = code.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Line comment ;;
        if i + 1 < len && chars[i] == ';' && chars[i + 1] == ';' {
            let start = i;
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            push_segment(&mut job, &s, TokenKind::Comment, &font_id);
            continue;
        }

        // Block comment (; ... ;)
        if i + 1 < len && chars[i] == '(' && chars[i + 1] == ';' {
            let start = i;
            i += 2;
            let mut depth = 1;
            while i + 1 < len && depth > 0 {
                if chars[i] == '(' && chars[i + 1] == ';' {
                    depth += 1;
                    i += 2;
                } else if chars[i] == ';' && chars[i + 1] == ')' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            push_segment(&mut job, &s, TokenKind::Comment, &font_id);
            continue;
        }

        // String "..."
        if chars[i] == '"' {
            let start = i;
            i += 1;
            while i < len && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < len {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            push_segment(&mut job, &s, TokenKind::String, &font_id);
            continue;
        }

        // Number (including hex)
        if chars[i].is_ascii_digit()
            || (chars[i] == '-' && i + 1 < len && chars[i + 1].is_ascii_digit())
            || (chars[i] == '.' && i + 1 < len && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            if chars[i] == '-' {
                i += 1;
            }
            if i + 1 < len && chars[i] == '0' && (chars[i + 1] == 'x' || chars[i + 1] == 'X') {
                i += 2;
                while i < len && (chars[i].is_ascii_hexdigit() || chars[i] == '_') {
                    i += 1;
                }
            } else {
                while i < len
                    && (chars[i].is_ascii_digit()
                        || chars[i] == '.'
                        || chars[i] == '_'
                        || chars[i] == 'e'
                        || chars[i] == 'E'
                        || chars[i] == '+'
                        || chars[i] == '-')
                {
                    i += 1;
                }
            }
            // consume trailing type suffixes like _f32, _i32, etc
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            push_segment(&mut job, &s, TokenKind::Number, &font_id);
            continue;
        }

        // Identifier or keyword (includes dotted names like i32.add)
        if chars[i].is_alphabetic() || chars[i] == '_' || chars[i] == '$' {
            let start = i;
            while i < len
                && (chars[i].is_alphanumeric()
                    || chars[i] == '_'
                    || chars[i] == '.'
                    || chars[i] == '$')
            {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let kind = if keywords.contains(&word.as_str()) {
                TokenKind::Keyword
            } else if builtins.contains(&word.as_str()) {
                TokenKind::Builtin
            } else {
                TokenKind::Normal
            };
            push_segment(&mut job, &word, kind, &font_id);
            continue;
        }

        // Parentheses (WASM uses s-expression syntax)
        if chars[i] == '(' || chars[i] == ')' {
            let s: String = chars[i..=i].iter().collect();
            push_segment(&mut job, &s, TokenKind::Operator, &font_id);
            i += 1;
            continue;
        }

        // Everything else
        let s: String = chars[i..=i].iter().collect();
        push_segment(&mut job, &s, TokenKind::Normal, &font_id);
        i += 1;
    }

    job
}

pub fn detect_language(path: &str) -> ScriptLanguage {
    if path.ends_with(".lua") {
        ScriptLanguage::Lua
    } else if path.ends_with(".wat") || path.ends_with(".wasm") {
        ScriptLanguage::Wasm
    } else {
        ScriptLanguage::Lua
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua_keyword_highlight() {
        let job = highlight_lua("local x = 42", FontId::monospace(14.0));
        assert!(!job.sections.is_empty());
    }

    #[test]
    fn test_lua_comment_highlight() {
        let job = highlight_lua("-- this is a comment\nlocal x = 1", FontId::monospace(14.0));
        assert!(!job.sections.is_empty());
    }

    #[test]
    fn test_wasm_highlight() {
        let job = highlight_wasm(
            "(func $add (param $a i32) (param $b i32) (result i32))",
            FontId::monospace(14.0),
        );
        assert!(!job.sections.is_empty());
    }

    #[test]
    fn test_detect_language_lua() {
        assert_eq!(detect_language("test.lua"), ScriptLanguage::Lua);
    }

    #[test]
    fn test_detect_language_wasm() {
        assert_eq!(detect_language("test.wat"), ScriptLanguage::Wasm);
    }
}
