macro_rules! generate_api_handler {
    ($name: ident, $api: ty, $enum_name: ident) => {
        async fn $name(
            message: Message,
            bot: AutoSend<Throttle<Bot>>,
            command: $enum_name,
            cfg: ConfigParameters,
        ) -> anyhow::Result<()> {
            let chat_id = message.chat.id.to_string();
            match command {
                $enum_name::LastSub1(username) => {
                    last_n_data::<$api>(
                        &bot,
                        &cfg.req_sender,
                        username,
                        1,
                        &chat_id,
                        SubscriptionType::Subscription1,
                    )
                    .await
                }
                $enum_name::LastSub2(username) => {
                    last_n_data::<$api>(
                        &bot,
                        &cfg.req_sender,
                        username,
                        1,
                        &chat_id,
                        SubscriptionType::Subscription2,
                    )
                    .await
                }
                $enum_name::LastNSub1(username, n) => {
                    last_n_data::<$api>(
                        &bot,
                        &cfg.req_sender,
                        username,
                        n,
                        &chat_id,
                        SubscriptionType::Subscription2,
                    )
                    .await
                }
                $enum_name::LastNSub2(username, n) => {
                    last_n_data::<$api>(
                        &bot,
                        &cfg.req_sender,
                        username,
                        n,
                        &chat_id,
                        SubscriptionType::Subscription2,
                    )
                    .await
                }
                $enum_name::Subscribe1(username) => {
                    subscribe::<$api>(
                        &bot,
                        &cfg.req_sender,
                        username,
                        &chat_id,
                        SubscriptionType::Subscription1,
                    )
                    .await
                }
                $enum_name::Subscribe2(username) => {
                    subscribe::<$api>(
                        &bot,
                        &cfg.req_sender,
                        username,
                        &chat_id,
                        SubscriptionType::Subscription2,
                    )
                    .await
                }
                $enum_name::Unsubscribe1(username) => {
                    unsubscribe::<$api>(&bot, username, &chat_id, SubscriptionType::Subscription1)
                        .await
                }
                $enum_name::Unsubscribe2(username) => {
                    unsubscribe::<$api>(&bot, username, &chat_id, SubscriptionType::Subscription2)
                        .await
                }
            }
        }
    };
}
