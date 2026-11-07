use std::io;
use std::path::Path;
use std::ptr;
use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

const CLIENT_KEY: &str = r"Software\Clients\StartMenuInternet\BrowserRouter";
const REGISTERED_APPLICATIONS_KEY: &str = r"Software\RegisteredApplications";
const URL_CLASS_KEY: &str = r"Software\Classes\BrowserRouterURL";
const CAPABILITIES_PATH: &str = r"Software\Clients\StartMenuInternet\BrowserRouter\Capabilities";

pub fn register(executable: &Path) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let executable = executable.to_string_lossy();
    let command = format!(r#""{executable}" "%1""#);
    let icon = format!("{executable},0");

    let (client, _) = hkcu.create_subkey(CLIENT_KEY)?;
    client.set_value("", &"Browser Router")?;

    let (default_icon, _) = client.create_subkey("DefaultIcon")?;
    default_icon.set_value("", &icon)?;

    let (client_command, _) = client.create_subkey(r"shell\open\command")?;
    client_command.set_value("", &command)?;

    let (capabilities, _) = client.create_subkey("Capabilities")?;
    capabilities.set_value("ApplicationName", &"Browser Router")?;
    capabilities.set_value(
        "ApplicationDescription",
        &"Routes web links to a browser according to configurable schedules and domains.",
    )?;
    capabilities.set_value("ApplicationIcon", &icon)?;

    let (associations, _) = capabilities.create_subkey("URLAssociations")?;
    associations.set_value("http", &"BrowserRouterURL")?;
    associations.set_value("https", &"BrowserRouterURL")?;

    let (registered, _) = hkcu.create_subkey(REGISTERED_APPLICATIONS_KEY)?;
    registered.set_value("BrowserRouter", &CAPABILITIES_PATH)?;

    let (url_class, _) = hkcu.create_subkey(URL_CLASS_KEY)?;
    url_class.set_value("", &"Browser Router URL")?;
    url_class.set_value("URL Protocol", &"")?;

    let (url_icon, _) = url_class.create_subkey("DefaultIcon")?;
    url_icon.set_value("", &icon)?;

    let (url_command, _) = url_class.create_subkey(r"shell\open\command")?;
    url_command.set_value("", &command)?;

    notify_association_change();
    Ok(())
}

pub fn unregister() -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    remove_tree_if_present(&hkcu, CLIENT_KEY)?;
    remove_tree_if_present(&hkcu, URL_CLASS_KEY)?;

    match hkcu.open_subkey_with_flags(REGISTERED_APPLICATIONS_KEY, winreg::enums::KEY_SET_VALUE) {
        Ok(registered) => match registered.delete_value("BrowserRouter") {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    notify_association_change();
    Ok(())
}

fn remove_tree_if_present(root: &RegKey, path: &str) -> io::Result<()> {
    match root.delete_subkey_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn notify_association_change() {
    // SAFETY: SHCNE_ASSOCCHANGED with SHCNF_IDLIST requires both item pointers
    // to be null and only broadcasts that associations should be refreshed.
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED as i32,
            SHCNF_IDLIST,
            ptr::null(),
            ptr::null(),
        );
    }
}
