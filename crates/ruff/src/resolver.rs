//! Discover Python files, and their corresponding [`Settings`], from the
//! filesystem.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use anyhow::{anyhow, bail, Result};
use ignore::{DirEntry, WalkBuilder, WalkState};
use itertools::Itertools;
use log::debug;
use path_absolutize::path_dedot;
use rustc_hash::FxHashSet;

use crate::fs;
use crate::settings::configuration::Configuration;
use crate::settings::pyproject::settings_toml;
use crate::settings::{pyproject, AllSettings, Settings};

/// The configuration information from a `pyproject.toml` file.
pub struct PyprojectConfig {
    /// The strategy used to discover the relevant `pyproject.toml` file for
    /// each Python file.
    pub strategy: PyprojectDiscoveryStrategy,
    /// All settings from the `pyproject.toml` file.
    pub settings: AllSettings,
    /// Absolute path to the `pyproject.toml` file. This would be `None` when
    /// either using the default settings or the `--isolated` flag is set.
    pub path: Option<PathBuf>,
}

impl PyprojectConfig {
    pub fn new(
        strategy: PyprojectDiscoveryStrategy,
        settings: AllSettings,
        path: Option<PathBuf>,
    ) -> Self {
        Self {
            strategy,
            settings,
            path: path.map(fs::normalize_path),
        }
    }
}

/// The strategy used to discover the relevant `pyproject.toml` file for each
/// Python file.
#[derive(Debug, is_macro::Is)]
pub enum PyprojectDiscoveryStrategy {
    /// Use a fixed `pyproject.toml` file for all Python files (i.e., one
    /// provided on the command-line).
    Fixed,
    /// Use the closest `pyproject.toml` file in the filesystem hierarchy, or
    /// the default settings.
    Hierarchical,
}

/// The strategy for resolving file paths in a `pyproject.toml`.
#[derive(Copy, Clone)]
pub enum Relativity {
    /// Resolve file paths relative to the current working directory.
    Cwd,
    /// Resolve file paths relative to the directory containing the
    /// `pyproject.toml`.
    Parent,
}

impl Relativity {
    pub fn resolve(&self, path: &Path) -> PathBuf {
        match self {
            Relativity::Parent => path
                .parent()
                .expect("Expected pyproject.toml file to be in parent directory")
                .to_path_buf(),
            Relativity::Cwd => path_dedot::CWD.clone(),
        }
    }
}

#[derive(Default)]
pub struct Resolver {
    settings: BTreeMap<PathBuf, AllSettings>,
}

impl Resolver {
    /// Add a resolved [`Settings`] under a given [`PathBuf`] scope.
    pub fn add(&mut self, path: PathBuf, settings: AllSettings) {
        self.settings.insert(path, settings);
    }

    /// Return the appropriate [`AllSettings`] for a given [`Path`].
    pub fn resolve_all<'a>(
        &'a self,
        path: &Path,
        pyproject_config: &'a PyprojectConfig,
    ) -> &'a AllSettings {
        match pyproject_config.strategy {
            PyprojectDiscoveryStrategy::Fixed => &pyproject_config.settings,
            PyprojectDiscoveryStrategy::Hierarchical => self
                .settings
                .iter()
                .rev()
                .find_map(|(root, settings)| path.starts_with(root).then_some(settings))
                .unwrap_or(&pyproject_config.settings),
        }
    }

    pub fn resolve<'a>(
        &'a self,
        path: &Path,
        pyproject_config: &'a PyprojectConfig,
    ) -> &'a Settings {
        &self.resolve_all(path, pyproject_config).lib
    }

    /// Return an iterator over the resolved [`Settings`] in this [`Resolver`].
    pub fn iter(&self) -> impl Iterator<Item = &AllSettings> {
        self.settings.values()
    }
}

pub trait ConfigProcessor: Copy + Send + Sync {
    fn process_config(&self, config: &mut Configuration);
}

