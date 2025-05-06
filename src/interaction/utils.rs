use crate::interaction::InteractionContext;
use crate::message;
use sparkle_convenience::reply::Reply;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::Instant;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::{Channel, Message};
use twilight_model::guild::Permissions;
use twilight_model::http::permission_overwrite::{PermissionOverwrite, PermissionOverwriteType};
use twilight_model::id::marker::{ChannelMarker, GuildMarker, MessageMarker, RoleMarker};
use twilight_model::id::Id;

impl InteractionContext<'_> {
    pub async fn bulk_delete(
        &self,
        messages: Vec<Message>,
        guild_id: Option<Id<GuildMarker>>,
    ) -> anyhow::Result<()> {
        let mut messages = messages;

        while (!messages.is_empty()) {
            if (SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
                - u64::try_from(messages[0].timestamp.as_secs())?)
                > 2 * 7 * 24 * 60 * 60
                || messages.len() == 1
            {
                for (idx, message) in messages.iter().enumerate() {
                    if (idx + 1) % 10 == 0 && guild_id.as_ref().is_some() {
                        println!(
                            "deleting messages in {}: {}/{}",
                            guild_id.as_ref().unwrap(),
                            idx + 1,
                            messages.len()
                        );
                    }

                    self.ctx
                        .bot
                        .http
                        .delete_message(message.channel_id, message.id)
                        .await?;

                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            } else {
                self.ctx
                    .bot
                    .http
                    .delete_messages(
                        messages[0].channel_id,
                        &messages
                            .drain(..messages.len().min(100))
                            .map(|message| message.id)
                            .collect::<Vec<_>>(),
                    )?
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_message_borned(
        &self,
        channel: Id<ChannelMarker>,
        from: Id<MessageMarker>,
        to: Option<Id<MessageMarker>>,
    ) -> anyhow::Result<Vec<Message>> {
        let mut from = Some(from);
        let mut to = to;

        if (to.is_some() && from.unwrap() > to.unwrap()) {
            (to, from) = (from, to);
        }

        let mut messages = Vec::new();
        let mut last_message_id: Option<Id<MessageMarker>> = None;
        messages.push(
            self.ctx
                .bot
                .http
                .message(channel, from.unwrap())
                .await?
                .model()
                .await?,
        );
        if (!to.is_some() || from.unwrap() != to.unwrap()) {
            loop {
                last_message_id = messages.last().map(|m| m.id);
                let mut channel_messages;
                let request = self.ctx.bot.http.channel_messages(channel).limit(100)?;
                channel_messages = request
                    .after(last_message_id.unwrap())
                    .await?
                    .model()
                    .await?;

                if channel_messages.is_empty() {
                    break;
                }
                channel_messages.reverse();
                if let Some(found_index) = channel_messages.iter().position(|m| Some(m.id) == to) {
                    messages.extend(channel_messages.drain(..found_index + 1));
                    break;
                }
                messages.extend(channel_messages);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        Ok(messages)
    }
    pub async fn get_all_messages_from_beginning(
        &self,
        id: Id<ChannelMarker>,
    ) -> anyhow::Result<Vec<Message>> {
        let mut messages = Vec::new();
        let mut last_message_id = None;

        loop {
            let channel_messages;
            let request = self.ctx.bot.http.channel_messages(id).limit(100)?;
            if let Some(last_id) = last_message_id {
                channel_messages = request.before(last_id).await?.model().await?;
            } else {
                channel_messages = request.await?.model().await?;
            }
            if channel_messages.is_empty() {
                break;
            }
            messages.extend(channel_messages);
            last_message_id = messages.last().map(|m| m.id);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        messages.reverse();
        Ok(messages)
    }

    pub async fn move_message(
        self,
        message: Message,
        channel: Channel,
        remove: bool,
    ) -> anyhow::Result<()> {
        let message_id = message.id;
        let message_channel_id = message.channel_id;

        // Check if the message has any attachments
        if !message.attachments.is_empty() {
            let mut http_attachments = Vec::new();

            for channel_attachment in &message.attachments {
                // Check if it has a spoiler
                let filename = if let Some(flags) = message.flags {
                    if flags.contains(MessageFlags::EPHEMERAL) {
                        format!("SPOILER_{}", channel_attachment.filename)
                    } else {
                        channel_attachment.filename.clone()
                    }
                } else {
                    channel_attachment.filename.clone()
                };

                let id = channel_attachment.id.into();

                // Download the attachment content
                let file_content = reqwest::get(&channel_attachment.url)
                    .await?
                    .bytes()
                    .await?
                    .to_vec();

                let mut http_attachment = twilight_model::http::attachment::Attachment::from_bytes(
                    filename,
                    file_content,
                    id,
                );
                // Check if the attachment has a description (alt)
                if let Some(description) = &channel_attachment.description {
                    http_attachment.description(description.clone());
                }
                http_attachments.push(http_attachment);
            }
            self.ctx
                .execute_webhook_as_member(&message, &channel, &http_attachments)
                .await?;
        } else {
            self.ctx
                .execute_webhook_as_member(&message, &channel, &[])
                .await?;
        }
        if (remove) {
            self.ctx
                .bot
                .http
                .delete_message(message_channel_id, message_id)
                .await?;
        }
    
        Ok(())
    }
    
    pub async fn display_funny_message(self, messages: &Vec<Message>){
        let reply_content = match messages.len() {
            0..=10 => "starting up the car :red_car:",
            11..=20 => "starting up the truck :pickup_truck:",
            21..=30 => "starting up the truck :truck:",
            31..=40 => "starting up the lorry :articulated_lorry:",
            _ => "starting up the ship :ship: ",
        };
        self.handle.reply(Reply::new().content(reply_content)).await.unwrap();
    }
    
    pub async fn move_messages(&self, messages: &Vec<Message>, result_channel: &Channel, guild_id: Id<GuildMarker>, hide_channel: Option<bool>)-> anyhow::Result<()> {
        let mut hide_channel = hide_channel.unwrap_or(true);
        let mut role_id : Option<Id<RoleMarker>> = None;
        let total = messages.len();
        let mut last_update = Instant::now();
        let update_interval = Duration::from_millis(4000);
        if(hide_channel){
            hide_channel = false;
            let roles = self.ctx.bot.http.roles(guild_id).await?.model().await?;
            for role in roles {
                if role.name == "Hide" {
                    hide_channel = true;
                    role_id = Some(role.id);
                    break;
                }
            }
        }

        let mut hide_channel_permission_overwrite : Option<PermissionOverwrite> = None;

        let mut show_channel_permission_overwrite : Option<PermissionOverwrite>= None;
        if hide_channel {
            hide_channel_permission_overwrite = Some(PermissionOverwrite{
                allow: None,
                deny: Some(Permissions::VIEW_CHANNEL),
                id: role_id.unwrap().cast(),
                kind: PermissionOverwriteType::Role,
            });
            show_channel_permission_overwrite = Some(PermissionOverwrite{
                allow: Some(Permissions::VIEW_CHANNEL),
                deny: None,
                id: role_id.unwrap().cast(),
                kind: PermissionOverwriteType::Role,
            });
            self.ctx.bot.http.update_channel_permission(result_channel.id, &hide_channel_permission_overwrite.unwrap()).await?;
        }
        for message in messages {
            message::check(message)?;
        }

        for (idx, message) in messages.iter().enumerate() {
            if last_update.elapsed() >= update_interval || idx == total - 1 {
                self.show_progress(idx, total).await?;
                last_update = Instant::now();
            }

            if (idx + 1) % 10 == 0 {
                println!(
                    "moving messages in {guild_id}: {}/{}",
                    idx + 1,
                    messages.len()
                );
            }

            // Check if the message has any attachments
            if !message.attachments.is_empty() {
                let mut http_attachments = Vec::new();
                println!("Found {} attachments", message.attachments.len());
                for channel_attachment in &message.attachments {
                    // Check if it has a spoiler
                    let filename = if let Some(flags) = message.flags {
                        if flags.contains(MessageFlags::EPHEMERAL) {
                            format!("SPOILER_{}", channel_attachment.filename)
                        } else {
                            channel_attachment.filename.clone()
                        }
                    } else {
                        channel_attachment.filename.clone()
                    };

                    let id = channel_attachment.id.into();

                    // Download the attachment content
                    let file_content = reqwest::get(&channel_attachment.url)
                        .await?
                        .bytes()
                        .await?
                        .to_vec();
                    println!(
                        "Downloaded {} bytes from {}",
                        file_content.len(),
                        channel_attachment.url
                    );
                    let mut http_attachment =
                        twilight_model::http::attachment::Attachment::from_bytes(
                            filename,
                            file_content,
                            id,
                        );
                    // Check if the attachment has a description (alt)
                    if let Some(description) = &channel_attachment.description {
                        http_attachment.description(description.clone());
                    }
                    http_attachments.push(http_attachment);
                }

                self.ctx
                    .execute_webhook_as_member(&message, &result_channel, &http_attachments)
                    .await?;
            } else {
                // Send the message content
                self.ctx
                    .execute_webhook_as_member(&message, &result_channel, &[])
                    .await?;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        if hide_channel {
            self.ctx.bot.http.update_channel_permission(result_channel.id, &show_channel_permission_overwrite.unwrap()).await?;
        }
        Ok(())
    }
    pub async fn show_progress(&self, current: usize, total: usize) -> anyhow::Result<()> {
        let progress_bar = create_progress_bar(current, total);

        self.handle.reply(
            Reply::new()
                .ephemeral()
                .update_last()  // Edit the previous message
                .content(&format!("Moving {}/{}:\n{}", current + 1, total, progress_bar))
        ).await?;

        Ok(())
    }
}

fn create_progress_bar(progress: usize, total: usize) -> String {
    const BAR_LENGTH: usize = 40;
    let filled = (progress * BAR_LENGTH) / total;
    let empty = BAR_LENGTH - filled;

    format!(
        "[{}{}] {}%",
        "=".repeat(filled),
        " ".repeat(empty),
        (progress * 100) / total
    )
}
