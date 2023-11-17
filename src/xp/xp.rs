use serenity::{async_trait, futures::StreamExt};
use std::time::{Duration, Instant};
use serenity::{client::{Context, EventHandler}, model::channel::Message};
use serenity::{
    model::prelude::*,
    prelude::*,
};
use mongodb::{bson::{doc, Document, DateTime}, Collection};
use std::collections::HashMap;
use chrono::{Utc, NaiveDateTime};
use tokio::time::sleep;
use lazy_static::lazy_static;

use crate::MongoDb;

// COOLDOWNS
const TEXT_CD_AS_SEC:  u64 = 8;
const VOICE_CD_AS_SEC: u64 = 240;
// XP VARIABLES
const TEXT_XP:  f64 = 3.0;
const IMAGE_XP: f64 = 6.0;
const VOICE_XP: f64 = 10.0;
// LEVEL VARIALBES
const BASE_XP:          i32 = 130;
const MAX_LEVEL:        usize = 51;
const FIRST_PERCENTAGE: f64 = 0.7;
const LAST_PERCENTAGE:  f64 = 0.3;
// ART CHANNEL IDS THAT DO NOT GIVE MULTIPLIED XP
const ART_CHANNEL_IDS: [u64; 6] = [903596413231964191, 903596378293411891, 903596456466853969, 903596435663097907, 928645070134075462, 1143976152348770434];
// LEVEL ROLE IDs
const ROLE_IDS: [u64; 11] = [1137825594084708575,1136686234085888120,1136686481809875014,
                            1136686410317971556,1136686642283941898,1136686557156352112,
                            1136686515800518757,1136686706930745365,1136686735988883487,
                            1136686810785919006,1136686841236553908];
struct VoiceUser {
    guild: GuildId,
    user_id: UserId,
    cd_end: Instant,
    channel_id: ChannelId,
}

impl VoiceUser {
    fn new(guild: GuildId, user_id: UserId, channel_id: ChannelId) -> Self {
        VoiceUser {
            guild: guild,
            user_id: user_id,
            cd_end: Instant::now() + Duration::from_secs(VOICE_CD_AS_SEC),
            channel_id: channel_id,
        }
    }
}

lazy_static! {
    // COOLDOWNS
    static ref TEXT_CD: Mutex<HashMap<UserId, GuildId>> = Mutex::new(HashMap::new());
    static ref VOICE_CD: Mutex<HashMap<UserId, VoiceUser>> = Mutex::new(HashMap::new());

    // LOCAL CACHE
    static ref BLACKLIST: Mutex<HashMap<u64, u64>> = Mutex::new(HashMap::new());
    static ref MULTIPLIER: Mutex<HashMap<u64, f64>> = Mutex::new(HashMap::new());
}
pub struct XpHandler;

#[async_trait]
impl EventHandler for XpHandler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot { return; }
        if check_text_cooldown(&msg.author.id, &ctx).await {
            //println!("ON COOLDOWN FOR {}", &msg.author.name);
            return;
        }
        else if check_blacklist(&msg.channel_id, msg.clone().member.unwrap().roles).await {return;}
        let role_multiplier = calculate_role_multiplier(msg.clone().member.unwrap().roles).await;
        if let Some(guild_id) = msg.guild_id {
            if let Ok(member) = guild_id.member(&ctx.http, msg.author.id).await {
                calculate_text_xp(&msg, &ctx, &msg.channel_id, role_multiplier, &member).await;
                text_cooldown(msg.author.id, msg.guild_id.unwrap(), &ctx).await;
            }
        }
        
    }
}

pub fn calculate_xp(level: u32) -> u32 {
    let levels = MAX_LEVEL;
    let base_xp = BASE_XP;
    let a = FIRST_PERCENTAGE;
    let b = LAST_PERCENTAGE;

    let last_20_percent = (levels as f64 * b).ceil() as usize;
    let first_80_percent = levels - last_20_percent;

    let mut total_xp = 0;
    let xp_scale = (base_xp as f64 / a).ceil() as u32;
    for i in 1..=first_80_percent {
        let xp = (xp_scale as f64 * f64::ln(i as f64 + 1.0)).ceil() as u32;
        total_xp += xp;
        if i == level as usize {
            return total_xp;
        }
    }

    let remaining_xp = total_xp as f64;
    for i in (first_80_percent + 1)..=levels {
        let xp = ((remaining_xp / last_20_percent as f64) * (i - first_80_percent) as f64).ceil() as u32;
        total_xp += xp;
        if i == level as usize {
            return total_xp;
        }
    }

    0
}

