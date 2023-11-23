use minecraft_manager::{
    chat::ChatMessage, events::JoinLeaveEvent, threaded::MinecraftServerStopReason,
};
use poise::serenity_prelude::{json::json, json::Value};

pub fn chat_message(e: &ChatMessage) -> Value {
    json!({
        "embeds": [{
            "title": e.author,
            "description": e.message
        }]
    })
}
pub fn join_leave(e: &JoinLeaveEvent) -> Value {
    json!({
        "embeds": [{
            "description": if e.joined {
                format!("{} joined", e.username)
            } else {
                format!("{} left", e.username)
            },
        }]
    })
}

pub fn server_started(name: &str, ip: Option<String>) -> Value {
    json!({
        "embeds": [{
            "color": 26880,
            "title": name,
            "description": if let Some(ip) = ip {
                format!("Server was started, IP: {ip}")
            } else {
                format!("Server was started")
            },
        }]
    })
}
pub fn server_stopped(reason: Option<MinecraftServerStopReason>) -> Value {
    if let Some(reason) = reason {
        json!({
            "embeds": [{
                "color": 6881280,
                "title": reason.to_string(),
            }]
        })
    } else {
        json!({
            "embeds": [{
                "color": 6881280,
                "title": "Stopped.",
            }]
        })
    }
}
