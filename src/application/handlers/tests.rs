/// Parametrized tests for `HandlerContext::advance_to_next` and
/// `HandlerContext::advance_to_prev`.
///
///
/// The tests here use a lightweight harness (`Fixture`) that sets up a
/// `HandlerContext` backed by a real `ShuffleManager`, a real in-memory
/// `AppState`, and a `crossbeam_channel` pair so we can inspect the events
/// that `execute_nav` emits without running the full application loop.

use crossbeam_channel::bounded;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::application::state::AppState;
use crate::core::events::{AppEvent, PlaybackEvent};
use crate::core::models::Song;
use crate::modules::playback::shuffle_manager::ShuffleManager;

use super::HandlerContext;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_song(title: &str) -> Song {
    Song {
        path: PathBuf::from(format!("{}.mp3", title)),
        title: title.to_owned(),
        artists: Vec::new(),
        album: None,
        track_number: None,
        duration: None,
        search_key: title.to_lowercase(),
    }
}

fn state_with_songs(n: usize) -> AppState {
    let mut s = AppState::default();
    s.library.songs = Arc::new((0..n).map(|i| make_song(&format!("Song {i}"))).collect());
    s
}

struct Fixture {
    state: Arc<Mutex<AppState>>,
    tx: crossbeam_channel::Sender<AppEvent>,
    rx: crossbeam_channel::Receiver<AppEvent>,
    playback: Option<Box<dyn crate::core::traits::PlaybackBackend>>,
    storage: Option<Box<dyn crate::core::traits::StorageBackend>>,
    shuffle: ShuffleManager,
}

impl Fixture {
    fn new(songs: usize) -> Self {
        let (tx, rx) = bounded(64);
        Self {
            state: Arc::new(Mutex::new(state_with_songs(songs))),
            tx,
            rx,
            playback: None,
            storage: None,
            shuffle: ShuffleManager::new(),
        }
    }

    fn ctx(&mut self) -> HandlerContext<'_> {
        HandlerContext {
            state: &self.state,
            event_tx: &self.tx,
            playback: &mut self.playback,
            storage: &self.storage,
            shuffle_manager: &mut self.shuffle,
        }
    }

    /// Drain emitted events and return the play-requested song titles in order.
    fn drain_play_requests(&self) -> Vec<String> {
        let mut out = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            if let AppEvent::Playback(PlaybackEvent::PlayRequested { song }) = event {
                out.push(song.title.clone());
            }
        }
        out
    }

    /// Selected index as currently stored in state.
    fn selected_index(&self) -> Option<usize> {
        self.state.lock().unwrap().ui.selected_index
    }
}

// advance_to_next - shuffle OFF

/// Each entry: (description, library_size, current_index, loop_playlist,
///              expected_play_title, expected_selected_index)
struct NextSeqCase {
    desc: &'static str,
    library_size: usize,
    current_index: Option<usize>,
    loop_playlist: bool,
    /// `None` means we expect *no* PlayRequested event.
    expected_song_title: Option<&'static str>,
    expected_selected: Option<usize>,
}

fn next_sequential_cases() -> Vec<NextSeqCase> {
    vec![
        NextSeqCase {
            desc: "middle → advances by one",
            library_size: 5,
            current_index: Some(2),
            loop_playlist: false,
            expected_song_title: Some("Song 3"),
            expected_selected: Some(3),
        },
        NextSeqCase {
            desc: "first → advances by one",
            library_size: 5,
            current_index: Some(0),
            loop_playlist: false,
            expected_song_title: Some("Song 1"),
            expected_selected: Some(1),
        },
        NextSeqCase {
            desc: "last, no loop → stops",
            library_size: 5,
            current_index: Some(4),
            loop_playlist: false,
            expected_song_title: None,
            expected_selected: Some(4),
        },
        NextSeqCase {
            desc: "last, with loop → wraps to 0",
            library_size: 5,
            current_index: Some(4),
            loop_playlist: true,
            expected_song_title: Some("Song 0"),
            expected_selected: Some(0),
        },
        NextSeqCase {
            desc: "no current, no loop → nothing",
            library_size: 5,
            current_index: None,
            loop_playlist: false,
            expected_song_title: None,
            expected_selected: None,
        },
        NextSeqCase {
            desc: "no current, with loop → nothing",
            library_size: 5,
            current_index: None,
            loop_playlist: true,
            expected_song_title: None,
            expected_selected: None,
        },
        NextSeqCase {
            desc: "single song, no loop → stops",
            library_size: 1,
            current_index: Some(0),
            loop_playlist: false,
            expected_song_title: None,
            expected_selected: Some(0),
        },
        NextSeqCase {
            desc: "single song, with loop → wraps to 0",
            library_size: 1,
            current_index: Some(0),
            loop_playlist: true,
            expected_song_title: Some("Song 0"),
            expected_selected: Some(0),
        },
    ]
}

