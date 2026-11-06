//! Native worker: reminders and recurrence work while webviews are hidden.
use parking_lot::Mutex;
use tauri::{Emitter, Manager};

use crate::error::{Error, Result};
use crate::model::now_ms;
use crate::store::Store;

#[derive(Default)]
pub struct Status(pub Mutex<Option<String>>);

fn set_status(app: &tauri::AppHandle, error: Option<String>) {
    let status = app.state::<Status>();
    let mut current = status.0.lock();
    if *current != error {
        *current = error.clone();
        let _ = app.emit("reminder-status", error);
    }
}

pub fn start(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        let mut failure = None;
        let mut attempted = false;
        let result = app.state::<Store>().process_time(now_ms(), |card, task| {
            attempted = true;
            let result = show(&format!("Jotter · {card}"), task);
            if let Err(err) = &result {
                failure = Some(err.to_string());
            }
            result
        });
        match result {
            Ok(true) => {
                let _ = app.emit("workspace-changed", ());
            }
            Ok(false) => {}
            Err(err) => {
                failure = Some(format!("保存提醒状态失败：{err}"));
                log::warn!("scheduler failed: {err}");
            }
        }
        let failed = failure.is_some();
        if attempted || failed {
            set_status(&app, failure);
        }
        // Wake after resume; missed periods/reminders are reconciled from disk.
        std::thread::sleep(std::time::Duration::from_secs(if failed { 60 } else { 15 }));
    });
}

#[tauri::command]
pub fn get_reminder_status(app: tauri::AppHandle) -> Option<String> {
    app.state::<Status>().0.lock().clone()
}

#[tauri::command]
pub async fn test_notification(app: tauri::AppHandle) -> Result<()> {
    let result = tauri::async_runtime::spawn_blocking(|| {
        show(
            "Jotter 提醒测试",
            "通知已送达。任务提醒会在设定时间自动发送。",
        )
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))?;
    set_status(&app, result.as_ref().err().map(ToString::to_string));
    result
}

#[cfg(windows)]
fn show(title: &str, body: &str) -> Result<()> {
    use windows::core::HSTRING;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
    use windows::UI::Notifications::{
        NotificationSetting, ToastNotification, ToastNotificationManager,
    };
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    // An unpackaged/portable Windows 11 app needs a registered sender identity.
    // Per-user registration works without an installer or administrator rights.
    const APP_ID: &str = "com.ztcools.jotter";
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(r"Software\Classes\AppUserModelId\com.ztcools.jotter")?;
    key.set_value("DisplayName", &"Jotter")?;

    fn native_error(err: windows::core::Error) -> Error {
        Error::Invalid(format!("Windows 通知失败：{err}"))
    }
    // Every caller uses a dedicated background thread; balance COM on that thread.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
        .ok()
        .map_err(native_error)?;
    struct Com;
    impl Drop for Com {
        fn drop(&mut self) {
            unsafe { CoUninitialize() };
        }
    }
    let _com = Com;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID))
        .map_err(native_error)?;
    if notifier.Setting().map_err(native_error)? != NotificationSetting::Enabled {
        return Err(Error::Invalid(
            "Windows 已禁用 Jotter 通知，请在系统设置 → 通知中开启".into(),
        ));
    }
    let xml = XmlDocument::new().map_err(native_error)?;
    xml.LoadXml(&HSTRING::from(toast_xml(title, body)))
        .map_err(native_error)?;
    let toast = ToastNotification::CreateToastNotification(&xml).map_err(native_error)?;
    notifier.Show(&toast).map_err(native_error)?;
    Ok(())
}

#[cfg(not(windows))]
fn show(_title: &str, _body: &str) -> Result<()> {
    Err(Error::Invalid("系统通知目前仅支持 Windows".into()))
}

#[cfg(any(windows, test))]
fn toast_xml(title: &str, body: &str) -> String {
    fn escape(text: &str, max: usize) -> String {
        text.chars()
            .filter(|c| !c.is_control() || *c == '\n')
            .take(max)
            .collect::<String>()
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
    format!("<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>", escape(title, 100), escape(body, 240))
}

#[cfg(test)]
mod tests {
    #[test]
    fn task_text_cannot_inject_notification_xml() {
        let xml = super::toast_xml("A & B", "<text>\"任务\"</text>\0");
        assert!(xml.contains("A &amp; B"));
        assert!(xml.contains("&lt;text&gt;&quot;任务&quot;&lt;/text&gt;"));
        assert!(!xml.contains('\0'));
    }
}
