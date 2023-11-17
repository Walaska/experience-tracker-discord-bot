use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{CommandDataOption, CommandDataOptionValue};
use serenity::model::permissions::Permissions;
use serenity::client::Context;
use serenity::model::prelude::{GuildId, ChannelId};
use mongodb::options::UpdateOptions;
use mongodb::bson::{doc, Document};

use crate::{MongoDb, xp};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("xp_set_level")
        .description("Set level for member. Admin only.")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .create_option(|option| {
            option
                .name("member")
                .description("Choose the member")
                .kind(CommandOptionType::User)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("level")
                .description("Level")
                .kind(CommandOptionType::Integer)
                .required(true)
        })
}

pub async fn run(options: &[CommandDataOption], ctx: &Context, guild_id: &GuildId, channel_id: &ChannelId) -> String {
    let mut user = None;
    let mut level = None;

    for option in options.iter() {
        match &option.resolved {
            Some(CommandDataOptionValue::User(user_option, _)) => user = Some(user_option.clone()),
            Some(CommandDataOptionValue::Integer(level_option)) => level = Some(level_option),
            _ => {}
        }
    }
    let client = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
    };
    let db = client.database("xp");
    let collection = db.collection::<Document>("users");
    let level = match level {
        Some(x) => x,
        None => &0
    };
    if let Some(user) = user {
        let xp = xp::xp::calculate_xp(level.clone() as u32);

        let filter = doc! {
            "user_id": format!("{}", user.id),
            "server_id": format!("{}", guild_id),
        };
        let update = doc! {
            "$set": {
                "xp": xp as u32,
            }
        };
        let options = UpdateOptions::builder()
        .upsert(true)
        .build();
        collection.update_one(filter, update, options).await.expect("Error while updating user xp.");
        if let Ok(member) = ctx.http.get_member(guild_id.as_u64().clone(), user.id.as_u64().clone()).await {
            xp::xp::check_level_up(ctx, channel_id, &member, guild_id, &collection).await;
        }
        return format!("{}'s level set to {}", user.name, level);
    }
    "Something went wrong...".to_string()
}