async fn calculate_multiplier(channel_id: &ChannelId, role_multi: f64) -> f64 {
    let channel_multi = {
        *MULTIPLIER.lock().await.get(channel_id.as_u64()).unwrap_or(&1.0)
    };
    (channel_multi + role_multi) - 1.0
}

async fn check_blacklist(channel_id: &ChannelId, roles: Vec<RoleId>) -> bool {
    for role in roles {
        if BLACKLIST.lock().await.contains_key(role.as_u64()) {
            return true;
        }
    }
    if BLACKLIST.lock().await.contains_key(channel_id.as_u64()) {
        return true;
    } else {
        return false;
    }
}

async fn calculate_role_multiplier(roles: Vec<RoleId>) -> f64 {
    let mut multiplier: f64 = 0.0;
    for role in roles {
        let role_multi = {
            *MULTIPLIER.lock().await.get(role.as_u64()).unwrap_or(&1.0) - 1.0
        };
        multiplier += role_multi;
    }
    return multiplier + 1.0;
}

async fn calculate_text_xp(msg: &Message, ctx: &Context, channel_id: &ChannelId, role_multi: f64, member: &Member) {
    let multiplier = calculate_multiplier(channel_id, role_multi).await;
    if msg.attachments.len() > 0 {
        add_xp(ctx, channel_id, member, msg.guild_id.unwrap(), IMAGE_XP * multiplier).await;
    } else {
        // Bandaid fix to not give multiplied XP in certain channels for text messages
        if ART_CHANNEL_IDS.contains(channel_id.as_u64()) {
            add_xp(ctx, channel_id, member, msg.guild_id.unwrap(), TEXT_XP * 1.0).await;            
        } else {
            add_xp(ctx, channel_id, member, msg.guild_id.unwrap(), TEXT_XP * multiplier).await;
        }
    }
}

async fn calculate_voice_xp(ctx: &Context, guild_id: GuildId, channel_id: &ChannelId, role_multi: f64, member: &Member) {
    let multiplier = calculate_multiplier(channel_id, role_multi).await;
    add_xp(ctx, channel_id, member, guild_id, VOICE_XP * multiplier).await;
}

async fn add_xp(ctx: &Context, channel: &ChannelId, member: &Member, guild_id: GuildId, xp: f64) {
    let client = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
    };
    let db = client.database("xp");
    let collection = db.collection::<Document>("users");
    let filter = doc! {
        "user_id": format!("{}", member.user.id.as_u64()),
        "server_id": format!("{}", guild_id.as_u64())
    };
    let options = mongodb::options::UpdateOptions::builder().upsert(true).build();
    //println!("{} gained {} XP in:{}", member.user.name, xp as u32, channel.name(ctx).await.unwrap_or("Unknown Channel".to_string()));
    if let Err(e) = collection.update_one(filter, doc! { "$inc": { "xp": xp as u32 } }, options).await {eprintln!("{:?}", e)}
    check_level_up(ctx, channel, &member, &guild_id, &collection).await;
}

pub async fn check_level_up(ctx: &Context, channel_id: &ChannelId, member: &Member, guild_id: &GuildId, collection: &Collection<Document>) {
    let filter = doc! {
        "user_id": format!("{}", member.user.id.as_u64()),
        "server_id": format!("{}", guild_id.as_u64())
    };
    let document = collection.find_one(filter, None).await;
    if let Ok(doc) = document {
        if let Some(doc) = doc {
            if let Ok(xp) = doc.get_i32("xp") {
                let mut level = 0; 
                let new_level = xp_to_level(xp.try_into().unwrap_or(0));
                if let Ok(level_key) = doc.get_i32("level") { level = level_key; }
                if new_level != level.try_into().unwrap_or(0) {
                    level_up(ctx, channel_id, guild_id, collection, new_level, &member).await;
                }
            }
        }
    }
}

async fn level_up_msg(ctx: &Context, channel: &ChannelId, level: u32, member: &Member) {
    channel.send_message(ctx, |m| {
        m.add_embed(|e| e
        .color(0xF34D69)
        .description(format!("{} just leveled up! New level: **{}** <a:breakdancing_cat:1013526376323764365>", member.user.name, level))
        )
    }).await.expect("Error when sending level up message.");
}

