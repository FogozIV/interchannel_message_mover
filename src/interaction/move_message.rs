use anyhow::{anyhow, Result};
use sparkle_convenience::interaction::extract::InteractionDataExt;
use std::collections::HashMap;
use twilight_model::application::command::CommandOption;
use twilight_model::application::command::{Command, CommandType};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::channel::Channel;
use twilight_model::id::marker::{ChannelMarker, MessageMarker};
use twilight_model::id::Id;
use twilight_util::builder::command::{
    BooleanBuilder, ChannelBuilder, CommandBuilder, StringBuilder,
};

use crate::interaction::move_message_and_below::parse_message_link;
use crate::{interaction::InteractionContext, message, REQUIRED_PERMISSIONS};

pub const NAME: &str = "move message";
pub const CHAT_INPUT_NAME: &str = "move_message";

pub const CHAT_INPUT_NAME_2: &str = "move_message_link";

pub fn command() -> Command {
    CommandBuilder::new(NAME, "", CommandType::Message)
        .dm_permission(false)
        .default_member_permissions(REQUIRED_PERMISSIONS)
        .build()
}
pub fn slash_command() -> Command {
    let mut map = HashMap::new();
    map.insert(
        "fr".to_string(),
        "Déplace un message jusqu'au channel correspondant".to_string(),
    );
    CommandBuilder::new(
        CHAT_INPUT_NAME,
        "Move a message to the corresponding channel",
        CommandType::ChatInput,
    )
    .default_member_permissions(REQUIRED_PERMISSIONS)
    .dm_permission(false)
    .option(CommandOption::from(
        StringBuilder::new("message_id", "the message id").required(true),
    ))
    .option(CommandOption::from(
        ChannelBuilder::new("channel", "the channel where to move the message to").required(true),
    ))
    .option(CommandOption::from(BooleanBuilder::new(
        "delete_old",
        "delete the old messages",
    )))
    //.default_member_permissions(REQUIRED_PERMISSIONS)
    .description_localizations(map.iter())
    .build()
}

pub fn slash_command2() -> Command {
    let mut map = HashMap::new();
    map.insert(
        "fr".to_string(),
        "Déplace un message jusqu'au channel correspondant".to_string(),
    );
    CommandBuilder::new(
        CHAT_INPUT_NAME_2,
        "Move a message to the corresponding channel",
        CommandType::ChatInput,
    )
        .default_member_permissions(REQUIRED_PERMISSIONS)
        .dm_permission(false)
        .option(CommandOption::from(
            StringBuilder::new("message_link", "the message link").required(true),
        ))
        .option(CommandOption::from(
            ChannelBuilder::new("channel", "the channel where to move the message to").required(true),
        ))
        .option(CommandOption::from(BooleanBuilder::new(
            "delete_old",
            "delete the old messages",
        )))
        .description_localizations(map.iter())
        .build()
}

impl InteractionContext<'_> {
    pub async fn handle_move_message_command(self) -> Result<()> {
        let message = self.handle_message_command()?;
        message::check(&message)?;
        let channel = self.wait_for_channel_select_interaction().await?;
        self.move_message(message, channel, true).await?;
        Ok(())
    }
    pub async fn handle_command_call(self) -> Result<()> {
        let mut message_id: Option<String> = None;
        let mut result_channel: Option<Id<ChannelMarker>> = None;
        let mut remove: Option<bool> = None;
        let mut i_channel : Option<Channel> = None;
        if let Some(data) = self.interaction.data.clone(){
            if let Some(command_data) = data.command(){
                for option in &command_data.options{
                    match option.name.as_str() {
                        "message_id" => {
                            if let CommandOptionValue::String(id) = &option.value{
                                message_id = Some(id.clone());
                                i_channel = self.interaction.channel.clone();
                            }
                        },
                        "channel" => {
                            if let CommandOptionValue::Channel(id) = &option.value{
                                result_channel = Some(id.clone());
                            }
                        },
                        "delete_old" => {
                            if let CommandOptionValue::Boolean(b) = &option.value{
                                remove = Some(*b);
                            }
                        },
                        "message_link" => {
                            if let CommandOptionValue::String(link) = &option.value{
                                let a = parse_message_link(link).unwrap();
                                i_channel = Some(self.ctx.bot.http.channel(a.1).await?.model().await?);
                                message_id = Some(a.2.to_string());
                            }
                        }
                        _ =>{}
                    }
                }
            }
        }
        if message_id.is_none() || result_channel.is_none(){
            return Err(anyhow!("Missing parameters"));
        }
        let message_id_num = Id::<MessageMarker>::new(
            message_id.ok_or(anyhow!("Error unwraping message ID")).unwrap().parse().map_err(|_| anyhow!("Invalid message ID format"))?
        );
        if i_channel.is_none() {
            return Err(anyhow!("Missing input channel"));
        }
        // Rest of your code remains the same
        let message = self.ctx.bot.http.message(i_channel.unwrap().id, message_id_num)
            .await?
            .model()
            .await?;

        let r_channel = self.ctx.bot.http.channel(result_channel.unwrap())
            .await?
            .model()
            .await?;

        self.move_message(message, r_channel, remove.unwrap_or(false)).await?;
        Ok(())
    }

}
