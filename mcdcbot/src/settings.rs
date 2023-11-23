use std::path::Path;

#[derive(Clone)]
pub struct Settings {
    pub channel_id_info: u64,
    pub channel_id_chat: u64,
    pub send_join_and_leave_messages: bool,
    pub send_start_stop_messages_in_chat: bool,
    pub get_my_ip_url1: String,
    pub get_my_ip_url2: String,
}

impl Settings {
    pub fn from_file(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let file = std::fs::read_to_string(path)?;
        let mut cii = None;
        let mut cic = None;
        let mut send_join_and_leave_messages = false;
        let mut send_start_stop_messages_in_chat = false;
        let mut get_my_ip_url1 = String::new();
        let mut get_my_ip_url2 = String::new();
        for (name, value) in file
            .lines()
            .map(|line| line.split_once("=").unwrap_or((line, "")))
        {
            match name {
                "channel_id_info" => {
                    cii = value.trim().parse().ok();
                }
                "channel_id_chat" => {
                    cic = value.trim().parse().ok();
                }
                "get_my_ip_url1" => get_my_ip_url1 = value.trim().to_owned(),
                "get_my_ip_url2" => get_my_ip_url2 = value.trim().to_owned(),
                "send_join_and_leave_messages" => send_join_and_leave_messages = value != "false",
                "send_start_stop_messages_in_chat" => {
                    send_start_stop_messages_in_chat = value != "false"
                }
                _ => {}
            }
        }
        Ok(Self {
            channel_id_info: cii.expect("[settings] Missing `channel_id_info`"),
            channel_id_chat: cic.expect("[settings] Missing `channel_id_chat`"),
            send_join_and_leave_messages,
            send_start_stop_messages_in_chat,
            get_my_ip_url1,
            get_my_ip_url2,
        })
    }
}
