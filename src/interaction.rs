use crate::{err_reply, Context, CustomError, Error, MessageInteractError, TEST_GUILD_ID};
use anyhow::Result;
use sparkle_convenience::reply::Reply;
use sparkle_convenience::{
    error::IntoError,
    interaction::{extract::InteractionExt, InteractionHandle},
    Bot,
};
use twilight_model::application::interaction::Interaction;

mod channel_select_menu;
mod message_command;
mod move_channel_select;
mod move_message;
mod move_message_and_below;
mod move_to_channel;
mod move_to_until;
mod delete_messages;
mod utils;

struct InteractionContext<'ctx> {
    ctx: &'ctx Context,
    handle: InteractionHandle<'ctx>,
    interaction: Interaction
}

impl<'ctx> InteractionContext<'ctx> {
    async fn _handle(self) -> Result<()> {
        match self.interaction.name().ok()? {
            move_message::NAME => self.handle_move_message_command().await,
            move_message_and_below::NAME => self.handle_move_message_and_below_command().await,
            move_message::CHAT_INPUT_NAME => self.handle_command_call().await,
            move_message_and_below::CHAT_INPUT_NAME => self.handle_move_and_below_command_call().await,
            move_message::CHAT_INPUT_NAME_2 => self.handle_command_call().await,
            move_message_and_below::CHAT_INPUT_NAME_2 => self.handle_move_and_below_command_call().await,
            move_to_channel::CHAT_INPUT_NAME => self.handle_move_channel_call().await,
            move_to_until::CHAT_INPUT_NAME => self.handle_move_to_until_cmd().await,
            delete_messages::CHAT_INPUT_NAME => self.handle_delete_cmd().await,
            move_channel_select::CUSTOM_ID => Ok(()),
            name => Err(Error::UnknownCommand(name.to_owned()).into()),
        }
    }

    pub async fn handle(self) -> Result<()> {
        let handle = self.handle.clone();
        match self._handle().await {
            Ok(_) => Ok(()),
            Err(err) => {
                if let Some(interaction_error) = err.downcast_ref::<MessageInteractError>() {
                    handle.reply(Reply::new().ephemeral().update_last().content(interaction_error.to_string())).await?;
                    Ok(())
                }else{
                    Err(err)
                }
            }
        }
    }
    
}

pub async fn set_commands(bot: &Bot) -> Result<()> {
    let commands = &[
        move_message::command(), 
        move_message_and_below::command(), 
        move_message::slash_command(), 
        move_message_and_below::slash_command(), 
        move_message::slash_command2(), 
        move_message_and_below::slash_command2(), 
        move_to_channel::slash_command(),
        move_to_until::slashCommand(),
        delete_messages::slashCommand(),
    ];
    
    bot.interaction_client()
        .set_global_commands(commands)
        .await?;
    bot.interaction_client()
        .set_guild_commands(TEST_GUILD_ID, commands)
        .await?;

    Ok(())
}

impl Context {
    pub async fn handle_interaction(&self, interaction: Interaction) {
        let handle = self.bot.interaction_handle(&interaction);
        let ctx = InteractionContext {
            ctx: self,
            handle: handle.clone(),
            interaction,
        };

        if let Err(err) = ctx.handle().await {
            handle
                .handle_error::<CustomError>(err_reply(&err), err)
                .await;
        }
    }
}
