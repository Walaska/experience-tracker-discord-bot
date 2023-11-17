use serenity::builder::{CreateApplicationCommand, CreateEmbed};
use serenity::futures::StreamExt;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{CommandDataOption, CommandDataOptionValue};
use serenity::model::permissions::Permissions;
use serenity::client::Context;
use serenity::model::prelude::GuildId;
use mongodb::options::FindOptions;
use mongodb::bson::{doc, Document};

use crate::{MongoDb, xp};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("leaderboard")
        .description("Check the leaderboard!")
}

pub async fn run(options: &[CommandDataOption], ctx: &Context, guild_id: &GuildId) -> CreateEmbed {
    let mut embed = CreateEmbed::default();
    let mut level = 0;
    let mut rank = 1;
    let mut user_name = "".to_string();

    let client = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
    };
    let db = client.database("xp");
    let collection = db.collection::<Document>("users");

    let find_options = FindOptions::builder()
        .sort(doc! { "xp": -1 })
        .limit(10)
        .build();

    let mut document = collection.find(doc! {"server_id": format!("{}", guild_id)}, find_options).await;
    while let Some(doc) = document.as_mut().unwrap().next().await {
        match doc {
            Ok(res) => {
                if let Ok(level_value) = res.get_i32("level") {level = level_value}
                if let Ok(user_id) = res.get("user_id").unwrap().as_str().unwrap_or("1").parse::<u64>() {
                    user_name = guild_id.member(&ctx.http, user_id).await.unwrap().user.name;
                }
                embed.field("\u{200B}", format!("{}. {} - Level {}", rank, user_name, level), false)
                    .color(0x323337);
            }
            Err(e) => {
                eprintln!("Error while inspecting doc: {}", e);
            }
        }
        rank += 1;
    }

    return embed;
}   