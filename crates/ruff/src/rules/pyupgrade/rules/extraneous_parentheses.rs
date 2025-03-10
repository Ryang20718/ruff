use ruff_text_size::TextRange;
use rustpython_parser::lexer::LexResult;
use rustpython_parser::Tok;

use ruff_diagnostics::{AlwaysAutofixableViolation, Diagnostic, Edit, Fix};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::source_code::Locator;

use crate::registry::Rule;
use crate::settings::Settings;

#[violation]
pub struct ExtraneousParentheses;

impl AlwaysAutofixableViolation for ExtraneousParentheses {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!("Avoid extraneous parentheses")
    }

    fn autofix_title(&self) -> String {
        "Remove extraneous parentheses".to_string()
    }
}

// See: https://github.com/asottile/pyupgrade/blob/97ed6fb3cf2e650d4f762ba231c3f04c41797710/pyupgrade/_main.py#L148
fn match_extraneous_parentheses(tokens: &[LexResult], mut i: usize) -> Option<(usize, usize)> {
    i += 1;

    loop {
        if i >= tokens.len() {
            return None;
        }
        let Ok((tok, _)) = &tokens[i] else {
            return None;
        };
        match tok {
            Tok::Comment(..) | Tok::NonLogicalNewline => {
                i += 1;
            }
            Tok::Lpar => {
                break;
            }
            _ => {
                return None;
            }
        }
    }

    // Store the location of the extraneous opening parenthesis.
    let start = i;

    // Verify that we're not in a tuple or coroutine.
    let mut depth = 1;
    while depth > 0 {
        i += 1;
        if i >= tokens.len() {
            return None;
        }
        let Ok((tok, _)) = &tokens[i] else {
            return None;
        };

        // If we find a comma or a yield at depth 1 or 2, it's a tuple or coroutine.
        if depth == 1 && matches!(tok, Tok::Comma | Tok::Yield) {
            return None;
        } else if matches!(tok, Tok::Lpar | Tok::Lbrace | Tok::Lsqb) {
            depth += 1;
        } else if matches!(tok, Tok::Rpar | Tok::Rbrace | Tok::Rsqb) {
            depth -= 1;
        }
    }

    // Store the location of the extraneous closing parenthesis.
    let end = i;

    // Verify that we're not in an empty tuple.
    if (start + 1..i).all(|i| {
        matches!(
            tokens[i],
            Ok((Tok::Comment(..) | Tok::NonLogicalNewline, _))
        )
    }) {
        return None;
    }

    // Find the next non-coding token.
    i += 1;
    loop {
        if i >= tokens.len() {
            return None;
        }
        let Ok(( tok, _)) = &tokens[i] else {
            return None;
        };
        match tok {
            Tok::Comment(..) | Tok::NonLogicalNewline => {
                i += 1;
            }
            _ => {
                break;
            }
        }
    }

    if i >= tokens.len() {
        return None;
    }
    let Ok(( tok, _)) = &tokens[i] else {
        return None;
    };
    if matches!(tok, Tok::Rpar) {
        Some((start, end))
    } else {
        None
    }
}

/// UP034
pub(crate) fn extraneous_parentheses(
    tokens: &[LexResult],
    locator: &Locator,
    settings: &Settings,
) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];
    let mut i = 0;
    while i < tokens.len() {
        if matches!(tokens[i], Ok((Tok::Lpar, _))) {
            if let Some((start, end)) = match_extraneous_parentheses(tokens, i) {
                i = end + 1;
                let Ok((_, start_range)) = &tokens[start] else {
                    return diagnostics;
                };
                let Ok((.., end_range)) = &tokens[end] else {
                    return diagnostics;
                };
                let mut diagnostic = Diagnostic::new(
                    ExtraneousParentheses,
                    TextRange::new(start_range.start(), end_range.end()),
                );
                if settings.rules.should_fix(Rule::ExtraneousParentheses) {
                    let contents =
                        locator.slice(TextRange::new(start_range.start(), end_range.end()));
                    #[allow(deprecated)]
                    diagnostic.set_fix(Fix::unspecified(Edit::replacement(
                        contents[1..contents.len() - 1].to_string(),
                        start_range.start(),
                        end_range.end(),
                    )));
                }
                diagnostics.push(diagnostic);
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    diagnostics
}
