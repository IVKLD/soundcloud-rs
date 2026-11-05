pub fn resize_artwork_url(url: &str, size: &str) -> String {
    let size_part = if size.starts_with('-') {
        size.to_string()
    } else {
        format!("-{size}")
    };
    if url.contains("-large.") {
        url.replace("-large.", &format!("{size_part}."))
    } else if url.contains("-large") {
        url.replace("-large", &size_part)
    } else {
        url.to_string()
    }
}

pub fn is_soundcloud_url(url: &str) -> bool {
    if let Ok(parsed) = reqwest::Url::parse(url) {
        if let Some(host) = parsed.host_str() {
            let host = host.to_lowercase();
            return host == "soundcloud.com"
                || host.ends_with(".soundcloud.com")
                || host == "on.soundcloud.com"
                || host == "soundcloud.cloud"
                || host.ends_with(".soundcloud.cloud");
        }
    }
    let lower = url.to_lowercase();
    lower.contains("soundcloud.com")
        || lower.contains("on.soundcloud.com")
        || lower.contains("soundcloud.cloud")
}
