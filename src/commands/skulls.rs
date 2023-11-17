use serenity::{
    framework::standard::{macros::command, Args, CommandResult,},
    model::prelude::*,
    prelude::*,
};
use rand::seq::SliceRandom;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[command]
#[bucket = "basic"]
async fn skulls(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut skulls: Vec<ReactionType> = vec![
        ReactionType::try_from("<:HOLYSkull:1088497735486947399>").unwrap(), ReactionType::try_from("<a:SpinningSkull:1088497537134104616>").unwrap(),
        ReactionType::try_from("<:Troll_Skull:1088498319522811966>").unwrap(), ReactionType::try_from("<:UpdatedSkull:1088499486831165552>").unwrap(),
        ReactionType::try_from("<:noto_skull_sunglasses:1088497996175523961>").unwrap(), ReactionType::try_from("<a:skull1:1088497411208519710>").unwrap(),
        ReactionType::try_from("<:skullclown:1088498222995095625>").unwrap(), ReactionType::try_from("<a:white_skull:1088497626468593724>").unwrap()
    ];
    let mut rng: StdRng = SeedableRng::seed_from_u64(rand::random());
    skulls.shuffle(&mut rng);
    let channel = match ctx.cache.guild_channel(msg.channel_id) {
        Some(channel) => channel,
        None => {
            let result = msg
                .channel_id
                .say(&ctx, "Could not find channel").await;
            if let Err(e) = result {
                println!("Problem with skulls");
                eprintln!("Could not send msg");
            }
            return Ok(());
        }
    };
    println!("Skulls channel {:?}", channel);
    let last_message = channel.messages(&ctx.http, |retriever| retriever.limit(2) ).await?;
    if let Some(message) = last_message.get(1) {
        for emoji in skulls {
            message.react(&ctx.http, emoji).await?;
        }
    } else {
        eprintln!("No messages in this channel REEEE");
    }
    println!("Skulls");
    Ok(())
}