struct NoOpProcessor;
impl ConfigProcessor for &NoOpProcessor {
    fn process_config(&self, _config: &mut Configuration) {}
}

/// Recursively resolve a [`Configuration`] from a `pyproject.toml` file at the
/// specified [`Path`].
// TODO(charlie): This whole system could do with some caching. Right now, if a
// configuration file extends another in the same path, we'll re-parse the same
// file at least twice (possibly more than twice, since we'll also parse it when
// resolving the "default" configuration).
pub fn resolve_configuration(
    pyproject: &Path,
    relativity: &Relativity,
    processor: impl ConfigProcessor,
) -> Result<Configuration> {
    let mut seen = FxHashSet::default();
    let mut stack = vec![];
    let mut next = Some(fs::normalize_path(pyproject));
    while let Some(path) = next {
        if seen.contains(&path) {
            bail!("Circular dependency detected in pyproject.toml");
        }

        // Resolve the current path.
        let options = pyproject::load_options(&path)
            .map_err(|err| anyhow!("Failed to parse `{}`: {}", path.display(), err))?;
        let project_root = relativity.resolve(&path);
        let configuration = Configuration::from_options(options, &project_root)?;

        // If extending, continue to collect.
        next = configuration.extend.as_ref().map(|extend| {
            fs::normalize_path_to(
                extend,
                path.parent()
                    .expect("Expected pyproject.toml file to be in parent directory"),
            )
        });

        // Keep track of (1) the paths we've already resolved (to avoid cycles), and (2)
        // the base configuration for every path.
        seen.insert(path);
        stack.push(configuration);
    }

    // Merge the configurations, in order.
    stack.reverse();
    let mut configuration = stack.pop().unwrap();
    while let Some(extend) = stack.pop() {
        configuration = configuration.combine(extend);
    }
    processor.process_config(&mut configuration);
    Ok(configuration)
}

/// Extract the project root (scope) and [`Settings`] from a given
/// `pyproject.toml`.
pub fn resolve_scoped_settings(
    pyproject: &Path,
    relativity: &Relativity,
    processor: impl ConfigProcessor,
) -> Result<(PathBuf, AllSettings)> {
    let configuration = resolve_configuration(pyproject, relativity, processor)?;
    let project_root = relativity.resolve(pyproject);
    let settings = AllSettings::from_configuration(configuration, &project_root)?;
    Ok((project_root, settings))
}

/// Extract the [`Settings`] from a given `pyproject.toml`.
pub fn resolve_settings(pyproject: &Path, relativity: &Relativity) -> Result<AllSettings> {
    let (_project_root, settings) = resolve_scoped_settings(pyproject, relativity, &NoOpProcessor)?;
    Ok(settings)
}

/// Extract the [`Settings`] from a given `pyproject.toml` and process the
/// configuration with the given [`ConfigProcessor`].
pub fn resolve_settings_with_processor(
    pyproject: &Path,
    relativity: &Relativity,
    processor: impl ConfigProcessor,
) -> Result<AllSettings> {
    let (_project_root, settings) = resolve_scoped_settings(pyproject, relativity, processor)?;
    Ok(settings)
}

/// Return `true` if the given file should be ignored based on the exclusion
/// criteria.
fn match_exclusion<P: AsRef<Path>, R: AsRef<Path>>(
    file_path: P,
    file_basename: R,
    exclusion: &globset::GlobSet,
) -> bool {
    exclusion.is_match(file_path) || exclusion.is_match(file_basename)
}

