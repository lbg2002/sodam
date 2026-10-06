//! Linux MPRIS2 integration.
//!
//! The D-Bus server lives on its own thread. GPUI only exchanges small commands
//! and a compact player-state snapshot with it, so D-Bus never blocks rendering.

#![cfg(target_os = "linux")]

use dbus::arg::{PropMap, RefArg, Variant};
use dbus::blocking::stdintf::org_freedesktop_dbus::PropertiesPropertiesChanged;
use dbus::blocking::LocalConnection;
use dbus::channel::Sender as _;
use dbus::message::SignalArgs;
use dbus_tree::{Access, Factory};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MprisState {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: String,
    pub playing: bool,
    pub position_micros: i64,
    pub duration_micros: i64,
    pub volume: f64,
    pub can_go_next: bool,
    pub can_go_previous: bool,
    pub can_play: bool,
    pub loop_status: String,
    pub shuffle: bool,
}

#[derive(Debug)]
pub enum MprisCommand {
    Raise,
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    SeekRelative(i64),
    SetVolume(f64),
}

pub struct MprisBridge {
    pub receiver: mpsc::Receiver<MprisCommand>,
    pub state: Arc<Mutex<MprisState>>,
}

pub fn start() -> MprisBridge {
    let (tx, receiver) = mpsc::channel();
    let state = Arc::new(Mutex::new(MprisState {
        volume: 1.0,
        loop_status: "None".to_string(),
        ..Default::default()
    }));
    let thread_state = state.clone();
    std::thread::Builder::new()
        .name("sodam-mpris".to_string())
        .spawn(move || {
            if let Err(err) = run_server(thread_state, tx) {
                eprintln!("[mpris] service stopped: {err}");
            }
        })
        .ok();
    MprisBridge { receiver, state }
}

fn sanitize_path_segment(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "none".to_string()
    } else {
        out
    }
}

fn metadata(state: &MprisState) -> PropMap {
    let mut map = PropMap::new();
    let path = dbus::Path::new(format!(
        "/org/mpris/MediaPlayer2/track/{}",
        sanitize_path_segment(&state.track_id)
    ))
    .unwrap_or_else(|_| {
        dbus::Path::new("/org/mpris/MediaPlayer2/track/none").expect("valid MPRIS object path")
    });

    map.insert(
        "mpris:trackid".to_string(),
        Variant(Box::new(path) as Box<dyn RefArg>),
    );
    map.insert(
        "xesam:title".to_string(),
        Variant(Box::new(state.title.clone()) as Box<dyn RefArg>),
    );
    map.insert(
        "xesam:artist".to_string(),
        Variant(Box::new(vec![state.artist.clone()]) as Box<dyn RefArg>),
    );
    map.insert(
        "xesam:album".to_string(),
        Variant(Box::new(state.album.clone()) as Box<dyn RefArg>),
    );
    map.insert(
        "mpris:length".to_string(),
        Variant(Box::new(state.duration_micros) as Box<dyn RefArg>),
    );
    if !state.art_url.trim().is_empty() {
        map.insert(
            "mpris:artUrl".to_string(),
            Variant(Box::new(state.art_url.clone()) as Box<dyn RefArg>),
        );
    }
    map
}

fn emit_properties_changed(connection: &LocalConnection, state: &MprisState) {
    let mut changed = PropMap::new();
    changed.insert(
        "PlaybackStatus".to_string(),
        Variant(Box::new(
            if state.playing { "Playing" } else { "Paused" }.to_string(),
        ) as Box<dyn RefArg>),
    );
    changed.insert(
        "Metadata".to_string(),
        Variant(Box::new(metadata(state)) as Box<dyn RefArg>),
    );
    changed.insert(
        "Volume".to_string(),
        Variant(Box::new(state.volume) as Box<dyn RefArg>),
    );
    changed.insert(
        "CanGoNext".to_string(),
        Variant(Box::new(state.can_go_next) as Box<dyn RefArg>),
    );
    changed.insert(
        "CanGoPrevious".to_string(),
        Variant(Box::new(state.can_go_previous) as Box<dyn RefArg>),
    );
    changed.insert(
        "CanPlay".to_string(),
        Variant(Box::new(state.can_play) as Box<dyn RefArg>),
    );
    changed.insert(
        "LoopStatus".to_string(),
        Variant(Box::new(state.loop_status.clone()) as Box<dyn RefArg>),
    );
    changed.insert(
        "Shuffle".to_string(),
        Variant(Box::new(state.shuffle) as Box<dyn RefArg>),
    );

    let signal = PropertiesPropertiesChanged {
        interface_name: "org.mpris.MediaPlayer2.Player".to_string(),
        changed_properties: changed,
        invalidated_properties: Vec::new(),
    };
    let path = dbus::Path::new("/org/mpris/MediaPlayer2").expect("valid MPRIS path");
    let _ = connection.send(signal.to_emit_message(&path));
}

