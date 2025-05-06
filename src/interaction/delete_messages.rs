use crate::interaction::move_message_and_below::parse_message_link;
use crate::interaction::InteractionContext;
use crate::MessageInteractError::NotInSameChannel;
use crate::REQUIRED_PERMISSIONS;
use anyhow::{anyhow, Result};
use sparkle_convenience::interaction::extract::InteractionDataExt;
use sparkle_convenience::reply::Reply;
use std::collections::HashMap;
use twilight_model::application::command::CommandOption;
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::id::marker::{ChannelMarker, MessageMarker};
use twilight_model::id::Id;
use twilight_model::application::command::{Command, CommandType};
use twilight_util::builder::command::{
    CommandBuilder, StringBuilder,
};

pub const CHAT_INPUT_NAME: &str = "delete_messages";

pub fn slashCommand() -> Command {
    let mut map = HashMap::new();
    map.insert(
        "fr".to_string(),
        "Supprime une partie des msgs".to_string(),
    );
    CommandBuilder::new(
        CHAT_INPUT_NAME,
        "Delete messages",
        CommandType::ChatInput,
    )
        .default_member_permissions(REQUIRED_PERMISSIONS)
        .dm_permission(false)
        .option(CommandOption::from(
            StringBuilder::new("message_from", "Source message url").required(true),
        ))
        .option(CommandOption::from(
            StringBuilder::new("message_to", "Source message url end").required(false),
        ))
        //.default_member_permissions(REQUIRED_PERMISSIONS)
        .description_localizations(map.iter())
        .build()
}

impl InteractionContext<'_> {
    
    pub async fn handle_delete_cmd(self) -> Result<()>{
        let mut from_message: Option<Id<MessageMarker>> = None;
        let mut channel: Option<Id<ChannelMarker>> = None;
        let mut to_message: Option<Id<MessageMarker>> = None;
        if let Some(data) = self.interaction.data.clone() {
            if let Some(command_data) = data.command() {
                for option in &command_data.options {
                    match option.name.as_str() {
                        "message_from" => {
                            if let CommandOptionValue::String(id) = &option.value {
                                let parsed = parse_message_link(&id);
                                let p = parsed?;
                                from_message = Some(p.2);
                                if(channel.is_none()){
                                    channel = Some(p.1);
                                }else{
                                    if(channel.unwrap() != p.1){
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
                                if(channel.is_none()){
                                    channel = Some(p.1);
                                }else{
                                    if(channel.unwrap() != p.1){
                                        return Err(anyhow!(NotInSameChannel))
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        let from_message = from_message.ok_or_else(|| anyhow!("No message from"))?;
        let channel = channel.ok_or_else(|| anyhow!("No channel"))?;
        self.handle.reply(Reply::new().ephemeral().update_last().content("Deleting messages...")).await?;
        let list = self.get_message_borned(channel, from_message, to_message).await?;
        self.handle.reply(Reply::new().ephemeral().update_last().content(format!("Found {} messages...", list.len()))).await?;
        self.bulk_delete(list, None).await?;
        self.handle.reply(Reply::new().ephemeral().update_last().content("Done!")).await?;
        
        Ok(())

    }
    
}