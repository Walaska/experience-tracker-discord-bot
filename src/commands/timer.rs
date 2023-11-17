use mongodb::Database;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use serenity::builder::CreateButton;
use tokio::time::sleep;
use mongodb::bson::{doc, Document};
use serenity::futures::StreamExt;
use serenity::model::application::component::ButtonStyle;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::prelude::*,
    prelude::*,
};
use humantime::{parse_duration, format_duration};
use lazy_static::lazy_static;
use async_recursion::async_recursion;

use crate::MongoDb;

lazy_static! {
    static ref QUEUE: Mutex<Vec<UserId>> = Mutex::new(vec![]);
}

#[command]
async fn timer(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let ALLOWED_CHANNEL_IDS = vec![932759575973748806];
    if !ALLOWED_CHANNEL_IDS.contains(msg.channel_id.as_u64()) {return Ok(()); }
    if let Ok(time) = parse_duration(args.message()) {
        if time.as_secs() < 15 || time.as_secs() > 14_400 {msg.reply(&ctx, "Timer can't be **<15s** or **>4 hrs**! <:shrug:998328408738115664>").await?;}
        else if check_timer(&msg.author.id).await {msg.reply(&ctx, "You have a timer running already. You could **.timer stop** it.").await?;}
        else {
            let client = {
                let data_read = ctx.data.read().await;
                data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
            };
            let db = client.database("timer");
            if let Err(e) = start_timer(time, &msg, &ctx, &db).await {eprintln!("{:?}", e);}
        }
    } else if args.is_empty() {
        if check_timer(&msg.author.id).await {msg.reply(&ctx, "You have a timer running already. You could **.timer stop** it.").await?;}
        else {
            let client = {
                let data_read = ctx.data.read().await;
                data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
            };
            let db = client.database("timer");
            let collection = db.collection::<Document>("timers");
            let userid = msg.author.id.as_u64();
            let document = collection.find_one(doc! { "_id": format!("{}", userid)}, None).await?;
            if let Some(doc) = document {
                let duration = doc.get_f64("duration").unwrap_or(0.0);
                if let Err(e) = start_timer(Duration::from_secs(duration as u64), &msg, &ctx, &db).await {eprintln!("{:?}", e);}
            } else {
                msg.reply(&ctx, "First time? You can repeat timers once you've used your first.\nFor now, use **.timer (time)**!").await?;
            }
        }
    } else {
        if let Some(argument) = args.current() {
            match argument {
                "stop" | "end" | "quit" => {
                    if check_timer(&msg.author.id).await {
                        QUEUE.lock().await.retain(|userid| userid.as_u64() != msg.author.id.as_u64());
                    } else {
                        msg.channel_id.say(&ctx, "You have no timers running!").await?;
                    }
                },
                "help" => {
                    if let Err(e) = help_command(&msg, &ctx).await {eprintln!("{:?}", e);}
                },
                _ => {
                    msg.channel_id.say(&ctx, "Invalid argument! Use .timer help").await?;
                }
            }
        }
    }

    Ok(())
}

