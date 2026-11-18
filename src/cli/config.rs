use crate::opt::Opt;
use anyhow::{Context, Result};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use log::*;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use styluau_lib::Config;
use styluau_lib::SortRequiresConfig;

#[cfg(feature = "editorconfig")]
use styluau_lib::editorconfig;

static CONFIG_FILE_NAME: [&str; 4] = [
    "styluau.toml",
    ".styluau.toml",
    "stylua.toml",
    ".stylua.toml",
];

#[derive(Clone, Debug)]
struct ConfigFile {
    directory: PathBuf,
    base: toml::value::Table,
    overrides: Vec<ConfigOverride>,
}

#[derive(Clone, Debug)]
struct ConfigOverride {
    files: GlobSet,
    settings: toml::value::Table,
}

fn merge_settings(base: &mut toml::value::Table, settings: &toml::value::Table) {
    for (key, value) in settings {
        match (base.get_mut(key), value) {
            (Some(toml::Value::Table(base)), toml::Value::Table(settings)) => {
                merge_settings(base, settings);
            }
            _ => {
                base.insert(key.clone(), value.clone());
            }
        }
    }
}

fn parse_settings(settings: &toml::value::Table) -> Result<Config> {
    toml::Value::Table(settings.clone())
        .try_into()
        .context("Config file not in correct format")
}

fn read_config_file(path: &Path) -> Result<ConfigFile> {
    let contents = fs::read_to_string(path).context("Failed to read config file")?;
    let mut base: toml::value::Table =
        toml::from_str(&contents).context("Config file not in correct format")?;
    let entries = match base.remove("overrides") {
        Some(toml::Value::Array(entries)) => entries,
        Some(_) => anyhow::bail!("Config overrides must be an array of tables"),
        None => Vec::new(),
    };
    parse_settings(&base)?;
    let mut overrides = Vec::new();
    for (index, entry) in entries.into_iter().enumerate() {
        let mut settings = entry
            .as_table()
            .cloned()
            .with_context(|| format!("Config override {} must be a table", index + 1))?;
        let files = settings
            .remove("files")
            .with_context(|| format!("Config override {} must have a files array", index + 1))?;
        let files = files.as_array().with_context(|| {
            format!(
                "Config override {} files must be an array of glob strings",
                index + 1
            )
        })?;
        let mut builder = GlobSetBuilder::new();
        for pattern in files {
            let pattern = pattern.as_str().with_context(|| {
                format!(
                    "Config override {} files must contain glob strings",
                    index + 1
                )
            })?;
            let glob = GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()
                .with_context(|| {
                    format!("Invalid glob {pattern:?} in config override {}", index + 1)
                })?;
            builder.add(glob);
        }
        let files = builder
            .build()
            .context("Failed to compile config override globs")?;
        let mut validated = base.clone();
        merge_settings(&mut validated, &settings);
        parse_settings(&validated)
            .with_context(|| format!("Invalid config override {}", index + 1))?;
        overrides.push(ConfigOverride { files, settings });
    }
    let absolute = env::current_dir()?.join(path);
    let directory = normalize_path(
        absolute
            .parent()
            .context("Config file has no parent directory")?,
    );
    Ok(ConfigFile {
        directory,
        base,
        overrides,
    })
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => (),
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn relative_path(base: &Path, path: &Path) -> Option<PathBuf> {
    let base: Vec<_> = base.components().collect();
    let path: Vec<_> = path.components().collect();
    let common = base.iter().zip(&path).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return None;
    }
    let mut result = PathBuf::new();
    for _ in common..base.len() {
        result.push("..");
    }
    for component in &path[common..] {
        result.push(component.as_os_str());
    }
    Some(result)
}

impl ConfigFile {
    fn resolve(&self, path: Option<&Path>, opt: &Opt) -> Result<Config> {
        let mut settings = self.base.clone();
        if let Some(path) = path {
            let path = normalize_path(path);
            if let Some(relative) = relative_path(&self.directory, &path) {
                for rule in &self.overrides {
                    if rule.files.is_match(&relative) {
                        merge_settings(&mut settings, &rule.settings);
                    }
                }
            }
        }
        Ok(load_overrides(parse_settings(&settings)?, opt))
    }
}