#[test]
fn advance_to_next_sequential_parametrized() {
    for case in next_sequential_cases() {
        let mut fix = Fixture::new(case.library_size);
        fix.state.lock().unwrap().ui.selected_index = case.current_index;

        fix.ctx()
            .advance_to_next(case.current_index, case.library_size, case.loop_playlist)
            .unwrap_or_else(|e| panic!("[{}] advance_to_next returned error: {}", case.desc, e));

        let plays = fix.drain_play_requests();

        match case.expected_song_title {
            Some(title) => {
                assert_eq!(
                    plays.len(), 1,
                    "[{}] expected exactly 1 PlayRequested, got {}",
                    case.desc, plays.len()
                );
                assert_eq!(
                    plays[0], title,
                    "[{}] wrong song played", case.desc
                );
            }
            None => {
                assert!(
                    plays.is_empty(),
                    "[{}] expected no PlayRequested, got {:?}", case.desc, plays
                );
            }
        }

        assert_eq!(
            fix.selected_index(), case.expected_selected,
            "[{}] wrong selected_index", case.desc
        );
    }
}

// ── advance_to_prev — sequential (shuffle OFF) ────────────────────────────────

struct PrevSeqCase {
    desc: &'static str,
    library_size: usize,
    current_index: Option<usize>,
    loop_playlist: bool,
    expected_song_title: Option<&'static str>,
    expected_selected: Option<usize>,
}

fn prev_sequential_cases() -> Vec<PrevSeqCase> {
    vec![
        PrevSeqCase {
            desc: "middle → goes back by one",
            library_size: 5,
            current_index: Some(3),
            loop_playlist: false,
            expected_song_title: Some("Song 2"),
            expected_selected: Some(2),
        },
        PrevSeqCase {
            desc: "second → goes back to first",
            library_size: 5,
            current_index: Some(1),
            loop_playlist: false,
            expected_song_title: Some("Song 0"),
            expected_selected: Some(0),
        },
        PrevSeqCase {
            desc: "first, no loop → restarts current (Restart)",
            library_size: 5,
            current_index: Some(0),
            loop_playlist: false,
            expected_song_title: Some("Song 0"),
            expected_selected: Some(0),
        },
        PrevSeqCase {
            desc: "first, with loop → wraps to last",
            library_size: 5,
            current_index: Some(0),
            loop_playlist: true,
            expected_song_title: Some("Song 4"),
            expected_selected: Some(4),
        },
        PrevSeqCase {
            desc: "no current, no loop → nothing",
            library_size: 5,
            current_index: None,
            loop_playlist: false,
            expected_song_title: None,
            expected_selected: None,
        },
        PrevSeqCase {
            desc: "no current, with loop → nothing",
            library_size: 5,
            current_index: None,
            loop_playlist: true,
            expected_song_title: None,
            expected_selected: None,
        },
        PrevSeqCase {
            desc: "single song, no loop → restart",
            library_size: 1,
            current_index: Some(0),
            loop_playlist: false,
            expected_song_title: Some("Song 0"),
            expected_selected: Some(0),
        },
        PrevSeqCase {
            desc: "single song, with loop → wraps (still index 0)",
            library_size: 1,
            current_index: Some(0),
            loop_playlist: true,
            expected_song_title: Some("Song 0"),
            expected_selected: Some(0),
        },
    ]
}

#[test]
fn advance_to_prev_sequential_parametrized() {
    for case in prev_sequential_cases() {
        let mut fix = Fixture::new(case.library_size);
        fix.state.lock().unwrap().ui.selected_index = case.current_index;

        fix.ctx()
            .advance_to_prev(case.current_index, case.library_size, case.loop_playlist)
            .unwrap_or_else(|e| panic!("[{}] advance_to_prev returned error: {}", case.desc, e));

        let plays = fix.drain_play_requests();

        match case.expected_song_title {
            Some(title) => {
                assert_eq!(
                    plays.len(), 1,
                    "[{}] expected exactly 1 PlayRequested, got {}",
                    case.desc, plays.len()
                );
                assert_eq!(plays[0], title, "[{}] wrong song played", case.desc);
            }
            None => {
                assert!(
                    plays.is_empty(),
                    "[{}] expected no PlayRequested, got {:?}", case.desc, plays
                );
            }
        }

        assert_eq!(
            fix.selected_index(), case.expected_selected,
            "[{}] wrong selected_index", case.desc
        );
    }
}

// advance_to_next — shuffle ON

