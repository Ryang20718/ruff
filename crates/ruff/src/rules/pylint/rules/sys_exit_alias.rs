use rustpython_parser::ast::{self, Expr, Ranged};

use ruff_diagnostics::{AutofixKind, Diagnostic, Edit, Fix, Violation};
use ruff_macros::{derive_message_formats, violation};

use crate::autofix::actions::get_or_import_symbol;
use crate::checkers::ast::Checker;
use crate::registry::AsRule;

/// ## What it does
/// Checks for uses of the `exit()` and `quit()`.
///
/// ## Why is this bad?
/// `exit` and `quit` come from the `site` module, which is typically imported
/// automatically during startup. However, it is not _guaranteed_ to be
/// imported, and so using these functions may result in a `NameError` at
/// runtime. Generally, these constants are intended to be used in an interactive
/// interpreter, and not in programs.
///
/// Prefer `sys.exit()`, as the `sys` module is guaranteed to exist in all
/// contexts.
///
/// ## Example
/// ```python
/// if __name__ == "__main__":
///     exit()
/// ```
///
/// Use instead:
/// ```python
/// import sys
///
/// if __name__ == "__main__":
///     sys.exit()
/// ```
///
/// ## References
/// - [Python documentation](https://docs.python.org/3/library/constants.html#constants-added-by-the-site-module)
#[violation]
pub struct SysExitAlias {
    name: String,
}

impl Violation for SysExitAlias {
    const AUTOFIX: AutofixKind = AutofixKind::Sometimes;

    #[derive_message_formats]
    fn message(&self) -> String {
        let SysExitAlias { name } = self;
        format!("Use `sys.exit()` instead of `{name}`")
    }

    fn autofix_title(&self) -> Option<String> {
        let SysExitAlias { name } = self;
        Some(format!("Replace `{name}` with `sys.exit()`"))
    }
}

/// PLR1722
pub(crate) fn sys_exit_alias(checker: &mut Checker, func: &Expr) {
    let Expr::Name(ast::ExprName { id, .. }) = func else {
        return;
    };
    for name in ["exit", "quit"] {
        if id != name {
            continue;
        }
        if !checker.ctx.is_builtin(name) {
            continue;
        }
        let mut diagnostic = Diagnostic::new(
            SysExitAlias {
                name: name.to_string(),
            },
            func.range(),
        );
        if checker.patch(diagnostic.kind.rule()) {
            diagnostic.try_set_fix(|| {
                let (import_edit, binding) = get_or_import_symbol(
                    "sys",
                    "exit",
                    func.start(),
                    &checker.ctx,
                    &checker.importer,
                    checker.locator,
                )?;
                let reference_edit = Edit::range_replacement(binding, func.range());
                #[allow(deprecated)]
                Ok(Fix::unspecified_edits(import_edit, [reference_edit]))
            });
        }
        checker.diagnostics.push(diagnostic);
    }
}
