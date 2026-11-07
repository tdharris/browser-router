#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]
#![cfg_attr(not(windows), allow(dead_code))]

mod config;
#[cfg(windows)]
mod registry;
mod router;

#[cfg(windows)]
fn main() {
    if let Err(error) = windows_main() {
        log_error(&error);
        console_line(&format!("Error: {error}"));
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("browser-router runs on Windows; this host can run its tests and cross-check it.");
}

#[cfg(windows)]
fn windows_main() -> Result<(), String> {
    use std::env;

    let mut arguments = env::args_os();
    let _executable = arguments.next();
    let Some(first) = arguments.next() else {
        attach_parent_console();
        console_line("Usage: browser-router.exe <URL> | --register | --unregister");
        return Ok(());
    };

    if first == "--register" {
        attach_parent_console();
        let executable =
            env::current_exe().map_err(|error| format!("cannot locate executable: {error}"))?;
        registry::register(&executable).map_err(|error| format!("registration failed: {error}"))?;

        if let Err(error) = config::load_or_create() {
            console_line(&format!("Warning: {error}"));
        }
        console_line("Browser Router was registered for the current user.");
        console_line("Open Settings with: start ms-settings:defaultapps?app=BrowserRouter");
        console_line("Then assign Browser Router to both HTTP and HTTPS.");
        return Ok(());
    }

    if first == "--unregister" {
        attach_parent_console();
        registry::unregister().map_err(|error| format!("unregistration failed: {error}"))?;
        console_line("Browser Router was unregistered for the current user.");
        return Ok(());
    }

    if arguments.next().is_some() {
        return Err("expected exactly one URL argument".to_owned());
    }

    let url = first.to_string_lossy();
    let url = router::validate_url(&url)
        .ok_or_else(|| "refusing to open an invalid or non-HTTP(S) URL".to_owned())?;

    if env::var_os("BROWSER_ROUTER_SHELL_FALLBACK").is_some() {
        return open_in_edge(url);
    }

    let config = config::load_or_create().unwrap_or_else(|error| {
        log_error(&error);
        config::Config::default()
    });
    let destination = router::select_destination(&config, url, router::local_moment());

    if let Err(error) = router::spawn_destination(destination, url) {
        log_error(&format!(
            "could not start browser '{}': {error}; using Windows URL association",
            destination.path
        ));
        shell_fallback(url)?;
    }

    Ok(())
}

#[cfg(windows)]
fn shell_fallback(url: &str) -> Result<(), String> {
    // SAFETY: browser-router is single-threaded and changes the environment before
    // starting any fallback process. The flag prevents recursive dispatch if this
    // program is still the Windows handler for HTTP and HTTPS.
    unsafe { std::env::set_var("BROWSER_ROUTER_SHELL_FALLBACK", "1") };

    if shell_execute(url) {
        Ok(())
    } else {
        open_in_edge(url)
    }
}

#[cfg(windows)]
fn open_in_edge(url: &str) -> Result<(), String> {
    let edge = router::default_edge();
    let destination = router::Destination {
        path: &edge.path,
        args: &edge.args,
    };
    if router::spawn_destination(destination, url).is_ok() {
        return Ok(());
    }

    let edge_uri = format!("microsoft-edge:{url}");
    shell_execute(&edge_uri)
        .then_some(())
        .ok_or_else(|| "both the configured browser and Microsoft Edge failed to start".to_owned())
}

#[cfg(windows)]
fn shell_execute(target: &str) -> bool {
    use std::iter;
    use std::ptr;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let operation: Vec<u16> = "open".encode_utf16().chain(iter::once(0)).collect();
    let target: Vec<u16> = target.encode_utf16().chain(iter::once(0)).collect();

    // SAFETY: All pointers refer to NUL-terminated UTF-16 buffers that remain
    // alive for the duration of the call. The other optional pointers are null.
    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    result as isize > 32
}

#[cfg(windows)]
fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};

    // SAFETY: ATTACH_PARENT_PROCESS is the documented sentinel for attaching to
    // the caller's console. Failure simply means there is no console to attach.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(windows)]
fn console_line(message: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;

    if let Ok(mut console) = OpenOptions::new().write(true).open("CONOUT$") {
        let _ = writeln!(console, "{message}");
    }
}

#[cfg(windows)]
fn log_error(message: &str) {
    use std::fs::{self, OpenOptions};
    use std::io::Write;

    let Some(path) = config::config_path() else {
        return;
    };
    let Some(directory) = path.parent() else {
        return;
    };
    if fs::create_dir_all(directory).is_err() {
        return;
    }
    if let Ok(mut log) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("browser-router.log"))
    {
        let _ = writeln!(log, "{message}");
    }
}
