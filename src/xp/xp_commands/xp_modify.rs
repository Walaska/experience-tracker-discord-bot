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
        .name("xp_modify")
        .description("Give or take xp from user. Admin only.")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .create_option(|option| {
            option
                .name("action")
                .description("description..")
                .kind(CommandOptionType::String)
                .add_string_choice("give", "give")
                .add_string_choice("take", "take")
                .required(true)
        })
        .create_option(|option| {
            option
                .name("user")
                .description("Select user to modify")
                .kind(CommandOptionType::User)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("amount")
                .description("Select user to modify")
                .kind(CommandOptionType::Integer)
                .required(true)
        })
}

pub async fn run(options: &[CommandDataOption], ctx: &Context, guild_id: &GuildId, channel_id: &ChannelId) -> String {
    let mut action = None;
    let mut user = None;
    let mut amount = None;

    for option in options.iter() {
        match &option.resolved {
            Some(CommandDataOptionValue::String(action_option)) => action = Some(action_option),
            Some(CommandDataOptionValue::User(user_option, _)) => user = Some(user_option.clone()),
            Some(CommandDataOptionValue::Integer(amount_option)) => amount = Some(amount_option),
            _ => {}
        }
    }

    let client = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
    };
    let db = client.database("xp");
    let collection = db.collection::<Document>("users");
    let mut xp_amount = match amount {
        Some(a) => a,
        None => &0
    };
    if action.unwrap_or(&String::from("")) == "take" {
        if let Some(user) = user {
            let filter = doc! {
                "user_id": format!("{}", user.id),
                "server_id": format!("{}", guild_id),
            };
            let document = collection.find_one(filter.clone(), None).await;
            let mut changed_xp = 0;
            if let Ok(doc) = document {
                if let Some(doc) = doc {
                    if let Ok(xp) = doc.get_i32("xp") {
                        changed_xp = (xp as i64) - xp_amount;
                    }
                }
            }
            if changed_xp < 0 {
                changed_xp = 0;
            }
            let update = doc! {
                "$set": { 
                    "xp": changed_xp as u32
                }
            };
            let options = UpdateOptions::builder()
            .upsert(true)
            .build();
        
            collection.update_one(filter.clone(), update, options.clone()).await.expect("Error while updating user xp.");
            if let Ok(member) = ctx.http.get_member(guild_id.as_u64().clone(), user.id.as_u64().clone()).await {
                xp::xp::check_level_up(ctx, channel_id, &member, guild_id, &collection).await;
            }
            return format!("{}' XP decreased by {}", user.name, xp_amount).to_string();
        }
    } else {
        if let Some(user) = user {
            let filter = doc! {
                "user_id": format!("{}", user.id),
                "server_id": format!("{}", guild_id),
            };
            let update = doc! {
                "$inc": {
                    "xp": *xp_amount as u32
                }
            };
            let options = UpdateOptions::builder()
            .upsert(true)
            .build();
        
            collection.update_one(filter.clone(), update, options.clone()).await.expect("Error while updating user xp.");
            if let Ok(member) = ctx.http.get_member(guild_id.as_u64().clone(), user.id.as_u64().clone()).await {
                xp::xp::check_level_up(ctx, channel_id, &member, guild_id, &collection).await;
            }
            return format!("{}'s XP increased by {}", user.name, amount.unwrap_or(&0)).to_string();
        }
    }
    "Something went wrong...".to_string()
}   