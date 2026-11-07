use serde::Deserialize;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const DEFAULT_CONFIG: &str = r#"# browser-router configuration
# Rules are checked from top to bottom; the first matching rule wins.

[default_browser]
# Executable names are resolved with the normal Windows executable search.
path = "msedge.exe"
args = []

# [[rules]]
# name = "Work links"
# browser = 'C:\Program Files\Google\Chrome\Application\chrome.exe'
# args = ["--profile-directory=Work"]
# days = ["Mon", "Tue", "Wed", "Thu", "Fri"]
# time_range = ["08:00", "18:00"]
# domains = ["*.internal.net", "*corp.com*", "regex:^jira[0-9]+\\.example\\.com$"]
"#;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(from = "BrowserLaunchValue")]
pub struct BrowserLaunch {
    pub path: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BrowserLaunchValue {
    Path(String),
    Detailed {
        path: String,
        #[serde(default)]
        args: Vec<String>,
    },
}

impl From<BrowserLaunchValue> for BrowserLaunch {
    fn from(value: BrowserLaunchValue) -> Self {
        match value {
            BrowserLaunchValue::Path(path) => Self {
                path,
                args: Vec::new(),
            },
            BrowserLaunchValue::Detailed { path, args } => Self { path, args },
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Rule {
    pub name: String,
    pub browser: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub days: Vec<String>,
    pub time_range: Option<[String; 2]>,
    #[serde(default)]
    pub domains: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub default_browser: BrowserLaunch,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_browser: BrowserLaunch {
                path: "msedge.exe".to_owned(),
                args: Vec::new(),
            },
            rules: Vec::new(),
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    env::var_os("APPDATA")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .map(|root| root.join("browser-router").join("config.toml"))
}

pub fn load_or_create() -> Result<Config, String> {
    let path = config_path().ok_or_else(|| "%APPDATA% is not available".to_owned())?;
    create_default_if_missing(&path).map_err(|error| {
        format!(
            "could not create default configuration at {}: {error}",
            path.display()
        )
    })?;

    let source = fs::read_to_string(&path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    toml::from_str(&source)
        .map_err(|error| format!("invalid configuration in {}: {error}", path.display()))
}

fn create_default_if_missing(path: &Path) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => file.write_all(DEFAULT_CONFIG.as_bytes()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_detailed_default_and_rule() {
        let config: Config = toml::from_str(
            r#"
                [default_browser]
                path = "msedge.exe"
                args = ["--inprivate"]

                [[rules]]
                name = "Work"
                browser = "chrome.exe"
                days = ["Mon", "Tue"]
                time_range = ["08:30", "17:00"]
                domains = ["*.example.com"]
            "#,
        )
        .unwrap();

        assert_eq!(config.default_browser.args, ["--inprivate"]);
        assert_eq!(config.rules[0].browser, "chrome.exe");
    }

    #[test]
    fn accepts_a_string_default_browser() {
        let config: Config = toml::from_str(
            r#"
                default_browser = "firefox.exe"
            "#,
        )
        .unwrap();

        assert_eq!(config.default_browser.path, "firefox.exe");
        assert!(config.default_browser.args.is_empty());
    }
}
