//! Rules from [flake8-pie](https://pypi.org/project/flake8-pie/).
pub(crate) mod rules;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Result;

    use test_case::test_case;

    use crate::registry::Rule;
    use crate::test::test_path;
    use crate::{assert_messages, settings};

    #[test_case(Rule::DuplicateClassFieldDefinition, Path::new("PIE794.py"); "PIE794")]
    #[test_case(Rule::UnnecessaryDictKwargs, Path::new("PIE804.py"); "PIE804")]
    #[test_case(Rule::MultipleStartsEndsWith, Path::new("PIE810.py"); "PIE810")]
    #[test_case(Rule::UnnecessaryPass, Path::new("PIE790.py"); "PIE790")]
    #[test_case(Rule::UnnecessarySpread, Path::new("PIE800.py"); "PIE800")]
    #[test_case(Rule::ReimplementedListBuiltin, Path::new("PIE807.py"); "PIE807")]
    #[test_case(Rule::NonUniqueEnums, Path::new("PIE796.py"); "PIE796")]
    fn rules(rule_code: Rule, path: &Path) -> Result<()> {
        let snapshot = format!("{}_{}", rule_code.noqa_code(), path.to_string_lossy());
        let diagnostics = test_path(
            Path::new("flake8_pie").join(path).as_path(),
            &settings::Settings::for_rule(rule_code),
        )?;
        assert_messages!(snapshot, diagnostics);
        Ok(())
    }
}