fn observable_state_changed(previous: &MprisState, current: &MprisState) -> bool {
    previous.track_id != current.track_id
        || previous.title != current.title
        || previous.artist != current.artist
        || previous.album != current.album
        || previous.art_url != current.art_url
        || previous.playing != current.playing
        || (previous.volume - current.volume).abs() > f64::EPSILON
        || previous.can_go_next != current.can_go_next
        || previous.can_go_previous != current.can_go_previous
        || previous.can_play != current.can_play
        || previous.loop_status != current.loop_status
        || previous.shuffle != current.shuffle
}

fn run_server(
    state: Arc<Mutex<MprisState>>,
    commands: mpsc::Sender<MprisCommand>,
) -> Result<(), Box<dyn std::error::Error>> {
    let connection = LocalConnection::new_session()?;
    connection.request_name("org.mpris.MediaPlayer2.sodam", false, true, false)?;

    let factory = Factory::new_fn::<()>();

    let raise_tx = commands.clone();
    let root_iface = factory
        .interface("org.mpris.MediaPlayer2", ())
        .add_m(factory.method("Raise", (), move |info| {
            let _ = raise_tx.send(MprisCommand::Raise);
            Ok(vec![info.msg.method_return()])
        }))
        .add_p(
            factory
                .property::<bool, _>("CanQuit", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append(false);
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<bool, _>("CanRaise", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append(true);
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<bool, _>("HasTrackList", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append(false);
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<String, _>("Identity", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append("SodaM".to_string());
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<String, _>("DesktopEntry", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append("sodam".to_string());
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<Vec<String>, _>("SupportedUriSchemes", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append(Vec::<String>::new());
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<Vec<String>, _>("SupportedMimeTypes", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append(Vec::<String>::new());
                    Ok(())
                }),
        );

    let play_tx = commands.clone();
    let pause_tx = commands.clone();
    let toggle_tx = commands.clone();
    let stop_tx = commands.clone();
    let next_tx = commands.clone();
    let prev_tx = commands.clone();
    let seek_tx = commands.clone();

    let playback_state = state.clone();
    let metadata_state = state.clone();
    let loop_state = state.clone();
    let shuffle_state = state.clone();
    let volume_state = state.clone();
    let position_state = state.clone();
    let next_state = state.clone();
    let previous_state = state.clone();
    let can_play_state = state.clone();
    let volume_tx = commands.clone();

    let player_iface = factory
        .interface("org.mpris.MediaPlayer2.Player", ())
        .add_m(factory.method("Play", (), move |info| {
            let _ = play_tx.send(MprisCommand::Play);
            Ok(vec![info.msg.method_return()])
        }))
        .add_m(factory.method("Pause", (), move |info| {
            let _ = pause_tx.send(MprisCommand::Pause);
            Ok(vec![info.msg.method_return()])
        }))
        .add_m(factory.method("PlayPause", (), move |info| {
            let _ = toggle_tx.send(MprisCommand::PlayPause);
            Ok(vec![info.msg.method_return()])
        }))
        .add_m(factory.method("Stop", (), move |info| {
            let _ = stop_tx.send(MprisCommand::Stop);
            Ok(vec![info.msg.method_return()])
        }))
        .add_m(factory.method("Next", (), move |info| {
            let _ = next_tx.send(MprisCommand::Next);
            Ok(vec![info.msg.method_return()])
        }))
        .add_m(factory.method("Previous", (), move |info| {
            let _ = prev_tx.send(MprisCommand::Previous);
            Ok(vec![info.msg.method_return()])
        }))
        .add_m(
            factory
                .method("Seek", (), move |info| {
                    let offset: i64 = info.msg.read1()?;
                    let _ = seek_tx.send(MprisCommand::SeekRelative(offset));
                    Ok(vec![info.msg.method_return()])
                })
                .inarg::<i64, _>("Offset"),
        )
        .add_p(
            factory
                .property::<String, _>("PlaybackStatus", ())
                .access(Access::Read)
                .on_get(move |iter, _| {
                    let playing = playback_state
                        .lock()
                        .map(|state| state.playing)
                        .unwrap_or(false);
                    iter.append(if playing {
                        "Playing".to_string()
                    } else {
                        "Paused".to_string()
                    });
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<String, _>("LoopStatus", ())
                .access(Access::Read)
                .on_get(move |iter, _| {
                    let value = loop_state
                        .lock()
                        .map(|state| state.loop_status.clone())
                        .unwrap_or_else(|_| "None".to_string());
                    iter.append(value);
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<f64, _>("Rate", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append(1.0f64);
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<bool, _>("Shuffle", ())
                .access(Access::Read)
                .on_get(move |iter, _| {
                    iter.append(
                        shuffle_state
                            .lock()
                            .map(|state| state.shuffle)
                            .unwrap_or(false),
                    );
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<PropMap, _>("Metadata", ())
                .access(Access::Read)
                .on_get(move |iter, _| {
                    let value = metadata_state
                        .lock()
                        .map(|state| metadata(&state))
                        .unwrap_or_default();
                    iter.append(value);
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<f64, _>("Volume", ())
                .access(Access::ReadWrite)
                .on_get(move |iter, _| {
                    iter.append(
                        volume_state
                            .lock()
                            .map(|state| state.volume)
                            .unwrap_or(1.0),
                    );
                    Ok(())
                })
                .on_set(move |iter, _| {
                    let value: f64 = iter.read()?;
                    let _ = volume_tx.send(MprisCommand::SetVolume(value));
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<i64, _>("Position", ())
                .access(Access::Read)
                .on_get(move |iter, _| {
                    iter.append(
                        position_state
                            .lock()
                            .map(|state| state.position_micros)
                            .unwrap_or(0),
                    );
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<f64, _>("MinimumRate", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append(1.0f64);
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<f64, _>("MaximumRate", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append(1.0f64);
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<bool, _>("CanGoNext", ())
                .access(Access::Read)
                .on_get(move |iter, _| {
                    iter.append(
                        next_state
                            .lock()
                            .map(|state| state.can_go_next)
                            .unwrap_or(false),
                    );
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<bool, _>("CanGoPrevious", ())
                .access(Access::Read)
                .on_get(move |iter, _| {
                    iter.append(
                        previous_state
                            .lock()
                            .map(|state| state.can_go_previous)
                            .unwrap_or(false),
                    );
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<bool, _>("CanPlay", ())
                .access(Access::Read)
                .on_get(move |iter, _| {
                    iter.append(
                        can_play_state
                            .lock()
                            .map(|state| state.can_play)
                            .unwrap_or(false),
                    );
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<bool, _>("CanPause", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append(true);
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<bool, _>("CanSeek", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append(true);
                    Ok(())
                }),
        )
        .add_p(
            factory
                .property::<bool, _>("CanControl", ())
                .access(Access::Read)
                .on_get(|iter, _| {
                    iter.append(true);
                    Ok(())
                }),
        );

    let tree = factory.tree(()).add(
        factory
            .object_path("/org/mpris/MediaPlayer2", ())
            .introspectable()
            .add(root_iface)
            .add(player_iface),
    );
    tree.start_receive(&connection);

    let mut last_emitted = MprisState::default();
    loop {
        connection.process(Duration::from_millis(250))?;
        if let Ok(current) = state.lock().map(|state| state.clone()) {
            if observable_state_changed(&last_emitted, &current) {
                emit_properties_changed(&connection, &current);
                last_emitted = current;
            }
        }
    }
}