pub struct ConfigResolver<'a> {
    config_cache: HashMap<(PathBuf, Option<PathBuf>), Option<ConfigFile>>,
    forced_configuration: Option<ConfigFile>,
    current_directory: PathBuf,
    default_configuration: Config,
    opt: &'a Opt,
}

impl ConfigResolver<'_> {
    pub fn new(opt: &Opt) -> Result<ConfigResolver<'_>> {
        let forced_configuration = opt
            .config_path
            .as_ref()
            .map(|config_path| {
                debug!(
                    "config: explicit config path provided at {}",
                    config_path.display()
                );
                read_config_file(config_path)
            })
            .transpose()?;

        Ok(ConfigResolver {
            config_cache: HashMap::new(),
            forced_configuration,
            current_directory: env::current_dir().context("Could not find current directory")?,
            default_configuration: load_overrides(Config::default(), opt),
            opt,
        })
    }

    /// Returns the root used when searching for configuration
    /// If `--search-parent-directories`, then there is no root, and we keep searching
    /// Else, the root is the current working directory, and we do not search higher than the cwd
    fn get_configuration_search_root(
        &self,
        search_root_override: Option<PathBuf>,
    ) -> Option<PathBuf> {
        match self.opt.search_parent_directories {
            true => None,
            false => {
                Some(search_root_override.unwrap_or_else(|| self.current_directory.to_path_buf()))
            }
        }
    }

    pub(crate) fn load_configuration_with_search_root(
        &mut self,
        path: &Path,
        search_root_override: Option<PathBuf>,
    ) -> Result<Config> {
        let absolute_path = normalize_path(&self.current_directory.join(path));
        if let Some(configuration) = &self.forced_configuration {
            return configuration.resolve(Some(&absolute_path), self.opt);
        }

        let root = self.get_configuration_search_root(search_root_override);

        let parent_path = &absolute_path
            .parent()
            .with_context(|| format!("no parent directory found for {}", path.display()))?;

        match self.find_config_file(parent_path, root)? {
            Some(config) => config.resolve(Some(&absolute_path), self.opt),
            None => {
                #[cfg(feature = "editorconfig")]
                if self.opt.no_editorconfig {
                    Ok(self.default_configuration.clone())
                } else {
                    editorconfig::parse(self.default_configuration.clone(), path)
                        .context("could not parse editorconfig")
                }
                #[cfg(not(feature = "editorconfig"))]
                Ok(self.default_configuration.clone())
            }
        }
    }

    pub fn load_configuration(&mut self, path: &Path) -> Result<Config> {
        self.load_configuration_with_search_root(path, None)
    }

    pub fn load_configuration_for_stdin(&mut self) -> Result<Config> {
        if let Some(configuration) = &self.forced_configuration {
            let path = self
                .opt
                .stdin_filepath
                .as_ref()
                .map(|path| self.current_directory.join(path));
            return configuration.resolve(path.as_deref(), self.opt);
        }

        let root = self.get_configuration_search_root(None);
        let my_current_directory = self.current_directory.to_owned();

        match &self.opt.stdin_filepath {
            Some(filepath) => self.load_configuration(filepath),
            None => match self.find_config_file(&my_current_directory, root)? {
                Some(config) => config.resolve(None, self.opt),
                None => {
                    #[cfg(feature = "editorconfig")]
                    if self.opt.no_editorconfig {
                        Ok(self.default_configuration.clone())
                    } else {
                        editorconfig::parse(
                            self.default_configuration.clone(),
                            &PathBuf::from("*.lua"),
                        )
                        .context("could not parse editorconfig")
                    }
                    #[cfg(not(feature = "editorconfig"))]
                    Ok(self.default_configuration.clone())
                }
            },
        }
    }

    fn lookup_config_file_in_directory(&self, directory: &Path) -> Result<Option<ConfigFile>> {
        debug!("config: looking for config in {}", directory.display());
        let config_file = find_toml_file(directory);
        match config_file {
            Some(file_path) => {
                debug!("config: found config at {}", file_path.display());
                let config = read_config_file(&file_path)?;
                debug!("config: {:#?}", config);
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// Looks for a configuration file in the directory provided
    /// Keep searching recursively upwards until we hit the root (if provided), then stop
    /// When `--search-parent-directories` is enabled, root = None, else root = Some(cwd)
    fn find_config_file(
        &mut self,
        directory: &Path,
        root: Option<PathBuf>,
    ) -> Result<Option<ConfigFile>> {
        let cache_key = (directory.to_path_buf(), root.clone());
        if let Some(config) = self.config_cache.get(&cache_key) {
            return Ok(config.clone());
        }

        let resolved_configuration = match self.lookup_config_file_in_directory(directory)? {
            Some(config) => Some(config),
            None => {
                let parent_directory = directory.parent();
                let should_stop = Some(directory) == root.as_deref() || parent_directory.is_none();

                if should_stop {
                    debug!("config: no configuration file found");
                    if self.opt.search_parent_directories {
                        if let Some(config) = self.search_config_locations()? {
                            return Ok(Some(config));
                        }
                    }

                    debug!("config: falling back to default config");
                    None
                } else {
                    self.find_config_file(parent_directory.unwrap(), root)?
                }
            }
        };

        self.config_cache
            .insert(cache_key, resolved_configuration.clone());
        Ok(resolved_configuration)
    }

    /// Looks for a configuration file at either `$XDG_CONFIG_HOME`, `$XDG_CONFIG_HOME/stylua`, `$HOME/.config` or `$HOME/.config/stylua`
    fn search_config_locations(&self) -> Result<Option<ConfigFile>> {
        // Look in `$XDG_CONFIG_HOME`
        if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            let xdg_config_path = Path::new(&xdg_config);
            if xdg_config_path.exists() {
                debug!("config: looking in $XDG_CONFIG_HOME");

                if let Some(config) = self.lookup_config_file_in_directory(xdg_config_path)? {
                    return Ok(Some(config));
                }

                for name in ["styluau", "stylua"] {
                    let path = xdg_config_path.join(name);
                    if path.exists() {
                        if let Some(config) = self.lookup_config_file_in_directory(&path)? {
                            return Ok(Some(config));
                        }
                    }
                }
            }
        }

        // Look in `$HOME/.config`
        if let Ok(home) = std::env::var("HOME") {
            let home_config_path = Path::new(&home).join(".config");

            if home_config_path.exists() {
                debug!("config: looking in $HOME/.config");

                if let Some(config) = self.lookup_config_file_in_directory(&home_config_path)? {
                    return Ok(Some(config));
                }

                for name in ["styluau", "stylua"] {
                    let path = home_config_path.join(name);
                    if path.exists() {
                        if let Some(config) = self.lookup_config_file_in_directory(&path)? {
                            return Ok(Some(config));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

/// Searches the directory for the configuration toml file (i.e. `stylua.toml` or `.stylua.toml`)
fn find_toml_file(directory: &Path) -> Option<PathBuf> {
    for name in &CONFIG_FILE_NAME {
        let file_path = directory.join(name);
        if file_path.exists() {
            return Some(file_path);
        }
    }

    None
}

pub fn find_ignore_file_path(mut directory: PathBuf, recursive: bool) -> Option<PathBuf> {
    debug!("config: looking for ignore file in {}", directory.display());
    for name in [".styluauignore", ".styluaignore"] {
        let file_path = directory.join(name);
        if file_path.is_file() {
            debug!("config: resolved ignore file at {}", file_path.display());
            return Some(file_path);
        }
    }
    if recursive && directory.pop() {
        find_ignore_file_path(directory, recursive)
    } else {
        None
    }
}

/// Handles any overrides provided by command line options
fn load_overrides(config: Config, opt: &Opt) -> Config {
    let mut new_config = config;

    if let Some(syntax) = opt.format_opts.syntax {
        new_config.syntax = syntax.into();
    };
    if let Some(column_width) = opt.format_opts.column_width {
        new_config.column_width = column_width;
    };
    if let Some(line_endings) = opt.format_opts.line_endings {
        new_config.line_endings = line_endings.into();
    };
    if let Some(indent_type) = opt.format_opts.indent_type {
        new_config.indent_type = indent_type.into();
    };
    if let Some(indent_width) = opt.format_opts.indent_width {
        new_config.indent_width = indent_width;
    };
    if let Some(quote_style) = opt.format_opts.quote_style {
        new_config.quote_style = quote_style.into();
    };
    if let Some(call_parentheses) = opt.format_opts.call_parentheses {
        new_config.call_parentheses = call_parentheses.into();
    };
    if let Some(space_after_function_names) = opt.format_opts.space_after_function_names {
        new_config.space_after_function_names = space_after_function_names.into();
    };
    if let Some(collapse_simple_statement) = opt.format_opts.collapse_simple_statement {
        new_config.collapse_simple_statement = collapse_simple_statement.into();
    }
    if let Some(preserve_block_newline_gaps) = opt.format_opts.preserve_block_newline_gaps {
        new_config.block_newline_gaps = preserve_block_newline_gaps.into();
    };
    if opt.format_opts.sort_requires {
        new_config.sort_requires = SortRequiresConfig { enabled: true }
    }

    new_config
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::StructOpt;
    use styluau_lib::{
        CallParenType, CollapseSimpleStatement, IndentType, LineEndings, LuaVersion, QuoteStyle,
    };

    #[test]
    fn test_styluau_config_precedence() {
        use assert_fs::prelude::*;

        let directory = assert_fs::TempDir::new().unwrap();
        directory.child("stylua.toml").touch().unwrap();
        assert_eq!(
            find_toml_file(directory.path()),
            Some(directory.child("stylua.toml").path().to_path_buf())
        );
        directory
            .child("styluau.toml")
            .write_str("table_trailing_comma = true\n")
            .unwrap();
        let path = find_toml_file(directory.path()).unwrap();
        assert_eq!(path, directory.child("styluau.toml").path());
        assert!(
            read_config_file(&path)
                .unwrap()
                .resolve(None, &Opt::parse_from(["styluau"]))
                .unwrap()
                .table_trailing_comma
        );
    }

    #[test]
    fn test_ordered_file_overrides_and_cli_precedence() {
        use assert_fs::prelude::*;

        let directory = assert_fs::TempDir::new().unwrap();
        let file = directory.child("styluau.toml");
        file.write_str(
            r#"
column_width = 100
indent_width = 4
fusion_scope_names = ["base"]
[sort_requires]
enabled = true
[[overrides]]
files = ["**/*.lua"]
column_width = 80
indent_width = 2
[[overrides]]
files = ["nested/*.lua"]
column_width = 60
fusion_scope_names = ["custom", "second"]
sort_requires = { enabled = false }
"#,
        )
        .unwrap();
        let config = read_config_file(file.path()).unwrap();
        let opt = Opt::parse_from(["styluau", "--column-width", "90"]);
        let matching = config
            .resolve(Some(&directory.path().join("nested/test.lua")), &opt)
            .unwrap();
        assert_eq!((matching.column_width, matching.indent_width), (90, 2));
        assert!(!matching.sort_requires.enabled);
        assert_eq!(matching.fusion_scope_names, ["custom", "second"]);
        let other = config
            .resolve(Some(&directory.path().join("test.luau")), &opt)
            .unwrap();
        assert_eq!((other.column_width, other.indent_width), (90, 4));
        assert!(other.sort_requires.enabled);
        assert_eq!(other.fusion_scope_names, ["base"]);
        assert_eq!(config.resolve(None, &opt).unwrap().indent_width, 4);
    }

    #[test]
    fn test_resolver_search_root_and_file_specific_cache() {
        use assert_fs::prelude::*;

        let directory = assert_fs::TempDir::new().unwrap();
        directory
            .child("styluau.toml")
            .write_str(
                "indent_width = 4\n[[overrides]]\nfiles = ['nested/a.lua']\nindent_width = 2\n",
            )
            .unwrap();
        directory.child("nested").create_dir_all().unwrap();
        let opt = Opt::parse_from(["styluau"]);
        let mut resolver = ConfigResolver::new(&opt).unwrap();
        let nested = directory.path().join("nested");
        let first = nested.join("a.lua");
        let second = nested.join("b.lua");
        assert_eq!(
            resolver
                .load_configuration_with_search_root(&first, Some(directory.path().to_path_buf()))
                .unwrap()
                .indent_width,
            2
        );
        assert_eq!(
            resolver
                .load_configuration_with_search_root(&second, Some(directory.path().to_path_buf()))
                .unwrap()
                .indent_width,
            4
        );
        assert_eq!(
            resolver
                .load_configuration_with_search_root(&first, Some(nested))
                .unwrap()
                .indent_width,
            Config::default().indent_width
        );
    }

    #[test]
    fn test_invalid_override_glob_and_settings() {
        use assert_fs::prelude::*;

        let directory = assert_fs::TempDir::new().unwrap();
        let file = directory.child("styluau.toml");
        file.write_str("[[overrides]]\nfiles = ['[']\n").unwrap();
        assert!(
            format!("{:#}", read_config_file(file.path()).unwrap_err()).contains("Invalid glob")
        );
        file.write_str("[[overrides]]\nfiles = ['*.lua']\nunknown = true\n")
            .unwrap();
        assert!(
            format!("{:#}", read_config_file(file.path()).unwrap_err()).contains("unknown field")
        );
    }

    #[test]
    fn test_override_syntax() {
        let override_opt = Opt::parse_from(vec!["BINARY_NAME", "--syntax", "Lua51"]);
        let default_config = Config::new();
        let config = load_overrides(default_config, &override_opt);
        assert_eq!(config.syntax, LuaVersion::Lua51);
    }

    #[test]
    fn test_override_column_width() {
        let override_opt = Opt::parse_from(vec!["BINARY_NAME", "--column-width", "80"]);
        let default_config = Config::new();
        let config = load_overrides(default_config, &override_opt);
        assert_eq!(config.column_width, 80);
    }

    #[test]
    fn test_override_line_endings() {
        let override_opt = Opt::parse_from(vec!["BINARY_NAME", "--line-endings", "Windows"]);
        let default_config = Config::new();
        let config = load_overrides(default_config, &override_opt);
        assert_eq!(config.line_endings, LineEndings::Windows);
    }

    #[test]
    fn test_override_indent_type() {
        let override_opt = Opt::parse_from(vec!["BINARY_NAME", "--indent-type", "Spaces"]);
        let default_config = Config::new();
        let config = load_overrides(default_config, &override_opt);
        assert_eq!(config.indent_type, IndentType::Spaces);
    }

    #[test]
    fn test_override_indent_width() {
        let override_opt = Opt::parse_from(vec!["BINARY_NAME", "--indent-width", "2"]);
        let default_config = Config::new();
        let config = load_overrides(default_config, &override_opt);
        assert_eq!(config.indent_width, 2);
    }

    #[test]
    fn test_override_quote_style() {
        let override_opt = Opt::parse_from(vec!["BINARY_NAME", "--quote-style", "ForceSingle"]);
        let default_config = Config::new();
        let config = load_overrides(default_config, &override_opt);
        assert_eq!(config.quote_style, QuoteStyle::ForceSingle);
    }

    #[test]
    fn test_override_call_parentheses() {
        let override_opt = Opt::parse_from(vec!["BINARY_NAME", "--call-parentheses", "None"]);
        let default_config = Config::new();
        let config = load_overrides(default_config, &override_opt);
        assert_eq!(config.call_parentheses, CallParenType::None);
    }

    #[test]
    fn test_override_collapse_simple_statement() {
        let override_opt =
            Opt::parse_from(vec!["BINARY_NAME", "--collapse-simple-statement", "Input"]);
        let default_config = Config::new();
        let config = load_overrides(default_config, &override_opt);
        assert_eq!(
            config.collapse_simple_statement,
            CollapseSimpleStatement::Input
        );
    }
}
