pub mod cli;

use std::error::Error;

use async_openai::{
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestMessageArgs,
        ChatCompletionResponseMessage, CreateChatCompletionRequestArgs,
        CreateCompletionRequestArgs, Role,
    },
    Client,
};

pub struct OpenAIChat {
    client: Client,
    messages: Vec<ChatCompletionRequestMessage>,
}

impl OpenAIChat {
    pub fn new(client: Client, starter_message: ChatCompletionRequestMessage) -> Self {
        Self {
            client,
            messages: vec![starter_message],
        }
    }

    pub async fn complete(
        &mut self,
        user_message: String,
    ) -> Result<ChatCompletionResponseMessage, Box<dyn Error>> {
        let new_message = ChatCompletionRequestMessageArgs::default()
            .role(Role::User)
            .content(user_message)
            .build()?;
        self.messages.push(new_message);
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-4")
            .messages(self.messages.clone())
            .build()?;

        let response = &self.client.chat().create(request).await?.choices[0].message;
        self.messages.push(
            ChatCompletionRequestMessageArgs::default()
                .role(Role::from(response.role.clone()))
                .content(response.content.clone())
                .build()?,
        );
        Ok(response.clone())
    }
}
