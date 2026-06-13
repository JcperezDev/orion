use anyhow::Result;

#[allow(dead_code)]
pub async fn handle_command(_agent: &mut crate::core::agent::Agent, input: &str) -> Result<()> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts[0];

    match cmd {
        "/help" => {
            print_help();
        }
        _ => {}
    }

    Ok(())
}

#[allow(dead_code)]
fn print_help() {
    eprintln!(
        r#"
ORION Commands:
  /help              Show this help
  /providers list    List available providers
  /providers sync    Sync models from OpenRouter
  /models list       List models
  /models search <q> Search models
  /model <id>       Set model
"#
    );
}
