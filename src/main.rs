mod commands;
mod xp;
mod misc;
use serenity::model::voice::VoiceState;
use xp::xp::XpHandler;
use serenity::model::application::command::Command;
use serenity::builder::CreateEmbed;

use std::env;
use std::sync::Arc;
use mongodb::options::ClientOptions;
use serenity::async_trait;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::client::bridge::gateway::ShardManager;
use serenity::framework::standard::macros::group;
use serenity::framework::StandardFramework;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::model::channel::Message;
use tracing::{error, info};

use crate::commands::timer::*;
use crate::commands::skulls::*;
use crate::commands::xd::*;

pub struct ShardManagerContainer;

struct MongoDb;

impl TypeMapKey for MongoDb {
    type Value = Arc<mongodb::Client>;
}

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler {
    xp: XpHandler,
}

enum ResponseContent {
    Text(String),
    Embed(CreateEmbed),
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!(" {} online!", ready.user.name);

        Command::set_global_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    xp::xp_commands::xp_modify::register(command)
                })
                .create_application_command(|command| {
                    xp::xp_commands::xp_multiplier::register(command)
                })
                .create_application_command(|command| {
                    xp::xp_commands::xp_blacklist::register(command)
                })
                .create_application_command(|command| {
                    xp::xp_commands::xp_set_level::register(command)
                })
                .create_application_command(|command| {
                    xp::xp_commands::rank::register(command)
                })
                .create_application_command(|command| {
                    xp::xp_commands::leaderboard::register(command)
                })
                .create_application_command(|command| {
                    xp::xp_commands::xp_info::register(command)
                })
        }).await.expect("Error making app commands");

        xp::xp::initialize_multiplier(&ctx).await;
        xp::xp::initialize_blacklist(&ctx).await;

        tokio::spawn(async move {
            xp::xp::voice_hash_loop(&ctx.clone()).await;
        });
    }
    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed!");
    }

    async fn message(&self, ctx: Context, msg: Message) {
        self.xp.message(ctx, msg).await;
    } 

    async fn voice_state_update(&self, ctx: Context, old: Option<VoiceState>, new: VoiceState) {
        misc::vc_icon::vc_icon(&ctx, &old, &new).await;
        match old {
            Some(old) => {
                if new.channel_id == None {
                    xp::xp::remove_voice_hash(&old).await;
                } else {
                    xp::xp::update_voice_channel_id(&new).await;
                }
        },
            None =>  xp::xp::voice_cooldown(&ctx, &new).await
        }
    } 
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let res_content = match command.data.name.as_str() {
                "xp_modify" => ResponseContent::Text(xp::xp_commands::xp_modify::run(&command.data.options, &ctx, &command.guild_id.unwrap(), &command.channel_id).await),
                "xp_multiplier" => ResponseContent::Text(xp::xp_commands::xp_multiplier::run(&command.data.options, &ctx, &command.guild_id.unwrap()).await),
                "xp_blacklist" => ResponseContent::Text(xp::xp_commands::xp_blacklist::run(&command.data.options, &ctx, &command.guild_id.unwrap()).await),
                "xp_set_level" => ResponseContent::Text(xp::xp_commands::xp_set_level::run(&command.data.options, &ctx, &command.guild_id.unwrap(), &command.channel_id).await),
                "xp_info" => ResponseContent::Embed(xp::xp_commands::xp_info::run(&command.data.options, &ctx, &command.guild_id.unwrap()).await),
                "rank" => ResponseContent::Embed(xp::xp_commands::rank::run(&command.data.options, &ctx, &command.member.clone().unwrap()).await),
                "leaderboard" => ResponseContent::Embed(xp::xp_commands::leaderboard::run(&command.data.options, &ctx, &command.guild_id.unwrap()).await),
                _ => ResponseContent::Text("not implemented :(".to_string()),
            };

            if let Err(e) = command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message|
                        match res_content {
                            ResponseContent::Text(text) => message.content(text),
                            ResponseContent::Embed(embed) => message.add_embed(embed),
                        })
            }).await
        {
            println!("Cannot respond to slash command: {}", e);
        }
        }
    }
}

#[group]
#[commands(timer, skulls, xd)]
struct General;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");
    tracing_subscriber::fmt::init();
    let token = env::var("DISCORD_TOKEN").expect("No bot token!");

    let framework = StandardFramework::new().configure(|c| c.prefix(".")).group(&GENERAL_GROUP).bucket("basic", |b| b.delay(15).limit(1)).await;

    let handler = Handler {
        xp: XpHandler,
    };

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_VOICE_STATES;
    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(handler)
        .await
        .expect("Error creating the client! :(");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    }

    {
        let mut data = client.data.write().await;
        if let Ok(client_option) = ClientOptions::parse(format!("mongodb+srv://neyasbot:{}@digitalart.k4xqkao.mongodb.net/?retryWrites=true&w=majority", env::var("MONGO_PASSWORD").expect("No mongo password!"))).await {
            let client = mongodb::Client::with_options(client_option).unwrap();
            data.insert::<MongoDb>(Arc::new(client));
        }
    }

    let shard_manager = client.shard_manager.clone();
    
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    };
}