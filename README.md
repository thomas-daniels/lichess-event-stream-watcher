Usage: create `src/conf.rs` that looks like this:

```
pub const TOKEN: &'static str = "Lichess API token";
pub const RULES_PATH: &'static str = "rules/rules.txt";
pub const SLACK_BOT_TOKEN: &'static str = "Slack bot token";
pub const SLACK_BOT_USER_ID: &'static str = "Slack bot user ID";
pub const SLACK_CHANNEL: &'static str = "Slack channel ID";
```

Then run with cargo: `cargo run`.