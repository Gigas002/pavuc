//! Prints a one-shot snapshot of the audio server, mirroring what the
//! pavuc TUI shows. Useful as a smoke test of the library.
//!
//! Run with: `cargo run -p libpavuc --example list`

use std::thread::sleep;
use std::time::Duration;

fn main() -> Result<(), libpavuc::Error> {
    let mut client = libpavuc::PulseClient::connect("libpavuc-example")?;

    // Pump the loop a few times to let the initial introspection complete.
    for _ in 0..20 {
        client.iterate()?;
        sleep(Duration::from_millis(10));
    }

    let state = client.snapshot();

    println!("Default sink:   {:?}", state.default_sink);
    println!("Default source: {:?}", state.default_source);

    println!("\nOutput devices ({}):", state.sinks.len());
    for sink in &state.sinks {
        println!(
            "  [{}] {} — {}% {}",
            sink.index,
            sink.description,
            sink.volume_percent(),
            if sink.mute { "(muted)" } else { "" }
        );
    }

    println!("\nInput devices ({}):", state.sources.len());
    for source in &state.sources {
        println!(
            "  [{}] {} — {}%{}",
            source.index,
            source.description,
            source.volume_percent(),
            if source.monitor { " (monitor)" } else { "" }
        );
    }

    println!("\nPlayback streams ({}):", state.sink_inputs.len());
    for stream in &state.sink_inputs {
        println!(
            "  [{}] {} — {}%",
            stream.index,
            stream.name,
            stream.volume_percent()
        );
    }

    println!("\nRecording streams ({}):", state.source_outputs.len());
    for stream in &state.source_outputs {
        println!(
            "  [{}] {} — {}%",
            stream.index,
            stream.name,
            stream.volume_percent()
        );
    }

    println!("\nCards ({}):", state.cards.len());
    for card in &state.cards {
        println!(
            "  [{}] {} — profile: {:?} ({} profiles)",
            card.index,
            card.description,
            card.active_profile,
            card.profiles.len()
        );
    }

    Ok(())
}
