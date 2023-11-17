use mongodb::Database;
use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::{ GuildId, PartialChannel, Role};
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{CommandDataOption, CommandDataOptionValue};
use serenity::model::permissions::Permissions;
use serenity::client::Context;
use mongodb::bson::{doc, Document};

use crate::{MongoDb, xp};

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("xp_blacklist")
        .description("Remove or add role or channel to blacklist. Admin only.")
        .default_member_permissions(Permissions::ADMINISTRATOR)
        .create_option(|option| {
            option
                .name("action")
                .description("Would you want to remove or add to blacklist")
                .kind(CommandOptionType::String)
                .add_string_choice("Add", "add")
                .add_string_choice("Remove", "remove")
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

pub async fn run(options: &[CommandDataOption], ctx: &Context, guild_id: &GuildId,) -> String {
    if options.len() < 2 {
        return "Expected at least two options...".to_string()
    }
    let client = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
    };
    let db = client.database("xp");
    let mut action = None;
    let mut role_option = None;
    let mut channel_option = None;
    let mut response = String::from("Something went wrong...");

    for option in options.iter() {
        match &option.resolved {
            Some(CommandDataOptionValue::String(first_option)) => action = Some(first_option),
            Some(CommandDataOptionValue::Role(role)) => role_option = Some(role.clone()),
            Some(CommandDataOptionValue::Channel(channel)) => channel_option = Some(channel.clone()),
            _ => {}
        }
    }

    if action.unwrap() == "add" {
        match (channel_option, role_option) {
            (Some(channel), None) => {
                response = add_blacklist(Some(channel), None, guild_id, db).await;
            },
            (None, Some(role)) => {
                response = add_blacklist(None, Some(role), guild_id, db).await;
            },
            (Some(channel), Some(role)) => {
                response = add_blacklist(Some(channel), Some(role), guild_id, db).await;
            },
            (None, None) => {}
        }
    } else {
        match (channel_option, role_option) {
            (Some(channel), None) => {
                response = remove_blacklist(Some(channel), None, db).await;
            },
            (None, Some(role)) => {
                response = remove_blacklist(None, Some(role), db).await;
            },
            (Some(channel), Some(role)) => {
                response = remove_blacklist(Some(channel), Some(role), db).await;
            },
            (None, None) => {}
        }
    }

    xp::xp::initialize_blacklist(ctx).await;
    response
}

async fn add_blacklist(channel: Option<PartialChannel>, role: Option<Role>, guild: &GuildId, db: Database) -> String {
    let collection = db.collection::<Document>("blacklist");
    let mut response = String::new();
    match (channel, role) {
        (Some(channel), None) => {
            let filter = doc! {"blacklisted_id": format!("{}", channel.id.as_u64()), "guild": format!("{}", guild)};
            if let Ok(result) = collection.find_one(Some(filter), None).await {
                if result.is_some() {
                    response = format!("The channel '{}' is already blacklisted", channel.name.unwrap_or(String::from("channel_name_error")));
                } else {
                    if let Ok(_) = collection.insert_one(doc! {"blacklisted_id": format!("{}", channel.id.as_u64()), "guild": format!("{}", guild)}, None).await {
                        response = format!("The channel '{}' has been added to the blacklist", channel.name.unwrap_or(String::from("channel_name_error")));
                    }
                }
            }
        },
        (None, Some(role)) => {
            let filter = doc! {"blacklisted_id": format!("{}", role.id.as_u64()), "guild": format!("{}", guild)};
            if let Ok(result) = collection.find_one(Some(filter), None).await {
                if result.is_some() {
                    response = format!("The role '{}' is already blacklisted", role.name);
                } else {
                    if let Ok(_) = collection.insert_one(doc! {"blacklisted_id": format!("{}", role.id.as_u64()), "guild": format!("{}", guild)}, None).await {
                        response = format!("The role '{}' has been added to the blacklist", role.name);
                    }
                }
            }
        },
        (Some(channel), Some(role)) => {
            let channel_name = channel.name.unwrap_or(String::from("channel_name_error"));
            let channel_filter = doc! { "blacklisted_id": format!("{}", channel.id.as_u64()), "guild": format!("{}", guild) };
            let role_filter = doc! { "blacklisted_id": format!("{}", role.id.as_u64()), "guild": format!("{}", guild) };
            let mut channel_added = false;
            let mut role_added = false;
            if let Ok(result) = collection.find_one(Some(channel_filter), None).await {
                if result.is_some() {
                    response = format!("The channel '{}' is already blacklisted", &channel_name);
                } else {
                    if let Ok(_) = collection.insert_one(doc! {"blacklisted_id": format!("{}", channel.id.as_u64()), "guild": format!("{}", guild)}, None).await {
                        channel_added = true;
                    }
                }
            }
            if let Ok(result) = collection.find_one(Some(role_filter), None).await {
                if result.is_some() {
                    response = format!("{} The role '{}' is already blacklisted", response, role.name);
                } else {
                    if let Ok(_) = collection.insert_one(doc! {"blacklisted_id": format!("{}", role.id.as_u64()), "guild": format!("{}", guild)}, None).await {
                        role_added = true;
                    }
                }
            }
            if channel_added && role_added {
                response = format!("The channel '{}' and the role '{}' have been added to blacklist!", &channel_name, role.name);
            } else if channel_added {
                response = format!("The channel '{}' was added to blacklist. The role '{}' was already on the blacklist.", &channel_name, role.name);
            } else if role_added {
                response = format!("The role '{}' was added to blacklist. The channel '{}' was already on the blacklist", role.name, &channel_name);
            } else {
                response = format!("The channel '{}' and the role '{}' were both already on the blacklist.", &channel_name, role.name);
            }
        },
        (None, None) => {}
    };
    return response;
}

async fn remove_blacklist(channel: Option<PartialChannel>, role: Option<Role>, db: Database) -> String {
    let collection = db.collection::<Document>("blacklist");
    let mut response = String::new();
    match (channel, role) {
        (Some(channel), None) => {
            let filter = doc! {"blacklisted_id": format!("{}", channel.id.as_u64())};
            if let Ok(result) = collection.delete_one(filter, None).await {
                if result.deleted_count == 1 {
                    response = format!("The channel '{}' has been removed from the blacklist", channel.name.unwrap_or(String::from("channel_name_error")));
                } else {
                    response = format!("The channel '{}' was not found in the blacklist", channel.name.unwrap_or(String::from("channel_name_error")));
                }
            }
        },
        (None, Some(role)) => {
            let filter = doc! {"blacklisted_id": format!("{}", role.id.as_u64())};
            if let Ok(result) = collection.delete_one(filter, None).await {
                if result.deleted_count == 1 {
                    response = format!("The role '{}' has been removed from the blacklist", role.name);
                } else {
                    response = format!("The role '{}' was not found in the blacklist", role.name);
                }
            }
        },
        (Some(channel), Some(role)) => {
            let channel_filter = doc! {"blacklisted_id": format!("{}", channel.id.as_u64())};
            let role_filter = doc! {"blacklisted_id": format!("{}", role.id.as_u64())};
            let mut channel_deleted = false;
            let mut role_deleted = false;
            if let Ok(result) = collection.delete_one(channel_filter, None).await {
                if result.deleted_count == 1 {
                    channel_deleted = true;
                }
            }
            if let Ok(result) = collection.delete_one(role_filter, None).await {
                if result.deleted_count == 1 {
                    role_deleted = true;
                }
            }
            if channel_deleted && role_deleted {
                response = format!("The channel '{}' and the role '{}' have been removed from the blacklist", channel.name.unwrap_or(String::from("channel_name_error")), role.name);
            } else if channel_deleted {
                response = format!("The channel '{}' has been removed from the blacklist", channel.name.unwrap_or(String::from("channel_name_error")));
            } else if role_deleted {
                response = format!("The role '{}' has been removed from the blacklist", role.name);
            }
        },
        (None, None) => {}
    };
    response
}