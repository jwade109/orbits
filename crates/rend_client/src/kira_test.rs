use std::time::Instant;

use kira::clock::*;
use kira::effect::filter::*;
use kira::sound::static_sound::StaticSoundData;
use kira::track::*;
use kira::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const TEMPO: f64 = 168.0 / 2.0 * 1.2;

    let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;
    // Create a clock that ticks 120 times per minute. In this case,
    // each tick is one musical beat. We can use a tick to represent any
    // arbitrary amount of time.
    let mut clock = manager.add_clock(ClockSpeed::TicksPerMinute(TEMPO))?;
    // Play a sound 2 ticks (beats) from now.

    let mut sounds = Vec::new();
    let mut handles = Vec::new();

    let mut builder = TrackBuilder::new();
    let mut filter = builder.add_effect(
        FilterBuilder::new()
            .cutoff(1500.0)
            .mode(FilterMode::LowPass),
    );

    let mut track = manager.add_sub_track(builder)?;

    let mut i = 0;
    for suffix in ["bass", "high", "mids"] {
        let freq = (i + 1) as f64 / 10.0;
        let path = format!("assets/sfx/winter-morning-sea-smoke-{suffix}.ogg");
        sounds.push((path.clone(), StaticSoundData::from_file(path)?, freq));
        i += 3;
    }

    for (path, sound, freq) in sounds {
        let mut handle = track.play(sound.clone())?;
        handle.set_playback_rate(1.0, kira::Tween::default());
        handles.push((path, handle, sound, freq));
    }

    // Start the clock.
    clock.start();

    let start = Instant::now();

    loop {
        println!("{:?}", clock.time());
        let t = Instant::now().duration_since(start).as_secs_f64();
        let cutoff = 1200.0 + 800.0 * t.sin();

        filter.set_cutoff(cutoff, Tween::default());

        for (path, handle, _sound, freq) in &mut handles {
            // let linear = (t * *freq).sin() * 0.5 + 0.5;
            // let dbs = 20.0 * (linear as f32).log10();
            // let volume = kira::Decibels::from(dbs);
            // handle.set_volume(volume, kira::Tween::default());
            // println!("{} {}: {:?} {:0.2} {:0.2}", t, path, handle.state(), handle.position(), linear);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
