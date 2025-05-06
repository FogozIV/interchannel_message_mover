use crate::{Context, CustomError};
use anyhow::Result;
use sparkle_convenience::error::IntoError;
use std::time::Duration;
use tokio::time::timeout;
use twilight_model::channel::{Channel, Message};
use twilight_model::http::attachment;
use twilight_util::builder::embed::{EmbedAuthorBuilder, EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder, ImageSource};

impl Context {
    pub async fn execute_webhook_as_member_reference(
        &self,
        message: &Message,
        channel: &Channel,
        attachments: &[attachment::Attachment],
    ) -> Result<()> {
        let mut channel_id = channel.id;
        let mut thread_id = None;
        if channel.kind.is_thread() {
            thread_id = Some(channel_id);
            channel_id = channel.parent_id.ok()?;
        }

        let mut payload = serde_json::json!({
            "username": message
                .member
                .as_ref()
                .and_then(|m| m.nick.as_ref())
                .unwrap_or(&message.author.name),
        });

        if(!message.content.is_empty()){
            payload.as_object_mut().unwrap().insert("content".to_string(), serde_json::json!(message.content));
        }

        // Add optional thread ID
        if let Some(thread_id) = thread_id {
            payload
                .as_object_mut()
                .unwrap()
                .insert("thread_id".to_string(), serde_json::json!(thread_id.to_string()));
        }

        // Add optional avatar URL
        if let Some(avatar_url) = message
            .member
            .as_ref()
            .and_then(|member| member.avatar)
            .zip(message.guild_id)
            .map(|(avatar, guild_id)| {
                format!(
                    "https://cdn.discordapp.com/guilds/{guild_id}/users/{}/avatar/{}.png",
                    message.author.id, avatar
                )
            })
            .or_else(|| {
                message.author.avatar.map(|avatar| {
                    format!(
                        "https://cdn.discordapp.com/avatars/{}/{}.png",
                        message.author.id, avatar
                    )
                })
            })
        {
            payload
                .as_object_mut()
                .unwrap()
                .insert("avatar_url".to_string(), serde_json::json!(avatar_url));
        }


        // Add optional message_reference
        if let Some(ref_msg) = message.reference.as_ref() {
            let ref_json = serde_json::json!({
                "message_id": ref_msg.message_id.unwrap().to_string(),
                "channel_id": ref_msg.channel_id.unwrap().to_string(),
                "guild_id": ref_msg.guild_id.unwrap().to_string()
            });

            payload
                .as_object_mut()
                .unwrap()
                .insert("message_reference".to_string(), ref_json);
        }

        // Send via webhook
        let webhook = match self
            .bot
            .http
            .channel_webhooks(channel_id)
            .await?
            .models()
            .await?
            .into_iter()
            .find(|webhook| webhook.token.is_some())
        {
            Some(webhook) => webhook,
            None => {
                self.bot
                    .http
                    .create_webhook(channel_id, "interchannel message mover")?
                    .await?
                    .model()
                    .await?
            }
        };
        let webhook_token = webhook.token.ok()?;
        let mut execute_webhook = self
            .bot
            .http
            .execute_webhook(webhook.id, &webhook_token);
        execute_webhook = execute_webhook.attachments(attachments).expect("attachments");
        let str_payload = serde_json::to_string(&payload).ok();
        println!("{:?}", str_payload);
        let u8_payload = str_payload.as_ref().map(|s| s.as_bytes());
        if let Some(u8)=u8_payload{
            execute_webhook=execute_webhook.payload_json(u8);
        }
        execute_webhook.await?;
        Ok(())
    }
    pub async fn execute_webhook_as_member(
        &self,
        message: &Message,
        channel: &Channel,
        attachments: &[attachment::Attachment],
    ) -> Result<()> {
        let mut channel_id = channel.id;
        let mut thread_id = None;
        if channel.kind.is_thread() {
            thread_id = Some(channel_id);
            channel_id = channel.parent_id.ok()?;
        };

        let webhook = match self
            .bot
            .http
            .channel_webhooks(channel_id)
            .await?
            .models()
            .await?
            .into_iter()
            .find(|webhook| webhook.token.is_some())
        {
            Some(webhook) => webhook,
            None => {
                self.bot
                    .http
                    .create_webhook(channel_id, "interchannel message mover")?
                    .await?
                    .model()
                    .await?
            }
        };
        let webhook_token = webhook.token.ok()?;

        let mut execute_webhook = self
            .bot
            .http
            .execute_webhook(webhook.id, &webhook_token)
            .attachments(attachments)
            .expect("attachments")
            .content(&message.content)
            .map_err(|_| CustomError::MessageTooLong)?
            .username(
                message
                    .member
                    .as_ref()
                    .and_then(|member| member.nick.as_ref())
                    .unwrap_or(&message.author.name),
            )?;
        let mut embed_array = Vec::new();
        let mut add_embed = false;
        if(!message.embeds.is_empty()){
            for embed in message.embeds.iter(){
                embed_array.push(embed.clone());
            }
            add_embed = true;
        }
        if let Some(ref_msg) = message.reference.as_ref() {
            
            let msg = self.bot.http.message(ref_msg.channel_id.unwrap(), ref_msg.message_id.unwrap()).await?.model().await?;
            let my_embed = EmbedBuilder::new()
                .author(
                    EmbedAuthorBuilder::new(msg.author.name)
                        .icon_url(
                            ImageSource::url(
                                msg.author.avatar.map(|avatar| { format!("https://cdn.discordapp.com/avatars/{}/{}.png", message.author.id, avatar) }).unwrap()
                            )?
                        )
                )
                .footer(EmbedFooterBuilder::new(format!("{}", self.bot.http.channel(msg.channel_id).await?.model().await?.name.unwrap())))
                .url(format!("https://discord.com/channels/{}/{}/{}", ref_msg.guild_id.map(|a| a.to_string()).unwrap_or("@me".to_string()), ref_msg.channel_id.unwrap(), ref_msg.message_id.unwrap()))
                .field(
                    EmbedFieldBuilder::new("Jump", format!("[Go to message](https://discord.com/channels/{}/{}/{})",ref_msg.guild_id.map(|a| a.to_string()).unwrap_or("@me".to_string()), ref_msg.channel_id.unwrap(), ref_msg.message_id.unwrap())).build()
                )
                .description(msg.content.clone())
                .timestamp(msg.timestamp)
                .build();
            embed_array.push(my_embed);
            add_embed = true;
        }
        if add_embed{
            execute_webhook = execute_webhook.embeds(&embed_array)?;       
        }
        if let Some(thread_id) = thread_id {
            execute_webhook = execute_webhook.thread_id(thread_id);
        }

        if let Some(avatar_url) = message
            .member
            .as_ref()
            .and_then(|member| member.avatar)
            .zip(message.guild_id)
            .map(|(avatar, guild_id)| {
                format!(
                    "https://cdn.discordapp.com/guilds/{guild_id}/users/{}/avatar/{}.png",
                    message.author.id, avatar
                )
            })
            .or_else(|| {
                message.author.avatar.map(|avatar| {
                    format!(
                        "https://cdn.discordapp.com/avatars/{}/{}.png",
                        message.author.id, avatar
                    )
                })
            })
        {
            match timeout(Duration::from_secs(60), execute_webhook.avatar_url(&avatar_url)).await{
                Ok(_) => {
                    return Ok(());
                }
                Err(_) => {
                    println!("Failed to send webhook.");
                }
            }
        } else {
            execute_webhook.await?;
        }

        Ok(())
    }
}

pub fn check(message: &Message) -> Result<()> {
    // if !message.attachments.is_empty() {
    //     return Err(CustomError::MessageAttachment.into());
    // }

    Ok(())
}
