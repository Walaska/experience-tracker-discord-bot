use serenity::builder::CreateApplicationCommand;
use serenity::json::prelude::to_string;
use serenity::model::prelude::GuildId;
use serenity::futures::StreamExt;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{CommandDataOption, CommandDataOptionValue};
use serenity::model::permissions::Permissions;
use serenity::client::Context;
use serenity::builder::CreateEmbed;
use mongodb::bson::{doc, Document};

use crate::{MongoDb, xp};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("xp_info")
        .description("Check currently set multipliers, blacklists. Admin only.")
        .default_member_permissions(Permissions::ADMINISTRATOR)
}

pub async fn run(options: &[CommandDataOption], ctx: &Context, guild_id: &GuildId) -> CreateEmbed {
    let mut embed = CreateEmbed::default();
    let client = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
    };
    let db = client.database("xp");

    embed.title("Info");
    let mut field_string = "".to_string();
    let collection = db.collection::<Document>("channel_multipliers");
    let mut document = collection.find(doc! {"guild": format!("{}", guild_id)}, None).await;
    while let Some(doc) = document.as_mut().unwrap().next().await {
        match doc {
            Ok(res) => {
                let mut amount = 0;
                if let Ok(amount_value) = res.get("amount").unwrap().as_str().unwrap_or("1").parse::<u32>() {amount = amount_value}
                if let Ok(channel_id) = res.get("channel_id").unwrap().as_str().unwrap_or("1").parse::<u64>() {
                    let channel_name = ctx.http.get_channel(channel_id).await.unwrap().guild().unwrap().name().to_string();
                    field_string += format!("{} > **{}x**\n", channel_name, amount).as_str();
                }
            }
            Err(e) => {
                eprintln!("Error while inspecting doc: {}", e);
            }
        }
    }
    embed.field("Channel multipliers", field_string, true);
    field_string = "".to_string();

    let collection = db.collection::<Document>("role_multipliers");
    let mut document = collection.find(doc! {"guild": format!("{}", guild_id)}, None).await;
    while let Some(doc) = document.as_mut().unwrap().next().await {
        match doc {
            Ok(res) => {
                let mut amount = 0;
                if let Ok(amount_value) = res.get("amount").unwrap().as_str().unwrap_or("1").parse::<u32>() {amount = amount_value}
                if let Ok(role_id) = res.get("role_id").unwrap().as_str().unwrap_or("1").parse::<u64>() {
                    field_string += format!("{} > **{}x**\n", ctx.cache.role(guild_id, role_id).unwrap().name, amount).as_str();
                }
            }
            Err(e) => {
                eprintln!("Error while inspecting doc: {}", e);
            }
        }
    }
    embed.field("Role multipliers", field_string, true);
    field_string = "".to_string();

    let collection = db.collection::<Document>("blacklist");
    let mut document = collection.find(doc! {"guild": format!("{}", guild_id)}, None).await;
    while let Some(doc) = document.as_mut().unwrap().next().await {
        match doc {
            Ok(res) => {
                if let Ok(blacklisted_id) = res.get("blacklisted_id").unwrap().as_str().unwrap_or("1").parse::<u64>() {
                    field_string += format!("{}\n", blacklisted_id).as_str();
                }
            }
            Err(e) => {
                eprintln!("Error while inspecting doc: {}", e);
            }
        }
    }
    embed.field("Blacklisted IDs", field_string, true);

    embed
}