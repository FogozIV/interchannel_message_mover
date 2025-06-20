use crate::interaction::InteractionContext;
use crate::MessageInteractError::NotBoth;
use crate::REQUIRED_PERMISSIONS;
use anyhow::{anyhow, Result};
use sparkle_convenience::interaction::extract::InteractionDataExt;
use sparkle_convenience::reply::Reply;
use std::collections::HashMap;
use twilight_model::application::command::CommandOption;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::{Channel, Message};
use twilight_model::id::marker::{ChannelMarker, GuildMarker};
use twilight_model::id::Id;
use twilight_model::application::command::{Command, CommandType};
use twilight_util::builder::command::{
    BooleanBuilder, ChannelBuilder, CommandBuilder, StringBuilder,
};

pub const CHAT_INPUT_NAME: &str = "move_channel_to";

pub fn slash_command() -> Command {
    let mut map = HashMap::new();
    map.insert(
        "fr".to_string(),
        "Déplace un channel jusqu'au channel correspondant".to_string(),
    );
    CommandBuilder::new(
        CHAT_INPUT_NAME,
        "Move messages between channels (uses current channel if none specified)",
        CommandType::ChatInput,
    )
        .default_member_permissions(REQUIRED_PERMISSIONS)
        .dm_permission(false)
        .option(CommandOption::from(
            ChannelBuilder::new("channel_from", "Source channel (default: current channel)").required(false),
        ))
        .option(CommandOption::from(
            ChannelBuilder::new("channel_to", "the channel where to move the channel to").required(false),
        ))
        .option(CommandOption::from(
            StringBuilder::new("channel_to_name", "the channel where to move the channel to").required(false),
        ))
        .option(CommandOption::from(BooleanBuilder::new(
            "delete_old",
            "delete the old channel",
        )))
        //.default_member_permissions(REQUIRED_PERMISSIONS)
        .description_localizations(map.iter())
        .build()
}


impl InteractionContext<'_> {


    pub async fn handle_move_channel_call(self) -> Result<()> {
        let mut result_channel: Option<Channel> = None;
        let mut remove: Option<bool> = None;
        let mut input_channel: Option<Channel> = None;
        let mut result_channel_name: Option<String> = None;
        if let Some(data) = self.interaction.data.clone() {
            if let Some(command_data) = data.command() {
                for option in &command_data.options {
                    match option.name.as_str() {
                        "channel_from" => {
                            if let CommandOptionValue::Channel(id) = &option.value {
                                input_channel = Some(self.ctx.bot.http.channel(*id).await?.model().await?);
                            }
                        }
                        "channel_to" => {
                            if let CommandOptionValue::Channel(id) = &option.value {
                                result_channel = Some(self.ctx.bot.http.channel(*id).await?.model().await?);
                            }
                        }
                        "channel_to_name" => {
                            if let CommandOptionValue::String(link) = &option.value {
                                result_channel_name = Some(link.clone())
                            }
                        },
                        "delete_old" => {
                            if let CommandOptionValue::Boolean(b) = &option.value {
                                remove = Some(*b);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        match (&result_channel, &result_channel_name) {
            (Some(_), Some(_)) => return Err(anyhow!(NotBoth("channel_to".to_string(), "channel_to_name".to_string()))),
            (None, None) => return Err(anyhow!(NotBoth("channel_to".to_string(), "channel_to_name".to_string()))),
            _=>{}
        }
        if(result_channel.is_none()) {
            let channel = self.ctx.bot.http.create_guild_channel(self.interaction.guild_id.unwrap(), result_channel_name.unwrap().as_str())?.await;
            result_channel =  Some(channel?.model().await?)
        }
        let messages: Vec<Message>;
        let int = self.interaction.clone();
        // Rest of your code remains the same
        input_channel = input_channel.or_else(||{
            int.channel
        });
        
        self.handle.reply(Reply::new().ephemeral().update_last().content("Moving messages...")).await?;
        messages = self.get_all_messages_from_beginning(input_channel.as_ref().unwrap().id).await?;
        self.handle.reply(Reply::new().ephemeral().update_last().content(format!("Found {} messages", messages.len()))).await?;
        let guild_id = int.guild_id.unwrap();
        
        self.move_messages_from_channel_to(guild_id, messages, result_channel.unwrap(), remove.unwrap_or(false), input_channel.unwrap().id, false)
            .await?;
        Ok(())
    }

    pub async fn move_messages_from_channel_to(self, guild_id: Id<GuildMarker>, messages: Vec<Message>, r_channel: Channel, remove_channel: bool, i_channel: Id<ChannelMarker>, remove_msg:bool) -> Result<()> {

        self.move_messages(&messages, &r_channel, guild_id, None).await?;
        if !remove_channel && remove_msg {
            self.bulk_delete(messages, Some(guild_id)).await?;
        }
        if remove_channel {
            self.ctx.bot.http.delete_channel(i_channel).await?;
        }
        self.handle.reply(Reply::new().ephemeral().update_last().content("Done!")).await?;

        Ok(())
    }
}