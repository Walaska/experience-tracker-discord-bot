use serenity::{
    framework::standard::{macros::command, Args, CommandResult,},
    model::prelude::*,
    prelude::*,
};

#[command]
#[bucket = "basic"]
async fn xd(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    match args.message() {
        "racn" => racn(&ctx, &msg).await,
        _ => println!("asd")
    }

    Ok(())
}

async fn racn(ctx: &Context, msg: &Message) {
    let channel = match ctx.cache.guild_channel(msg.channel_id) {
        Some(channel) => channel,
        None => {
            let result = msg
                .channel_id
                .say(&ctx, "Could not find channel").await;
            if let Err(e) = result {
                eprintln!("Could not send msg");
            }
            return;
        }
    };
    let last_message = channel.messages(&ctx.http, |retriever| retriever.limit(2) ).await.expect("Can't find messages.");
    if let Some(message) = last_message.get(0) { message.delete(&ctx.http).await.expect("Can't delete message"); }
    if let Some(message) = last_message.get(1) {
        message.react(&ctx.http, ReactionType::try_from("<:rcn:1008695941697642526>").unwrap()).await.expect("Can't react to message.");
        message.react(&ctx.http, '🤝').await.expect("Can't react to message.");
        message.react(&ctx.http, ReactionType::try_from("<:walas3:1141479523787997254>").unwrap()).await.expect("Can't react to message.");

    } else {
        eprintln!("No messages in this channel REEEE");
    }

}