use std::sync::Arc;

use minecraft_manager::{thread::MinecraftServerThread, MinecraftServerSettings};
use poise::futures_util::lock::Mutex;

use crate::settings::Settings;

pub struct Data {
    pub settings: Mutex<Settings>,
    pub servers: Mutex<Vec<Arc<Mutex<MinecraftServer>>>>,
    pub current: Arc<
        Mutex<
            Option<(
                Arc<Mutex<MinecraftServer>>,
                Arc<Mutex<Option<MinecraftServerThread>>>,
            )>,
        >,
    >,
}

pub struct MinecraftServer {
    pub name: String,
    pub short: Option<String>,
    pub settings: MinecraftServerSettings,
}
