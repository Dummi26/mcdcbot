pub async fn get_my_ip(url1: &str, url2: &str) -> String {
    let ip1 = get(url1).await;
    let ip2 = get(url2).await;
    match (ip1, ip2) {
        (Some(ip1), Some(ip2)) => {
            if ip1 == ip2 {
                ip1.to_string()
            } else {
                format!("{ip1} / {ip2}")
            }
        }
        (Some(ip), None) | (None, Some(ip)) => ip.to_string(),
        (None, None) => format!("unknown"),
    }
}

async fn get(url: &str) -> Option<String> {
    if url.is_empty() {
        return None;
    }
    Some(
        reqwest::get(url)
            .await
            .ok()?
            .text()
            .await
            .ok()?
            .trim()
            .to_string(),
    )
}
