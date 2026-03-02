use anyhow::Result;

/// Send a native desktop notification (macOS + Windows)
pub fn send_notification(title: &str, body: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        mac_notification_sys::set_application("com.ridgeline.prterminal")
            .map_err(|e| anyhow::anyhow!("failed to set notification app: {}", e))?;
        mac_notification_sys::send_notification(title, None, body, None)
            .map_err(|e| anyhow::anyhow!("failed to send notification: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        use winrt_notification::{Duration, Toast};
        Toast::new(Toast::POWERSHELL_APP_ID)
            .title(title)
            .text1(body)
            .duration(Duration::Short)
            .show()
            .map_err(|e| anyhow::anyhow!("failed to send notification: {}", e))?;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (title, body);
    }

    Ok(())
}