#[async_recursion]
async fn start_timer(duration: Duration, msg: &Message, ctx: &Context, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
    let mut messages: Vec<Message> = vec![];
    let end_time = Instant::now() + duration;
    let start_time = Instant::now();
    let mut loop_count: usize = 1;
    let mut special_user = "\u{200B}";

    let timer_emojis = vec!["<:5988lbg:1087721445158826034><:2827l2g:1087721442415743091><:2827l2g:1087721442415743091><:2881lb3g:1087721440884838512>",
                                        "<:5988lbg:1087721445158826034><:3451lg:1087721443887960104><:2827l2g:1087721442415743091><:2881lb3g:1087721440884838512>",
                                        "<:5988lbg:1087721445158826034><:3451lg:1087721443887960104><:3451lg:1087721443887960104><:2881lb3g:1087721440884838512>"];
    let y = 661642542219001874;
    if msg.author.id.as_u64() == &y {special_user = " raccoon <:rcn:1008695941697642526>"; }
    let mut start_timer_msg = msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| e
        .color(0xFEE75C)
        .title(format!("Timer started, stops <t:{}:R>", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + duration.as_secs()))
        .field("\u{200B}", format!("Good luck{}!", special_user), false)
        .footer(|f| {
            f.icon_url(msg.author.avatar_url().unwrap_or_default());
            f.text(format!("{}'s timer", msg.author.name))
        })
        );
        m.reference_message(msg);
        m.components(|c| {
            c.create_action_row(|r| {
                r.add_button(stop_button(msg.author.id, "<:stop:1083821536298934322>".parse().unwrap(), false))                     
            })
        })
    }).await?;

    let c_clone = ctx.clone();
    let start_timer_msg_clone = start_timer_msg.clone();
    let user = msg.author.clone();
    tokio::spawn(async move {
        let mut interaction_stream = 
        start_timer_msg_clone.await_component_interactions(&c_clone).timeout(duration).build();

        while let Some(interaction) = interaction_stream.next().await {
            if interaction.user.id == user.id {
                interaction.create_interaction_response(&c_clone, |r| {
                    r.kind(interaction::InteractionResponseType::UpdateMessage).interaction_response_data(|d| {
                        d.content("\u{200B}")
                    })
                }).await.unwrap();
                QUEUE.lock().await.retain(|userid| userid.as_u64() != user.id.as_u64());
            }
        } 
    });

    {
        let mut queue = QUEUE.lock().await;
        queue.push(msg.author.id);
    }

    let collection = db.collection::<Document>("timers");
    let x: u64 = msg.author.id.into();
    let options = mongodb::options::UpdateOptions::builder().upsert(true).build();
    if let Err(e) = collection.update_one(doc! { "_id": format!("{}", x)}, doc! { "$set": { "duration": duration.as_secs_f64() } }, options).await {eprintln!("{:?}", e)}

    while Instant::now() < end_time {
        sleep(Duration::from_millis(500)).await;
        if !check_timer(&msg.author.id).await {
            stop_timer(start_timer_msg, &msg.author, &ctx).await;
            return Ok(());
        }
        let percent = ((Instant::now() - start_time).as_secs_f64() * 100.0 / duration.as_secs_f64()) as u32;
        if percent >= loop_count as u32 * 25 && duration.as_secs() >= 60 && percent != 100 {
            if loop_count > 1 {
                messages[loop_count - 2].delete(&ctx).await?;
            }
            messages.push(msg.channel_id.send_message(&ctx, |m| {
                m.content(format!("{}", Mention::from(msg.author.id)));
                m.add_embed(|e| e
                .color(0xFEE75C)
                .title(format!("You are {}% of the way.", loop_count * 25))
                .field("\u{200B}", timer_emojis[loop_count - 1], false)
                .footer(|f| {
                    f.icon_url(msg.author.avatar_url().unwrap_or_default());
                    f.text(format!("{}'s timer", msg.author.name))
                })
                )
            }).await?);
            loop_count += 1;
        }
    }
    if duration.as_secs() > 30 {
        messages[messages.len() - 1].delete(&ctx).await?;
    }
    messages.push(msg.channel_id.send_message(&ctx, |m| {
        m.content(format!("{}", Mention::from(msg.author.id)));
        m.add_embed(|e| e
        .color(0x57F287)
        .title(format!("{} timer finished! Nice.", format_duration(duration).to_string()))
        .field("\u{200B}", "<:5988lbg:1087721445158826034><:3451lg:1087721443887960104><:3451lg:1087721443887960104><:3166lb4g:1087721439395860520>", false)
        .footer(|f| {
            f.icon_url(msg.author.avatar_url().unwrap_or_default());
            f.text(format!("{}'s timer", msg.author.name))
        })
        )
    }).await?);
    remove_from_queue(&msg.author, &ctx, start_timer_msg, duration, &msg, db).await;
    {
        QUEUE.lock().await.retain(|user| user.as_u64() != msg.author.id.as_u64());
    }
    Ok(())
}

