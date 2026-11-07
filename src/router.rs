use crate::config::{BrowserLaunch, Config, Rule};
use regex::Regex;
use std::io;
use std::process::{Command, Stdio};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weekday {
    Sun,
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalMoment {
    pub weekday: Weekday,
    pub minute_of_day: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct Destination<'a> {
    pub path: &'a str,
    pub args: &'a [String],
}

pub fn validate_url(url: &str) -> Option<&str> {
    if url.is_empty()
        || url.len() > 32_767
        || url.contains('"')
        || url.chars().any(char::is_control)
    {
        return None;
    }

    let scheme_end = url.find("://")?;
    let scheme = &url[..scheme_end];
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }

    extract_host(url).map(|_| url)
}

pub fn extract_host(url: &str) -> Option<String> {
    let scheme_end = url.find("://")?;
    let remainder = &url[scheme_end + 3..];
    let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
    let authority = &remainder[..authority_end];
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, tail)| tail);

    let host = if let Some(bracketed) = host_port.strip_prefix('[') {
        let closing = bracketed.find(']')?;
        &bracketed[..closing]
    } else if let Some((host, port)) = host_port.rsplit_once(':') {
        if port.bytes().all(|byte| byte.is_ascii_digit()) {
            host
        } else {
            host_port
        }
    } else {
        host_port
    };

    if host.is_empty() {
        None
    } else {
        Some(host.trim_end_matches('.').to_ascii_lowercase())
    }
}

pub fn select_destination<'a>(config: &'a Config, url: &str, now: LocalMoment) -> Destination<'a> {
    let host = extract_host(url).unwrap_or_default();

    for rule in &config.rules {
        if rule_matches(rule, &host, now) {
            return Destination {
                path: &rule.browser,
                args: &rule.args,
            };
        }
    }

    Destination {
        path: &config.default_browser.path,
        args: &config.default_browser.args,
    }
}

fn rule_matches(rule: &Rule, host: &str, now: LocalMoment) -> bool {
    day_matches(&rule.days, now.weekday)
        && time_matches(rule.time_range.as_ref(), now.minute_of_day)
        && domain_matches(&rule.domains, host)
}

fn day_matches(days: &[String], weekday: Weekday) -> bool {
    days.is_empty()
        || days
            .iter()
            .filter_map(|day| parse_weekday(day))
            .any(|day| day == weekday)
}

fn parse_weekday(value: &str) -> Option<Weekday> {
    match value.to_ascii_lowercase().as_str() {
        "sun" | "sunday" => Some(Weekday::Sun),
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tues" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thur" | "thurs" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        _ => None,
    }
}

fn time_matches(time_range: Option<&[String; 2]>, now: u16) -> bool {
    let Some([start, end]) = time_range else {
        return true;
    };
    let (Some(start), Some(end)) = (parse_time(start), parse_time(end)) else {
        return false;
    };

    if start == end {
        true
    } else if start < end {
        (start..end).contains(&now)
    } else {
        now >= start || now < end
    }
}

fn parse_time(value: &str) -> Option<u16> {
    if value.len() != 5 || value.as_bytes()[2] != b':' {
        return None;
    }
    let hour: u16 = value[..2].parse().ok()?;
    let minute: u16 = value[3..].parse().ok()?;
    (hour < 24 && minute < 60).then_some(hour * 60 + minute)
}

fn domain_matches(patterns: &[String], host: &str) -> bool {
    patterns.is_empty()
        || patterns
            .iter()
            .any(|pattern| domain_pattern_matches(pattern, host))
}

fn domain_pattern_matches(pattern: &str, host: &str) -> bool {
    let regex = if let Some(regex) = pattern.strip_prefix("regex:") {
        Some(regex)
    } else if pattern.len() >= 2 && pattern.starts_with('/') && pattern.ends_with('/') {
        Some(&pattern[1..pattern.len() - 1])
    } else {
        None
    };

    if let Some(regex) = regex {
        // Domain names are ASCII (internationalized names arrive as Punycode).
        // Disabling Unicode here keeps case-insensitive regexes lightweight.
        let expression = format!("(?i-u:{regex})");
        Regex::new(&expression).is_ok_and(|regex| regex.is_match(host))
    } else {
        glob_matches(pattern.as_bytes(), host.as_bytes())
    }
}

fn glob_matches(pattern: &[u8], value: &[u8]) -> bool {
    let (mut pattern_index, mut value_index) = (0, 0);
    let (mut star_index, mut star_value_index) = (None, 0);

    while value_index < value.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?'
                || pattern[pattern_index].eq_ignore_ascii_case(&value[value_index]))
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            pattern_index += 1;
            star_value_index = value_index;
        } else if let Some(star) = star_index {
            pattern_index = star + 1;
            star_value_index += 1;
            value_index = star_value_index;
        } else {
            return false;
        }
    }

    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

