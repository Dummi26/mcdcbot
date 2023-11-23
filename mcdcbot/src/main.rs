mod data;
mod embed;
mod getmyip;
mod settings;

use std::{collections::HashSet, env, sync::Arc, time::Duration};

use crate::{data::Data, settings::Settings};
use minecraft_manager::{
    events::{MinecraftServerEventType, MinecraftServerWarning},
    tasks::MinecraftServerTask,
    thread::MinecraftServerThread,
};
use poise::{futures_util::lock::Mutex, serenity_prelude as serenity};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command)]
async fn list(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say({
        let mut acc = format!("Available servers:");
        for server in ctx.data().servers.lock().await.iter() {
            let server = server.lock().await;
            acc.push_str("\n- ");
            if let Some(short) = &server.short {
                acc.push('(');
                acc.push_str(short);
                acc.push_str(") ");
            }
            acc.push_str(server.name.as_str());
        }
        acc
    })
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
async fn start(
    ctx: Context<'_>,
    #[description = "Server's name (see /list)"] srv: String,
) -> Result<(), Error> {
    // find server by name
    let servers_lock = ctx.data().servers.lock().await;
    let mut matching_server = None;
    for server in servers_lock.iter() {
        let server_lock = server.lock().await;
        if server_lock
            .short
            .as_ref()
            .is_some_and(|short| short == &srv)
        {
            matching_server = Some(Arc::clone(server));
            break;
        }
    }
    if matching_server.is_none() {
        for server in servers_lock.iter() {
            let server_lock = server.lock().await;
            if server_lock.name == srv {
                matching_server = Some(Arc::clone(server));
                break;
            }
        }
    }
    if let Some(server) = matching_server {
        let mut current_lock = ctx.data().current.lock().await;
        if let Some((current, _)) = current_lock.as_ref() {
            let current = current.lock().await;
            ctx.say(format!(
                "Already running '{}'! (stop the server before starting it)",
                current.name,
            ))
            .await?;
        } else {
            let server_lock = server.lock().await;
            let settings = ctx.data().settings.lock().await;
            _ = ctx
                .http()
                .send_message(
                    settings.channel_id_info,
                    &embed::server_started(
                        &server_lock.name,
                        Some(
                            getmyip::get_my_ip(&settings.get_my_ip_url1, &settings.get_my_ip_url2)
                                .await,
                        ),
                    ),
                )
                .await;
            if settings.send_start_stop_messages_in_chat {
                _ = ctx
                    .http()
                    .send_message(
                        settings.channel_id_chat,
                        &embed::server_started(&server_lock.name, None),
                    )
                    .await;
            }
            let thread = server_lock.settings.clone().spawn();
            drop(server_lock);
            *current_lock = Some((server, Arc::new(Mutex::new(Some(thread)))));
            ctx.say(format!("Starting...")).await?;
        }
    } else {
        ctx.say(format!("Can't find a server with that name!"))
            .await?;
    }
    Ok(())
}
#[poise::command(slash_command)]
async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let current_lock = ctx.data().current.lock().await;
    if let Some((_, thread)) = current_lock.as_ref() {
        _ = thread
            .lock()
            .await
            .as_ref()
            .unwrap()
            .clone_task_sender()
            .send_task(MinecraftServerTask::Stop);
        ctx.say(format!("Stopping...")).await?;
    } else {
        ctx.say(format!("Use /start to start a server first"))
            .await?;
    }
    Ok(())
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &poise::Event<'_>,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        poise::Event::Message { new_message } => {
            if !new_message.author.bot
                && new_message.channel_id.0 == data.settings.lock().await.channel_id_chat
            {
                let current_lock = data.current.lock().await;
                if let Some((_current, thread)) = current_lock.as_ref() {
                    let msg = new_message.content_safe(&ctx.cache);
                    let msg = msg
                        .replace("\\", "\\\\")
                        .replace("\n", "\\n")
                        .replace("\r", "\\r");
                    let author = new_message
                        .author_nick(&ctx)
                        .await
                        .unwrap_or_else(|| new_message.author.name.clone());
                    _ = thread
                        .lock()
                        .await
                        .as_ref()
                        .unwrap()
                        .clone_task_sender()
                        .send_task(MinecraftServerTask::RunCommand(format!(
                            "tellraw @a \"<{author}> {msg}\"",
                        )));
                } else {
                }
            }
        }
        _ => {}
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // read settings file
    let settings =
        Settings::from_file(env::var("McDcBotSettingsFile").unwrap_or(format!("settings.txt")))
            .unwrap();
    // read mc servers
    let mut servers = vec![];
    for file in std::fs::read_dir(env::var("McDcBotServersDir").unwrap_or(format!("servers")))
        .expect("Couldn't read servers dir, maybe specify the directory with the McDcBotServersDir env variable?") {
        let file = file.unwrap();
        let content = std::fs::read_to_string(file.path()).unwrap();
        let mut lines = content.lines().filter(|line| !line.trim().is_empty());
        let settings = minecraft_manager::MinecraftServerSettings::from_lines(&mut lines).unwrap();
        servers.push(data::MinecraftServer {
            name: file.file_name().to_string_lossy().into_owned(),
            short: None,
            settings
        });
    }
    let mut shorts = HashSet::new();
    for server in &mut servers {
        if let Some(ch) = server.name.trim().chars().next() {
            let ch = ch.to_lowercase();
            if shorts.insert(format!("{ch}")) {
                server.short = Some(format!("{ch}"));
                continue;
            }
        }
        let initials = server
            .name
            .trim()
            .split_whitespace()
            .filter_map(|v| v.chars().next().map(|c| c.to_uppercase()))
            .flatten()
            .collect::<String>();
        if !initials.is_empty() {
            if shorts.insert(initials.clone()) {
                server.short = Some(initials);
                continue;
            }
        }
    }
    // current server
    let current = Arc::new(Mutex::new(None));
    let current_thread = Arc::clone(&current);
    // start
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![list(), start(), stop()],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .token(env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN env var"))
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
        )
        .setup(|ctx, ready, framework: &poise::Framework<Data, Error>| {
            Box::pin(async move {
                ctx.idle().await;
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                eprintln!("Connected as '{}'.", ready.user.name);
                {
                    let ctx = ctx.clone();
                    let settings = settings.clone();
                    tokio::task::spawn(async move {
                        let sleep_time = Duration::from_millis(100);
                        let mut running = false;
                        loop {
                            tokio::time::sleep(sleep_time).await;
                            let mut current_lock: poise::futures_util::lock::MutexGuard<
                                '_,
                                Option<(
                                    Arc<Mutex<data::MinecraftServer>>,
                                    Arc<Mutex<Option<MinecraftServerThread>>>,
                                )>,
                            > = current_thread.lock().await;
                            if let Some((_current_server, current_thread_mutex)) =
                                current_lock.as_ref()
                            {
                                let mut current_thread_opt = current_thread_mutex.lock().await;
                                let current_thread = current_thread_opt.as_mut().unwrap();
                                current_thread.update();
                                for event in current_thread.handle_new_events() {
                                    match &event.event {
                                    MinecraftServerEventType::Warning(w) => match w {
                                        MinecraftServerWarning::CouldNotGetServerProcessStdio
                                        | MinecraftServerWarning::CantWriteToStdin(_) => {
                                            ctx.dnd().await;
                                        }
                                    },
                                    MinecraftServerEventType::JoinLeave(e) => {
                                        if settings.send_join_and_leave_messages {
                                        _ = ctx
                                            .http
                                            .send_message(
                                                settings.channel_id_chat,
                                                &embed::join_leave(
                                                    e
                                                ),
                                            )
                                            .await;
                                        }
                                    }
                                    MinecraftServerEventType::ChatMessage(e) => {
                                        _ = ctx
                                            .http
                                            .send_message(
                                                settings.channel_id_chat,
                                                &embed::chat_message(e),
                                            )
                                            .await;
                                    }
                                }
                                }
                                if current_thread.is_finished() {
                                    let cto = current_thread_opt.take().unwrap();
                                    let msg = embed::server_stopped(cto.get_stop_reason().ok());
                                    _ = ctx.http.send_message(settings.channel_id_info, &msg).await;
                                    if settings.send_start_stop_messages_in_chat {
                                        _ = ctx
                                            .http
                                            .send_message(settings.channel_id_chat, &msg)
                                            .await;
                                    }
                                    running = false;
                                    drop(current_thread_opt);
                                    *current_lock = None;
                                    ctx.idle().await;
                                } else if !running {
                                    running = true;
                                    ctx.online().await;
                                }
                            }
                        }
                    });
                }
                Ok(Data {
                    settings: Mutex::new(settings),
                    current,
                    servers: Mutex::new(
                        servers
                            .into_iter()
                            .map(|s| Arc::new(Mutex::new(s)))
                            .collect(),
                    ),
                })
            })
        });

    framework.run().await.unwrap();
}