#[async_recursion]
async fn remove_from_queue(user: &User, ctx: &Context, mut message: Message, duration: Duration, msg: &Message, db: &Database) {
    QUEUE.lock().await.retain(|userid| userid.as_u64() != user.id.as_u64());

    message.edit(ctx, |m| {
        m.embed(|e| e
        .color(0x57F287)
        .title("Timer finished!")
        .field("\u{200B}", format!("Good work {} <:hehe:1013526129224732893>, go again?", Mention::from(user.id)), false)
        .footer(|f| {
            f.icon_url(user.avatar_url().unwrap_or_default());
            f.text(format!("{}'s timer", user.name))
        })
        );
        m.components(|c| {
            c.create_action_row(|r| {
                r.add_button(repeat_button(user.id, "<:re:1081576876860002456>".parse().unwrap(), false))                     
            })
        })
    }).await.expect("...");

    let mut interaction_stream = 
    message.await_component_interactions(&ctx).timeout(duration).build();

    while let Some(interaction) = interaction_stream.next().await {
        if interaction.user.id == user.id {
            interaction.create_interaction_response(&ctx, |r| {
                r.kind(interaction::InteractionResponseType::UpdateMessage).interaction_response_data(|d| {
                    d.components(|c| {
                        c.create_action_row(|r| {
                            r.add_button(repeat_button(user.id, "<:re:1081576876860002456>".parse().unwrap(), true))
                        })
                    })
                })
            }).await.unwrap();
            if let Err(e) = start_timer(duration, msg, ctx, db).await {eprintln!("{:?}", e);}
        }
    } 
}

async fn stop_timer(mut x: Message, user: &User, ctx: &Context) {
    x.edit(ctx, |m| {
        m.embed(|e| e
        .color(0xED4245)
        .title("Timer stopped!")
        .field("\u{200B}", format!("Timer stopped {} <a:waah:1046792526008426546>", Mention::from(user.id)), false)
        .footer(|f| {
            f.icon_url(user.avatar_url().unwrap_or_default());
            f.text(format!("{}'s timer", user.name))
        })
        );
        m.components(|c| {
            c.create_action_row(|r| {
                r.add_button(stop_button(user.id, "<:stop:1083821536298934322>".parse().unwrap(), true))
            })
        })
    }).await.expect("...");
}

async fn check_timer(user: &UserId) -> bool {
    let queue = QUEUE.lock().await;
    queue.iter().any(|userid| userid == user)
}

async fn help_command(msg: &Message, ctx: &Context) -> Result<(), Box<dyn std::error::Error>> {
    msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| e
        .color(0x2c2d30)
        .title("Commands & Info")
		.description ("__Set timers for timed art challenges!__")
		.field("\u{200B}", ("<:y_right2:1046476267672842322>Use .timer (time) to start a timer\n<:y_right2:1046476267672842322>**.timer stop** or the button to stop\n<:y_right2:1046476267672842322>**.timer** to repeat previous timer\n\n<:ping:1087823433540313193> You will get pinged as your timer progresses.\n\n**15 sec is the minimum, 4hr the max.**"), false)
        .footer(|f| {
            f.icon_url(msg.author.avatar_url().unwrap_or_default());
            f.text(format!("Requested by {}", msg.author.name))
        })
        )
    }).await?;
    Ok(()) 
}

fn repeat_button(custom_id: UserId, emoji: ReactionType, disabled: bool) -> CreateButton {
    let mut b = CreateButton::default();
    b.custom_id(custom_id);
    b.label("Repeat");
    b.style(ButtonStyle::Primary);
    b.emoji(emoji);
    b.disabled(disabled);
    b
}

fn stop_button(custom_id: UserId, emoji: ReactionType, disabled: bool) -> CreateButton {
    let mut b = CreateButton::default();
    b.custom_id(custom_id);
    b.label("Stop");
    b.style(ButtonStyle::Danger);
    b.emoji(emoji);
    b.disabled(disabled);
    b
}