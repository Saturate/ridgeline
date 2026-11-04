#[cfg(target_os = "macos")]
pub fn send_with_url(title: &str, body: &str, url: &str) {
    use mac_notification_sys::{MainButton, Notification, NotificationResponse};

    let title = title.to_string();
    let body = body.to_string();
    let url = url.to_string();

    std::thread::spawn(move || {
        let _ = mac_notification_sys::set_application("com.ridgeline.app");

        let mut n = Notification::new();
        n.title(&title)
            .message(&body)
            .main_button(MainButton::SingleAction("View PR"));

        match n.send() {
            Ok(
                NotificationResponse::Click | NotificationResponse::ActionButton(_),
            ) => {
                let _ = open::that(&url);
            }
            _ => {}
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn send_with_url(title: &str, body: &str, _url: &str) {
    send_simple(title, body);
}

pub fn send_simple(title: &str, body: &str) {
    let title = title.to_string();
    let body = body.to_string();

    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new()
            .summary(&title)
            .body(&body)
            .show();
    });
}