/// Find all Python (`.py`, `.pyi` and `.ipynb` files) in a set of paths.
pub fn python_files_in_path(
    paths: &[PathBuf],
    pyproject_config: &PyprojectConfig,
    processor: impl ConfigProcessor,
) -> Result<(Vec<Result<DirEntry, ignore::Error>>, Resolver)> {
    // Normalize every path (e.g., convert from relative to absolute).
    let mut paths: Vec<PathBuf> = paths.iter().map(fs::normalize_path).unique().collect();

    // Search for `pyproject.toml` files in all parent directories.
    let mut resolver = Resolver::default();
    let mut seen = FxHashSet::default();
    if pyproject_config.strategy.is_hierarchical() {
        for path in &paths {
            for ancestor in path.ancestors() {
                if seen.insert(ancestor) {
                    if let Some(pyproject) = settings_toml(ancestor)? {
                        let (root, settings) =
                            resolve_scoped_settings(&pyproject, &Relativity::Parent, processor)?;
                        resolver.add(root, settings);
                    }
                }
            }
        }
    }

    // Check if the paths themselves are excluded.
    if pyproject_config.settings.lib.force_exclude {
        paths.retain(|path| !is_file_excluded(path, &resolver, pyproject_config));
        if paths.is_empty() {
            return Ok((vec![], resolver));
        }
    }

    // Create the `WalkBuilder`.
    let mut builder = WalkBuilder::new(
        paths
            .get(0)
            .ok_or_else(|| anyhow!("Expected at least one path to search for Python files"))?,
    );
    for path in &paths[1..] {
        builder.add(path);
    }
    builder.standard_filters(pyproject_config.settings.lib.respect_gitignore);
    builder.hidden(false);
    let walker = builder.build_parallel();

    // Run the `WalkParallel` to collect all Python files.
    let error: std::sync::Mutex<Result<()>> = std::sync::Mutex::new(Ok(()));
    let resolver: RwLock<Resolver> = RwLock::new(resolver);
    let files: std::sync::Mutex<Vec<Result<DirEntry, ignore::Error>>> =
        std::sync::Mutex::new(vec![]);
    walker.run(|| {
        Box::new(|result| {
            // Respect our own exclusion behavior.
            if let Ok(entry) = &result {
                if entry.depth() > 0 {
                    let path = entry.path();
                    let resolver = resolver.read().unwrap();
                    let settings = resolver.resolve(path, pyproject_config);
                    if let Some(file_name) = path.file_name() {
                        if !settings.exclude.is_empty()
                            && match_exclusion(path, file_name, &settings.exclude)
                        {
                            debug!("Ignored path via `exclude`: {:?}", path);
                            return WalkState::Skip;
                        } else if !settings.extend_exclude.is_empty()
                            && match_exclusion(path, file_name, &settings.extend_exclude)
                        {
                            debug!("Ignored path via `extend-exclude`: {:?}", path);
                            return WalkState::Skip;
                        }
                    } else {
                        debug!("Ignored path due to error in parsing: {:?}", path);
                        return WalkState::Skip;
                    }
                }
            }

            // Search for the `pyproject.toml` file in this directory, before we visit any
            // of its contents.
            if pyproject_config.strategy.is_hierarchical() {
                if let Ok(entry) = &result {
                    if entry
                        .file_type()
                        .map_or(false, |file_type| file_type.is_dir())
                    {
                        match settings_toml(entry.path()) {
                            Ok(Some(pyproject)) => match resolve_scoped_settings(
                                &pyproject,
                                &Relativity::Parent,
                                processor,
                            ) {
                                Ok((root, settings)) => {
                                    resolver.write().unwrap().add(root, settings);
                                }
                                Err(err) => {
                                    *error.lock().unwrap() = Err(err);
                                    return WalkState::Quit;
                                }
                            },
                            Ok(None) => {}
                            Err(err) => {
                                *error.lock().unwrap() = Err(err);
                                return WalkState::Quit;
                            }
                        }
                    }
                }
            }

            if result.as_ref().map_or(true, |entry| {
                if entry.depth() == 0 {
                    // Accept all files that are passed-in directly.
                    entry.file_type().map_or(false, |ft| ft.is_file())
                } else {
                    // Otherwise, check if the file is included.
                    let path = entry.path();
                    let resolver = resolver.read().unwrap();
                    let settings = resolver.resolve(path, pyproject_config);
                    if settings.include.is_match(path) {
                        debug!("Included path via `include`: {:?}", path);
                        true
                    } else if settings.extend_include.is_match(path) {
                        debug!("Included path via `extend-include`: {:?}", path);
                        true
                    } else {
                        false
                    }
                }
            }) {
                files.lock().unwrap().push(result);
            }

            WalkState::Continue
        })
    });

    error.into_inner().unwrap()?;

    Ok((files.into_inner().unwrap(), resolver.into_inner().unwrap()))
}

