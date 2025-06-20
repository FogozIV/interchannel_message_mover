use crate::interaction::move_message_and_below::parse_message_link;
use crate::interaction::InteractionContext;
use crate::MessageInteractError::{IdNotFoundLink, NotBoth, NotInSameChannel};
use crate::REQUIRED_PERMISSIONS;
use anyhow::{anyhow, Result};
use sparkle_convenience::interaction::extract::InteractionDataExt;
use sparkle_convenience::reply::Reply;
use std::collections::HashMap;
use twilight_model::application::command::CommandOption;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::{Channel, Message};
use twilight_model::id::marker::{ChannelMarker, MessageMarker};
use twilight_model::id::Id;
use twilight_model::application::command::{Command, CommandType};
use twilight_util::builder::command::{
    BooleanBuilder, ChannelBuilder, CommandBuilder, StringBuilder,
};

pub const CHAT_INPUT_NAME: &str = "move_channel_to_until";

pub fn slashCommand() -> Command {
    let mut map = HashMap::new();
    map.insert(
        "fr".to_string(),
        "Déplace une partie des msgs d'un channel jusqu'à un autre".to_string(),
    );
    CommandBuilder::new(
        CHAT_INPUT_NAME,
        "Move messages between channels (uses current channel if none specified)",
        CommandType::ChatInput,
    )
        .default_member_permissions(REQUIRED_PERMISSIONS)
        .dm_permission(false)
        .option(CommandOption::from(
            StringBuilder::new("message_from", "Source message url").required(true),
        ))
        .option(CommandOption::from(
            StringBuilder::new("message_to", "Source message url end").required(true),
        ))
        .option(CommandOption::from(
            ChannelBuilder::new("channel_to", "Target channel").required(false),
        ))
        .option(CommandOption::from(
            StringBuilder::new("channel_to_name", "Target channel").required(false),
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

    pub async fn handle_move_to_until_cmd(self) -> Result<()> {
        let mut result_channel: Option<Channel> = None;
        let mut remove: Option<bool> = None;
        let mut from_message: Option<Id<MessageMarker>> = None;
        let mut input_channel: Option<Id<ChannelMarker>> = None;
        let mut to_message: Option<Id<MessageMarker>> = None;
        let mut result_channel_name: Option<String> = None;
        if let Some(data) = self.interaction.data.clone() {
            if let Some(command_data) = data.command() {
                for option in &command_data.options {
                    match option.name.as_str() {
                        "message_from" => {
                            if let CommandOptionValue::String(id) = &option.value {
                                let parsed = parse_message_link(&id);
                                let p = parsed?;
                                from_message = Some(p.2);
                                if input_channel.is_none() {
                                    input_channel = Some(p.1);
                                }else{
                                    if(input_channel.unwrap() != p.1){
                                        return Err(anyhow!(NotInSameChannel))
                                    }
                                }
                            }
                        }
                        "message_to" => {
                            if let CommandOptionValue::String(id) = &option.value {
                                let parsed = parse_message_link(&id);
                                let p = parsed?;
                                to_message = Some(p.2);
                                if input_channel.is_none() {
                                    input_channel = Some(p.1);
                                }else{
                                    if input_channel.unwrap() != p.1 {
                                        return Err(anyhow!(NotInSameChannel))
                                    }
                                }
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
                        }
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
            _ => {}
        }
        if(result_channel.is_none()) {
            let channel = self.ctx.bot.http.create_guild_channel(self.interaction.guild_id.unwrap(), result_channel_name.unwrap().as_str())?.await;
            result_channel =  Some(channel?.model().await?)
        }
        if(from_message.is_none() || to_message.is_none()){
            return Err(anyhow!(IdNotFoundLink))
        }

        let messages: Vec<Message>;
        let int = self.interaction.clone();


        self.handle.reply(Reply::new().ephemeral().update_last().content("Moving messages...")).await?;
        messages = self.get_message_borned(input_channel.unwrap(), from_message.unwrap(), to_message).await?;
        self.handle.reply(Reply::new().ephemeral().update_last().content(format!("Found {} messages", messages.len()))).await?;
        let guild_id = int.guild_id.unwrap();
        
        self.move_messages_from_channel_to(guild_id, messages, result_channel.unwrap(), false, input_channel.unwrap(), remove.unwrap_or(false))
            .await?;
        Ok(())
    }

}