pub fn spawn_destination(destination: Destination<'_>, url: &str) -> io::Result<()> {
    if destination.path.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "browser executable path is empty",
        ));
    }

    #[cfg(windows)]
    let executable = resolve_edge_executable(destination.path);
    #[cfg(not(windows))]
    let executable = std::path::PathBuf::from(destination.path);

    let mut command = Command::new(executable);
    command
        .args(destination.args)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, DETACHED_PROCESS};
        command.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }

    command.spawn().map(|_| ())
}

#[cfg(windows)]
fn resolve_edge_executable(path: &str) -> std::path::PathBuf {
    use std::env;
    use std::path::PathBuf;

    if !path.eq_ignore_ascii_case("msedge.exe") {
        return PathBuf::from(path);
    }

    let candidates = [
        env::var_os("ProgramFiles(x86)")
            .map(PathBuf::from)
            .map(|root| root.join(r"Microsoft\Edge\Application\msedge.exe")),
        env::var_os("ProgramFiles")
            .map(PathBuf::from)
            .map(|root| root.join(r"Microsoft\Edge\Application\msedge.exe")),
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|root| root.join(r"Microsoft\Edge\Application\msedge.exe")),
    ];

    candidates
        .into_iter()
        .flatten()
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| PathBuf::from(path))
}

pub fn default_edge() -> BrowserLaunch {
    BrowserLaunch {
        path: "msedge.exe".to_owned(),
        args: Vec::new(),
    }
}

#[cfg(windows)]
pub fn local_moment() -> LocalMoment {
    use std::mem::MaybeUninit;
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;

    let mut system_time = MaybeUninit::<SYSTEMTIME>::uninit();
    // SAFETY: GetLocalTime initializes the SYSTEMTIME structure passed to it.
    unsafe { GetLocalTime(system_time.as_mut_ptr()) };
    // SAFETY: The call above initialized the value.
    let system_time = unsafe { system_time.assume_init() };
    let weekday = match system_time.wDayOfWeek {
        0 => Weekday::Sun,
        1 => Weekday::Mon,
        2 => Weekday::Tue,
        3 => Weekday::Wed,
        4 => Weekday::Thu,
        5 => Weekday::Fri,
        _ => Weekday::Sat,
    };

    LocalMoment {
        weekday,
        minute_of_day: system_time.wHour * 60 + system_time.wMinute,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monday_at(hour: u16, minute: u16) -> LocalMoment {
        LocalMoment {
            weekday: Weekday::Mon,
            minute_of_day: hour * 60 + minute,
        }
    }

    #[test]
    fn validates_only_http_urls_with_hosts() {
        assert!(validate_url("https://example.com/path?q=1").is_some());
        assert!(validate_url("HTTP://EXAMPLE.COM").is_some());
        assert!(validate_url("file:///C:/secret.txt").is_none());
        assert!(validate_url("https://").is_none());
        assert!(validate_url("https://example.com/\nmalicious").is_none());
    }

    #[test]
    fn extracts_hostname_without_credentials_port_or_trailing_dot() {
        assert_eq!(
            extract_host("https://user:pass@Sub.Example.com.:8443/path").as_deref(),
            Some("sub.example.com")
        );
        assert_eq!(
            extract_host("http://[::1]:8080/test").as_deref(),
            Some("::1")
        );
    }

    #[test]
    fn glob_and_regex_patterns_match_case_insensitively() {
        assert!(domain_pattern_matches("*.example.com", "team.example.com"));
        assert!(domain_pattern_matches("*CORP.com*", "mycorp.com.au"));
        assert!(domain_pattern_matches(
            r"regex:^jira[0-9]+\.example\.com$",
            "jira42.example.com"
        ));
        assert!(!domain_pattern_matches("*.example.com", "example.com"));
    }

    #[test]
    fn time_ranges_are_end_exclusive_and_can_cross_midnight() {
        let daytime = [String::from("08:00"), String::from("18:00")];
        assert!(time_matches(Some(&daytime), 8 * 60));
        assert!(!time_matches(Some(&daytime), 18 * 60));

        let overnight = [String::from("22:00"), String::from("02:00")];
        assert!(time_matches(Some(&overnight), 23 * 60));
        assert!(time_matches(Some(&overnight), 60));
        assert!(!time_matches(Some(&overnight), 12 * 60));
    }

    #[test]
    fn first_matching_rule_wins() {
        let config = Config {
            default_browser: BrowserLaunch {
                path: "edge.exe".into(),
                args: Vec::new(),
            },
            rules: vec![
                Rule {
                    name: "first".into(),
                    browser: "chrome.exe".into(),
                    args: vec!["--profile-directory=Work".into()],
                    days: vec!["Mon".into()],
                    time_range: Some(["08:00".into(), "18:00".into()]),
                    domains: vec!["*.example.com".into()],
                },
                Rule {
                    name: "second".into(),
                    browser: "firefox.exe".into(),
                    args: Vec::new(),
                    days: Vec::new(),
                    time_range: None,
                    domains: Vec::new(),
                },
            ],
        };

        let destination = select_destination(&config, "https://work.example.com", monday_at(9, 0));
        assert_eq!(destination.path, "chrome.exe");
    }
}
