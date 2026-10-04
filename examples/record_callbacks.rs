//! Refresh the two static callback recordings without starting a game.
use pdx_native::Native;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let installation = args.next().ok_or("expected installation path")?;
    let output = args.next().ok_or("expected recording directory")?;
    if args.next().is_some() {
        return Err("expected only installation and recording paths".into());
    }
    let native = Native::open(installation)?.record_answers_to(output);
    let on_actions = native.on_actions()?;
    let game_rules = native.game_rules()?;
    println!(
        "Recorded {} on_actions and {} game rules",
        on_actions.value.len(),
        game_rules.value.len()
    );
    Ok(())
}
