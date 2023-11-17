use mongodb::error::CommandError;
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
async fn racn(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let api_url = "https://some-random-api.com/animal/raccoon";
    let response = reqwest::get(api_url).await?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await?;

        if let (Some(image_url), Some(fact)) = (json["image"].as_str(), json["fact"].as_str()) {
            let _ = msg
                .channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.title("Random Racn Fact");
                        e.image(image_url);
                        e.description(fact);

                        e
                    })
                })
                .await?;
        }
    } else {
        println!("Error fetching racn from API.. :(");
    }

    Ok(())
}