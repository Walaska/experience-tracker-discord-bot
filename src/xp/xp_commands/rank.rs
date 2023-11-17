use serenity::builder::{CreateApplicationCommand, CreateEmbed};
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{CommandDataOption, CommandDataOptionValue};
use serenity::model::permissions::Permissions;
use serenity::client::Context;
use serenity::model::prelude::Member;
use mongodb::options::UpdateOptions;
use mongodb::bson::{doc, Document};

use crate::{MongoDb, xp};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("rank")
        .description("Check yours or someone elses rank :)))")
        .create_option(|option| {
            option
                .name("user")
                .description("Which user would you like to inspect")
                .kind(CommandOptionType::User)
                .required(false)
        })
}

pub async fn run(options: &[CommandDataOption], ctx: &Context, member: &Member) -> CreateEmbed {
    let mut embed = CreateEmbed::default();
    let mut xp = 0;
    let mut level = 0;

    let client = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
    };
    let db = client.database("xp");
    let collection = db.collection::<Document>("users");

    if options.len() < 1 {
        let document = collection.find_one(doc! {"user_id": format!("{}", member.user.id)}, None).await;
        let mut xp = 0;
        let mut level = 0;
        if let Ok(doc) = document {
            if let Some(doc) = doc {
                if let Ok(xp_value) = doc.get_i32("xp") {xp = xp_value}
                if let Ok(level_value) = doc.get_i32("level") {level = level_value}
            }
        }
        embed.title(format!("{}'s Rank", member.user.name))
        .field("\u{200B}", format!("Level: {}\nXP: {}", level, xp), false)
            .color(0x323337);
    } else {
        if let Some(CommandDataOptionValue::User(user_option, _)) = &options[0].resolved {
            let document = collection.find_one(doc! {"user_id": format!("{}", user_option.id)}, None).await;
            if let Ok(doc) = document {
                if let Some(doc) = doc {
                    if let Ok(xp_value) = doc.get_i32("xp") {xp = xp_value}
                    if let Ok(level_value) = doc.get_i32("level") {level = level_value}
                }
            }
    
            embed.title(format!("Rank of {}", user_option.name))
            .field("\u{200B}", format!("Level: {}\nXP: {}", level, xp), false)
                .color(0x323337);
        }
    }

    return embed;
}   