pub(crate) enum ActionType {
    Liked,
    Shared,
    Posted,
}

pub(crate) struct DescriptionBuilder {
    who: Option<TextedLink>,
    action: Option<ActionType>,
    content: Option<TextedLink>,
    from: Option<TextedLink>,
    description: Option<String>,
    size_limit: Option<usize>,
    achieved_content_size_limit: Option<bool>,
}

impl DescriptionBuilder {
    pub(crate) fn new() -> Self {
        Self {
            who: None,
            action: None,
            content: None,
            from: None,
            description: None,
            size_limit: None,
            achieved_content_size_limit: None,
        }
    }

    pub(crate) fn who(&mut self, text: &str, url: &str) -> &mut Self {
        self.who = Some(TextedLink::new(text, url));
        self
    }

    pub(crate) fn action(&mut self, action: ActionType) -> &mut Self {
        self.action = Some(action);
        self
    }

    pub(crate) fn content(&mut self, text: &str, url: &str) -> &mut Self {
        self.content = Some(TextedLink::new(text, url));
        self
    }

    pub(crate) fn from(&mut self, text: &str, url: &str) -> &mut Self {
        self.from = Some(TextedLink::new(text, url));
        self
    }

    pub(crate) fn description(&mut self, description: String) -> &mut Self {
        self.description = Some(description);
        self
    }

    pub(crate) fn size_limit(&mut self, size_limit: usize) -> &mut Self {
        self.size_limit = Some(size_limit);
        self
    }

    pub(crate) fn achieved_content_size_limit(
        &mut self,
        achieved_content_size_limit: bool,
    ) -> &mut Self {
        self.achieved_content_size_limit = Some(achieved_content_size_limit);
        self
    }

    pub(crate) fn build(&self) -> String {
        let mut description = "<i>".to_string();

        if let Some(achieved_content_size_limit) = self.achieved_content_size_limit {
            if achieved_content_size_limit {
                description.push_str("<b>Attached video is too huge for inline display.</b>\n\n");
            }
        }

        if let Some(who) = &self.who {
            description.push_str(&who.create_link());
        }

        if let Some(action) = &self.action {
            match action {
                ActionType::Liked => description.push_str(" liked"),
                ActionType::Shared => description.push_str(" shared"),
                ActionType::Posted => description.push_str(" posted"),
            }
        } else {
            description.push_str(" processed");
        }

        if let Some(content) = &self.content {
            description.push(' ');
            description.push_str(content.create_link().as_str());
        }

        if let Some(from) = &self.from {
            description.push_str(&format!(" from {}", from.create_link()));
        }
        description.push_str("</i>");

        if let Some(desc) = &self.description {
            description.push_str(&format!(":\n\n{}", desc));
        }

        if let Some(size_limit) = self.size_limit {
            let text = description.clone();
            let mut chars = text.chars();
            description = if description.chars().count() > size_limit {
                let end_message = "... [Read more in the source]";
                chars
                    .by_ref()
                    .take(size_limit - end_message.len())
                    .collect::<String>()
                    + end_message
            } else {
                chars.collect::<String>()
            };
        }

        description
    }
}

struct TextedLink {
    text: String,
    url: String,
}

impl TextedLink {
    fn new(text: &str, url: &str) -> Self {
        Self {
            text: text.to_string(),
            url: url.to_string(),
        }
    }

    fn create_link(&self) -> String {
        format!(
            "<a href=\"{url}\">{text}</a>",
            text = self.text,
            url = self.url
        )
    }
}