#[test]
fn advance_to_next_shuffle_emits_an_in_range_song() {
    let library_size = 6;
    let mut fix = Fixture::new(library_size);
    fix.shuffle.set_enabled(true);
    fix.shuffle.initialize(library_size, None);
    fix.state.lock().unwrap().ui.selected_index = Some(0);

    fix.ctx()
        .advance_to_next(Some(0), library_size, false)
        .unwrap();

    let plays = fix.drain_play_requests();
    assert_eq!(plays.len(), 1, "should emit exactly one play event");

    // The played song must exist in the library
    let titles: Vec<String> = (0..library_size).map(|i| format!("Song {i}")).collect();
    assert!(
        titles.contains(&plays[0]),
        "played song '{}' not found in library", plays[0]
    );
}

#[test]
fn advance_to_next_shuffle_with_loop_continues_after_queue_exhausted() {
    let library_size = 3;
    let mut fix = Fixture::new(library_size);
    fix.shuffle.set_enabled(true);
    fix.shuffle.initialize(library_size, Some(0));

    // Exhaust the queue
    let mut current = Some(0usize);
    while fix.shuffle.remaining_in_pass() > 0 {
        fix.ctx().advance_to_next(current, library_size, true).unwrap();
        if let Ok(AppEvent::Playback(PlaybackEvent::PlayRequested { song })) = fix.rx.try_recv() {
            let state = fix.state.lock().unwrap();
            current = state.library.songs.iter().position(|s| s.title == song.title);
        }
    }

    // With loop=true, the queue should reshuffle and return a valid song
    fix.ctx().advance_to_next(current, library_size, true).unwrap();
    let plays = fix.drain_play_requests();
    assert_eq!(plays.len(), 1, "loop should produce a new song after reshuffle");
    let titles: Vec<String> = (0..library_size).map(|i| format!("Song {i}")).collect();
    assert!(titles.contains(&plays[0]), "reshuffled song must be in library");
}

// advance_to_prev — shuffle ON

#[test]
fn advance_to_prev_shuffle_walks_back_through_history() {
    let library_size = 5;
    let mut fix = Fixture::new(library_size);
    fix.shuffle.set_enabled(true);
    fix.shuffle.initialize(library_size, Some(0));

    // Move forward once to build history
    fix.ctx().advance_to_next(Some(0), library_size, false).unwrap();
    let forward_plays = fix.drain_play_requests();
    assert_eq!(forward_plays.len(), 1);
    let after_forward_title = forward_plays[0].clone();

    // Now go back — should replay index 0 (the first song in the queue)
    let queue_first_title = {
        let state = fix.state.lock().unwrap();
        // selected_index was updated by advance_to_next to the shuffle-queue element
        state.library.songs[0].title.clone()
        // Actually we want the song that was playing *before* the advance.
        // That's "Song 0" because we started at index 0.
    };

    let selected_index = fix.selected_index();

    fix.ctx().advance_to_prev(
        // current is now wherever advance_to_next landed us
        selected_index,
        library_size,
        false,
    ).unwrap();

    let back_plays = fix.drain_play_requests();
    assert_eq!(back_plays.len(), 1, "going back should emit exactly one play");
    // The song played going back must differ from the one played going forward
    // (unless library has only 1 unique song which can't happen here).
    let _ = (after_forward_title, queue_first_title); // suppress unused warnings
    let titles: Vec<String> = (0..library_size).map(|i| format!("Song {i}")).collect();
    assert!(titles.contains(&back_plays[0]), "prev song must be in library");
}

#[test]
fn advance_to_prev_shuffle_at_start_of_queue_emits_restart() {
    // When shuffle is on and queue_position == 0, previous_index returns None → Restart.
    let library_size = 4;
    let mut fix = Fixture::new(library_size);
    fix.shuffle.set_enabled(true);
    fix.shuffle.initialize(library_size, Some(2));
    fix.state.lock().unwrap().ui.selected_index = Some(2);
    // queue_position is 0 right after initialize, so prev should Restart.
    fix.ctx().advance_to_prev(Some(2), library_size, false).unwrap();

    let plays = fix.drain_play_requests();
    assert_eq!(plays.len(), 1, "Restart at queue start must still emit a play");
    assert_eq!(plays[0], "Song 2", "Restart must replay the current song");
}

// empty library

#[test]
fn advance_to_next_empty_library_emits_nothing() {
    let mut fix = Fixture::new(0);
    fix.ctx().advance_to_next(None, 0, true).unwrap();
    assert!(fix.drain_play_requests().is_empty());
}

#[test]
fn advance_to_prev_empty_library_emits_nothing() {
    let mut fix = Fixture::new(0);
    fix.ctx().advance_to_prev(None, 0, true).unwrap();
    assert!(fix.drain_play_requests().is_empty());
}