async fn level_up(ctx: &Context, channel_id: &ChannelId, guild_id: &GuildId, collection: &Collection<Document>, level: u32, member: &Member) {
    let filter = doc! {
        "user_id": format!("{}", member.user.id.as_u64()),
        "server_id": format!("{}", guild_id.as_u64())
    };
    let options = mongodb::options::UpdateOptions::builder().upsert(true).build();

    if let Some(current_roles) = member.roles(ctx) {
        let mut roles_to_remove: Vec<u64> = vec![];
        for x in current_roles {
            if ROLE_IDS.contains(&x.id.as_u64()) {
                roles_to_remove.push(x.id.as_u64().clone());
            }
        }

        //println!("roles to remove {:?}", roles_to_remove);
        if let Err(e) = ctx.http.add_member_role(guild_id.as_u64().clone(), member.user.id.as_u64().clone(), ROLE_IDS.get(((level / 5)) as usize).unwrap_or(&ROLE_IDS[ROLE_IDS.len() - 1]).clone(), Some("Level up role")).await {
            println!("{:?}", e);
        }

        for role_id in roles_to_remove {
            if ROLE_IDS.get(((level / 5) as usize)) != Some(&role_id) { 
                if let Err(e) = ctx.http.remove_member_role(guild_id.as_u64().clone(), member.user.id.as_u64().clone(), role_id, Some("Level up role removed.")).await {
                    eprintln!("Error removing role: {:?}", e);
                }
            }
        }
    }

    if let Err(e) = collection.update_one(filter, doc! { "$set": { "level": level } }, options).await {eprintln!("{:?}", e)}
    level_up_msg(ctx, channel_id, level, &member).await;
}

pub fn xp_to_level(xp: u32) -> u32 {
    let mut level = 1;
    while xp >= calculate_xp(level) {
        level += 1;
    }
    level - 1
}

async fn text_cooldown(user_id: UserId, guild_id: GuildId, ctx: &Context) {
    /* 
    let client = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
    };
    let db = client.database("xp");
    let collection = db.collection::<Document>("users");
    let options = mongodb::options::UpdateOptions::builder().upsert(true).build();
    let filter = doc! {
        "user_id": format!("{}", user_id.as_u64()),
        "server_id": format!("{}", guild_id.as_u64())
    };
    let update_filter = doc! {"$set": {"last_txt_msg": Utc::now().timestamp()}};
    if let Err(e) = collection.update_one(filter, update_filter, options).await {eprintln!("{:?}", e)}
    */
    {
        TEXT_CD.lock().await.insert(user_id, guild_id);
    }
    //println!("Text cooldown STARTED for {}", user_id);
    sleep(Duration::from_secs(TEXT_CD_AS_SEC)).await;
    //println!("Text cooldown ENDED for {}", user_id);
    TEXT_CD.lock().await.remove(&user_id);
}

async fn check_text_cooldown(user_id: &UserId, ctx: &Context) -> bool {
    /* 
    let client = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
    };
    let db = client.database("xp");
    let collection = db.collection::<Document>("users");
    let filter = doc! {
        "user_id": format!("{}", user_id.as_u64()),
    };

    if let Some(document) = collection.find_one(filter, None).await.expect("Cooldown document not found") {
        if let Ok(cooldown) = document.get_i64("last_txt_msg") {
            if (cooldown + TEXT_CD_AS_SEC as i64) < Utc::now().timestamp() {
                println!("Text cooldown TRUE - cooldown: {} - UTC NOW: {}", (cooldown + TEXT_CD_AS_SEC as i64), Utc::now().timestamp());
                return true;
            }
        }
    }
    
    false
*/
    TEXT_CD.lock().await.contains_key(user_id)
}

async fn check_voice_cooldown(user_id: &UserId) -> bool {
    VOICE_CD.lock().await.contains_key(user_id)
}

