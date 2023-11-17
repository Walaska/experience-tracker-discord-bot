use mongodb::{Database, Collection};
use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::{GuildId, PartialChannel, Role};
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{CommandDataOption, CommandDataOptionValue};
use serenity::model::permissions::Permissions;
use serenity::client::Context;
use mongodb::bson::{doc, Document};
use tracing::log::info;
use mongodb::options::UpdateOptions;

use crate::{MongoDb, xp};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("xp_multiplier")
        .description("Set multiplier for role or channel. Admin only.")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .create_option(|option| {
            option
                .name("amount")
                .description("Insert multiplier amount")
                .kind(CommandOptionType::Number)
                .min_number_value(0.0)
                .max_number_value(100.0)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("role")
                .description("Choose the role")
                .kind(CommandOptionType::Role)
                .required(false)
        })
        .create_option(|option| {
            option
                .name("channel")
                .description("Choose the channel")
                .kind(CommandOptionType::Channel)
                .required(false)
        })
}

pub async fn run(options: &[CommandDataOption], ctx: &Context, guild_id: &GuildId) -> String {
    if options.len() < 2 {
        return "Expected at least two options...".to_string()
    }
    let client = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
    };
    let db = client.database("xp");
    let mut amount = None;
    let mut role_option = None;
    let mut channel_option = None;
    
    for option in options.iter() {
        match &option.resolved {
            Some(CommandDataOptionValue::Number(number)) => amount = Some(*number),
            Some(CommandDataOptionValue::Role(role)) => role_option = Some(role.clone()),
            Some(CommandDataOptionValue::Channel(channel)) => channel_option = Some(channel.clone()),
            _ => {}
        }
    }

    if amount.unwrap() == 1.0 {
        match (channel_option, role_option) {
            (Some(channel), None) => {
                remove_multiplier(Some(channel), None, db).await;
            },
            (None, Some(role)) => {
                remove_multiplier(None, Some(role), db).await;
            },
            (Some(channel), Some(role)) => {
                remove_multiplier(Some(channel), Some(role), db).await;
            },
            (None, None) => {}
        }
    } else {
        match (channel_option, role_option) {
            (Some(channel), None) => {
                update_multiplier(Some(channel), None, db, amount.unwrap(), guild_id).await;
            },
            (None, Some(role)) => {
                update_multiplier(None, Some(role), db, amount.unwrap(), guild_id).await;
            },
            (Some(channel), Some(role)) => {
                update_multiplier(Some(channel), Some(role), db, amount.unwrap(), guild_id).await;
            },
            (None, None) => {}
        }
    }
    xp::xp::initialize_multiplier(ctx).await;
    "Multiplier(s) set!".to_string()
}

async fn update_multiplier(channel: Option<PartialChannel>, role: Option<Role>, db: Database, amount: f64, guild_id: &GuildId) {
    let update = doc! {
        "$set": {
            "amount": format!("{}", amount),
            "guild": format!("{}", guild_id.as_u64())
        }
    };
    let options = UpdateOptions::builder()
    .upsert(true)
    .build();
    match (channel, role) {
        (Some(channel), None) => {
            let collection = db.collection::<Document>("channel_multipliers");
            collection.update_one(doc! {"channel_id": format!("{}", channel.id.as_u64())}, update, options).await.expect("Error while updating multiplier(s)");
        },
        (None, Some(role)) => {
            let collection = db.collection::<Document>("role_multipliers");
            collection.update_one(doc! {"role_id": format!("{}", role.id.as_u64())}, update, options).await.expect("Error while updating multiplier(s)");
        },
        (Some(channel), Some(role)) => {
            let collection = db.collection::<Document>("channel_multipliers");
            collection.update_one(doc! {"channel_id": format!("{}", channel.id.as_u64())}, update.clone(), options.clone()).await.expect("Error while updating multiplier(s)");
            let collection = db.collection::<Document>("role_multipliers");
            collection.update_one(doc! {"role_id": format!("{}", role.id.as_u64())}, update, options).await.expect("Error while updating multiplier(s)");
        },
        (None, None) => {}
    };
}

async fn remove_multiplier(channel: Option<PartialChannel>, role: Option<Role>, db: Database) {
    match (channel, role) {
        (Some(channel), None) => {
            let collection = db.collection::<Document>("channel_multipliers");
            collection.delete_one(doc! {"channel_id": format!("{}", channel.id.as_u64())}, None).await.expect("Error removing multipliers...");
        },
        (None, Some(role)) => {
            let collection = db.collection::<Document>("role_multipliers");
            collection.delete_one(doc! { "role_id": format!("{}", role.id.as_u64()) }, None).await.expect("Error removing multipliers...");
        },
        (Some(channel), Some(role)) => {
            let collection = db.collection::<Document>("channel_multipliers");
            collection.delete_one(doc! { "channel_id": format!("{}", channel.id.as_u64()), }, None).await.expect("Error removing multipliers...");
            let collection = db.collection::<Document>("role_multipliers");
            collection.delete_one(doc! { "role_id": format!("{}", role.id.as_u64()), }, None).await.expect("Error removing multipliers...");
        },
        (None, None) => {}
    };
}