/// Return `true` if the Python file at [`Path`] is _not_ excluded.
pub fn python_file_at_path(
    path: &Path,
    pyproject_config: &PyprojectConfig,
    processor: impl ConfigProcessor,
) -> Result<bool> {
    if !pyproject_config.settings.lib.force_exclude {
        return Ok(true);
    }

    // Normalize the path (e.g., convert from relative to absolute).
    let path = fs::normalize_path(path);

    // Search for `pyproject.toml` files in all parent directories.
    let mut resolver = Resolver::default();
    if pyproject_config.strategy.is_hierarchical() {
        for ancestor in path.ancestors() {
            if let Some(pyproject) = settings_toml(ancestor)? {
                let (root, settings) =
                    resolve_scoped_settings(&pyproject, &Relativity::Parent, processor)?;
                resolver.add(root, settings);
            }
        }
    }

    // Check exclusions.
    Ok(!is_file_excluded(&path, &resolver, pyproject_config))
}

/// Return `true` if the given top-level [`Path`] should be excluded.
fn is_file_excluded(
    path: &Path,
    resolver: &Resolver,
    pyproject_strategy: &PyprojectConfig,
) -> bool {
    // TODO(charlie): Respect gitignore.
    for path in path.ancestors() {
        if path.file_name().is_none() {
            break;
        }
        let settings = resolver.resolve(path, pyproject_strategy);
        if let Some(file_name) = path.file_name() {
            if !settings.exclude.is_empty() && match_exclusion(path, file_name, &settings.exclude) {
                debug!("Ignored path via `exclude`: {:?}", path);
                return true;
            } else if !settings.extend_exclude.is_empty()
                && match_exclusion(path, file_name, &settings.extend_exclude)
            {
                debug!("Ignored path via `extend-exclude`: {:?}", path);
                return true;
            }
        } else {
            debug!("Ignored path due to error in parsing: {:?}", path);
            return true;
        }
        if path == settings.project_root {
            // Bail out; we'd end up past the project root on the next iteration
            // (excludes etc. are thus "rooted" to the project).
            break;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Result;
    use globset::GlobSet;
    use path_absolutize::Absolutize;

    use crate::resolver::{
        is_file_excluded, match_exclusion, resolve_settings_with_processor, NoOpProcessor,
        PyprojectConfig, PyprojectDiscoveryStrategy, Relativity, Resolver,
    };
    use crate::settings::pyproject::find_settings_toml;
    use crate::settings::types::FilePattern;
    use crate::test::test_resource_path;

    fn make_exclusion(file_pattern: FilePattern) -> GlobSet {
        let mut builder = globset::GlobSetBuilder::new();
        file_pattern.add_to(&mut builder).unwrap();
        builder.build().unwrap()
    }

    #[test]
    fn exclusions() {
        let project_root = Path::new("/tmp/");

        let path = Path::new("foo").absolutize_from(project_root).unwrap();
        let exclude = FilePattern::User(
            "foo".to_string(),
            Path::new("foo")
                .absolutize_from(project_root)
                .unwrap()
                .to_path_buf(),
        );
        let file_path = &path;
        let file_basename = path.file_name().unwrap();
        assert!(match_exclusion(
            file_path,
            file_basename,
            &make_exclusion(exclude),
        ));

        let path = Path::new("foo/bar").absolutize_from(project_root).unwrap();
        let exclude = FilePattern::User(
            "bar".to_string(),
            Path::new("bar")
                .absolutize_from(project_root)
                .unwrap()
                .to_path_buf(),
        );
        let file_path = &path;
        let file_basename = path.file_name().unwrap();
        assert!(match_exclusion(
            file_path,
            file_basename,
            &make_exclusion(exclude),
        ));

        let path = Path::new("foo/bar/baz.py")
            .absolutize_from(project_root)
            .unwrap();
        let exclude = FilePattern::User(
            "baz.py".to_string(),
            Path::new("baz.py")
                .absolutize_from(project_root)
                .unwrap()
                .to_path_buf(),
        );
        let file_path = &path;
        let file_basename = path.file_name().unwrap();
        assert!(match_exclusion(
            file_path,
            file_basename,
            &make_exclusion(exclude),
        ));

        let path = Path::new("foo/bar").absolutize_from(project_root).unwrap();
        let exclude = FilePattern::User(
            "foo/bar".to_string(),
            Path::new("foo/bar")
                .absolutize_from(project_root)
                .unwrap()
                .to_path_buf(),
        );
        let file_path = &path;
        let file_basename = path.file_name().unwrap();
        assert!(match_exclusion(
            file_path,
            file_basename,
            &make_exclusion(exclude),
        ));

        let path = Path::new("foo/bar/baz.py")
            .absolutize_from(project_root)
            .unwrap();
        let exclude = FilePattern::User(
            "foo/bar/baz.py".to_string(),
            Path::new("foo/bar/baz.py")
                .absolutize_from(project_root)
                .unwrap()
                .to_path_buf(),
        );
        let file_path = &path;
        let file_basename = path.file_name().unwrap();
        assert!(match_exclusion(
            file_path,
            file_basename,
            &make_exclusion(exclude),
        ));

        let path = Path::new("foo/bar/baz.py")
            .absolutize_from(project_root)
            .unwrap();
        let exclude = FilePattern::User(
            "foo/bar/*.py".to_string(),
            Path::new("foo/bar/*.py")
                .absolutize_from(project_root)
                .unwrap()
                .to_path_buf(),
        );
        let file_path = &path;
        let file_basename = path.file_name().unwrap();
        assert!(match_exclusion(
            file_path,
            file_basename,
            &make_exclusion(exclude),
        ));

        let path = Path::new("foo/bar/baz.py")
            .absolutize_from(project_root)
            .unwrap();
        let exclude = FilePattern::User(
            "baz".to_string(),
            Path::new("baz")
                .absolutize_from(project_root)
                .unwrap()
                .to_path_buf(),
        );
        let file_path = &path;
        let file_basename = path.file_name().unwrap();
        assert!(!match_exclusion(
            file_path,
            file_basename,
            &make_exclusion(exclude),
        ));
    }

    #[test]
    fn rooted_exclusion() -> Result<()> {
        let package_root = test_resource_path("package");
        let resolver = Resolver::default();
        let pyproject_config = PyprojectConfig::new(
            PyprojectDiscoveryStrategy::Hierarchical,
            resolve_settings_with_processor(
                &find_settings_toml(&package_root)?.unwrap(),
                &Relativity::Parent,
                &NoOpProcessor,
            )?,
            None,
        );
        // src/app.py should not be excluded even if it lives in a hierarchy that should
        // be excluded by virtue of the pyproject.toml having `resources/*` in
        // it.
        assert!(!is_file_excluded(
            &package_root.join("src/app.py"),
            &resolver,
            &pyproject_config,
        ));
        // However, resources/ignored.py should be ignored, since that `resources` is
        // beneath the package root.
        assert!(is_file_excluded(
            &package_root.join("resources/ignored.py"),
            &resolver,
            &pyproject_config,
        ));
        Ok(())
    }
}
