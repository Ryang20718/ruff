use rustpython_parser::ast::Constant::Bool;
use rustpython_parser::ast::{self, Expr, Ranged, Stmt};

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};

use crate::checkers::ast::Checker;

use super::helpers;

/// ## What it does
/// Checks nullable string-based fields (like `CharField` and `TextField`)
/// in Django models.
///
/// ## Why is this bad?
/// If a string-based field is nullable, then your model will have two possible
/// representations for "no data": `None` and the empty string. This can lead to
/// confusion, as clients of the API have to check for both `None` and the
/// empty string when trying to determine if the field has data.
///
/// The Django convention is to use the empty string in lieu of `None` for
/// string-based fields.
///
/// ## Example
/// ```python
/// from django.db import models
///
///
/// class MyModel(models.Model):
///     field = models.CharField(max_length=255, null=True)
/// ```
///
/// Use instead:
/// ```python
/// from django.db import models
///
///
/// class MyModel(models.Model):
///     field = models.CharField(max_length=255, default="")
/// ```
#[violation]
pub struct DjangoNullableModelStringField {
    field_name: String,
}

impl Violation for DjangoNullableModelStringField {
    #[derive_message_formats]
    fn message(&self) -> String {
        let DjangoNullableModelStringField { field_name } = self;
        format!("Avoid using `null=True` on string-based fields such as {field_name}")
    }
}

const NOT_NULL_TRUE_FIELDS: [&str; 6] = [
    "CharField",
    "TextField",
    "SlugField",
    "EmailField",
    "FilePathField",
    "URLField",
];

/// DJ001
pub(crate) fn nullable_model_string_field(checker: &Checker, body: &[Stmt]) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    for statement in body.iter() {
        let Stmt::Assign(ast::StmtAssign {value, ..}) = statement else {
            continue
        };
        if let Some(field_name) = is_nullable_field(checker, value) {
            errors.push(Diagnostic::new(
                DjangoNullableModelStringField {
                    field_name: field_name.to_string(),
                },
                value.range(),
            ));
        }
    }
    errors
}

fn is_nullable_field<'a>(checker: &'a Checker, value: &'a Expr) -> Option<&'a str> {
    let Expr::Call(ast::ExprCall {func, keywords, ..}) = value else {
        return None;
    };

    let Some(valid_field_name) = helpers::get_model_field_name(&checker.ctx, func) else {
        return None;
    };

    if !NOT_NULL_TRUE_FIELDS.contains(&valid_field_name) {
        return None;
    }

    let mut null_key = false;
    let mut blank_key = false;
    let mut unique_key = false;
    for keyword in keywords.iter() {
        let Expr::Constant(ast::ExprConstant {value: Bool(true), ..}) = &keyword.value else {
            continue
        };
        let Some(argument) = &keyword.arg else {
            continue
        };
        match argument.as_str() {
            "blank" => blank_key = true,
            "null" => null_key = true,
            "unique" => unique_key = true,
            _ => continue,
        }
    }
    if blank_key && unique_key {
        return None;
    }
    if !null_key {
        return None;
    }
    Some(valid_field_name)
}
