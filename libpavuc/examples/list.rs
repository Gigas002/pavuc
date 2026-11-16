//! Prints one snapshot of the audio server: devices, streams and cards.
//!
//! A smoke test for libpavuc against a live PulseAudio or `pipewire-pulse`:
//!
//! ```sh
//! cargo run -p libpavuc --example list
//! ```

use std::time::{Duration, Instant};

use libpavuc::{PulseClient, PulseState};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = PulseClient::connect("libpavuc-list")?;

    // The first snapshot arrives over a few main-loop iterations; wait until
    // updates stop for a moment.
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut quiet_since = Instant::now();
    while Instant::now() < deadline && quiet_since.elapsed() < Duration::from_millis(100) {
        if client.iterate()? {
            quiet_since = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    print(&client.snapshot());
    Ok(())
}

/// Names come from other programs (a browser sets the media name from the
/// page title): replace control characters so they can't drive the terminal.
fn clean(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '\u{fffd}' } else { c })
        .collect()
}

fn print(state: &PulseState) {
    println!("Output devices:");
    for sink in &state.sinks {
        let default = if state.is_default_sink(sink) {
            " (default)"
        } else {
            ""
        };
        println!(
            "  #{} {}{default}: {}%",
            sink.index,
            clean(&sink.description),
            sink.volume_percent()
        );
    }
    println!("Input devices:");
    for source in &state.sources {
        let default = if state.is_default_source(source) {
            " (default)"
        } else {
            ""
        };
        println!(
            "  #{} {}{default}: {}%",
            source.index,
            clean(&source.description),
            source.volume_percent()
        );
    }
    println!("Playback:");
    for stream in &state.sink_inputs {
        let device = state
            .sink(stream.device)
            .map_or_else(|| "?".to_string(), |d| clean(&d.description));
        println!(
            "  #{} {} -> {device}: {}%",
            stream.index,
            clean(&stream.name),
            stream.volume_percent()
        );
    }
    println!("Recording:");
    for stream in &state.source_outputs {
        let device = state
            .source(stream.device)
            .map_or_else(|| "?".to_string(), |d| clean(&d.description));
        println!(
            "  #{} {} <- {device}: {}%",
            stream.index,
            clean(&stream.name),
            stream.volume_percent()
        );
    }
    println!("Cards:");
    for card in &state.cards {
        println!(
            "  #{} {}: {}",
            card.index,
            clean(&card.description),
            clean(card.active_profile.as_deref().unwrap_or("(none)"))
        );
    }
}
