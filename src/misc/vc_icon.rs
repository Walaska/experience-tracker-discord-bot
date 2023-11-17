use serenity::client::Context;
use serenity::{
    model::prelude::*,
};

const VC_ICON_ROLE_ID: u64 = 1155465494875357265;

pub async fn vc_icon(ctx: &Context, old: &Option<VoiceState>, new: &VoiceState) {
    match old {
        Some(old) => {
            if new.channel_id == None {
                remove_vc_role(ctx, old.guild_id.unwrap_or(GuildId(903586180711481385)), old.user_id).await;
            }
        },
        None => add_vc_role(ctx, new.guild_id.unwrap_or(GuildId(903586180711481385)), new.user_id).await
    }
}

async fn remove_vc_role(ctx: &Context, guild_id: GuildId, user_id: UserId) {
    if let Err(e) = ctx.http.remove_member_role(guild_id.as_u64().clone(), user_id.as_u64().clone(), VC_ICON_ROLE_ID, Some("Level up role removed.")).await {
        eprintln!("Error removing role: {:?}", e);
    }
}

async fn add_vc_role(ctx: &Context, guild_id: GuildId, user_id: UserId) {
    if let Err(e) = ctx.http.add_member_role(guild_id.as_u64().clone(), user_id.as_u64().clone(), VC_ICON_ROLE_ID, Some("Level up role")).await {
        println!("{:?}", e);
    }
}