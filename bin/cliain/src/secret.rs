use std::io::Result;

use dialoguer::theme::ColorfulTheme;

pub fn prompt_password_hidden(message: &str) -> Result<String> {
    let theme = ColorfulTheme::default();
    let mut input = dialoguer::Password::with_theme(&theme);

    input.with_prompt(message).allow_empty_password(false);
    let value = input.interact()?;
    Ok(value)
}