pub async fn initialize_blacklist(ctx: &Context) {
    let client = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
    };
    let db = client.database("xp");
    {
        BLACKLIST.lock().await.clear();
    }
    let collection = db.collection::<Document>("blacklist");
    let mut document = collection.find(doc! { }, None).await.expect("Error trying to find blacklist document");
    while let Some(result) = document.next().await {
        match result {
            Ok(doc) => {
                let blacklisted_id = doc.get("blacklisted_id").unwrap().as_str().unwrap_or("1").parse::<u64>().unwrap_or(1);
                let guild = doc.get("guild").unwrap().as_str().unwrap_or("1").parse::<u64>().unwrap_or(1);
                {
                    BLACKLIST.lock().await.insert(blacklisted_id, guild);
                }
            },
            Err(e) => {
                eprintln!("Error while iterating over cursor: {}", e);
                break;
            }
        }
    }
}

pub async fn initialize_multiplier(ctx: &Context) {
    let client = {
        let data_read = ctx.data.read().await;
        data_read.get::<MongoDb>().expect("Expected MongoDb").clone()
    };
    {
        MULTIPLIER.lock().await.clear();
    }
    let db = client.database("xp");
    let collection = db.collection::<Document>("role_multipliers");
    let mut document = collection.find(doc! { }, None).await.expect("Error trying to find blacklist document");
    // Role multiplier
    while let Some(result) = document.next().await {
        match result {
            Ok(doc) => {
                let role_id = doc.get("role_id").unwrap().as_str().unwrap_or("1").parse::<u64>().unwrap_or(1);
                let amount = doc.get("amount").unwrap().as_str().unwrap_or("1.0").parse::<f64>().unwrap_or(1.0);
                {
                    MULTIPLIER.lock().await.insert(role_id, amount);
                }
            },
            Err(e) => {
                eprintln!("Error while iterating over cursor: {}", e);
                break;
            }
        }
    }
    let collection = db.collection::<Document>("channel_multipliers");
    let mut document = collection.find(doc! { }, None).await.expect("Error trying to find blacklist document");
    // Channel multiplier
    while let Some(result) = document.next().await {
        match result {
            Ok(doc) => {
                let channel_id = doc.get("channel_id").unwrap().as_str().unwrap_or("1").parse::<u64>().unwrap_or(1);
                let amount = doc.get("amount").unwrap().as_str().unwrap_or("1.0").parse::<f64>().unwrap_or(1.0);
                {
                    MULTIPLIER.lock().await.insert(channel_id, amount);
                }
            },
            Err(e) => {
                eprintln!("Error while iterating over cursor: {}", e);
                break;
            }
        }
    }
}

// VOICE COOLDOWN FUNCTIONS

pub async fn remove_voice_hash(old: &VoiceState) {
    if let Some(member) = &old.member {
        VOICE_CD.lock().await.remove(&member.user.id);
    }
}

pub async fn voice_cooldown(ctx: &Context, new: &VoiceState) {
    let member = ctx.cache.member(new.guild_id.unwrap_or_default(), new.user_id).expect("Member not found in voice chat!!!");
    if check_blacklist(&new.channel_id.unwrap_or_default(), member.roles).await { return; }
    if !check_voice_cooldown(&member.user.id).await && !member.user.bot {
        {
            VOICE_CD.lock().await.insert(member.user.id, VoiceUser::new(new.guild_id.unwrap_or_default(), member.user.id, new.channel_id.unwrap_or_default()));
        }
    }
}

pub async fn update_voice_channel_id(new: &VoiceState) {
    let mut guard = VOICE_CD.lock().await;
    if let Some(user) = guard.get_mut(&new.user_id) {
        user.channel_id = new.channel_id.unwrap_or_default();
    }
}

pub async fn voice_hash_loop(ctx: &Context) {
    loop {
        {
            let mut guard = VOICE_CD.lock().await;
            if guard.len() > 1 {
                for (user, voiceuser) in guard.iter_mut() {
                    if voiceuser.cd_end <= Instant::now() {
                        let member = ctx.cache.member(voiceuser.guild, user);
                        if let Some(member) = member {
                            if !member.deaf {
                                let role_multiplier = calculate_role_multiplier(member.clone().roles).await;
                                calculate_voice_xp(ctx, voiceuser.guild, &voiceuser.channel_id, role_multiplier, &member).await;
                                voiceuser.cd_end = Instant::now() + Duration::from_secs(VOICE_CD_AS_SEC);
                            }
                        } else {
                            eprintln!("Can't find member: {} in guild: {}", user, voiceuser.guild);
                        }
                    }
                }
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
}