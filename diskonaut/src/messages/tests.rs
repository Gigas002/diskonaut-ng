use ::std::sync::mpsc;
use ::std::thread;

use super::{Event, Instruction, handle_events};

fn name(instruction: &Instruction) -> &'static str {
    match instruction {
        Instruction::SetPathToRed => "SetPathToRed",
        Instruction::ResetCurrentPathColor => "ResetCurrentPathColor",
        Instruction::FlashSpaceFreed => "FlashSpaceFreed",
        Instruction::UnflashSpaceFreed => "UnflashSpaceFreed",
        Instruction::Render => "Render",
        _ => "other",
    }
}

/// Each event becomes a flash and its reset, in that order. The receiver
/// blocks for them, so the test doesn't depend on the 250 ms in between
/// (it used to sleep 600 ms and hope the reset had arrived).
#[test]
fn events_flash_then_reset() {
    let (event_tx, event_rx) = mpsc::sync_channel(1);
    let (instruction_tx, instruction_rx) = mpsc::sync_channel(16);
    let worker = thread::spawn(move || handle_events(event_rx, instruction_tx));

    let next_four = |event| {
        event_tx.send(event).expect("send event");
        (0..4)
            .map(|_| name(&instruction_rx.recv().expect("instruction")))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        next_four(Event::PathError),
        ["SetPathToRed", "Render", "ResetCurrentPathColor", "Render"]
    );
    assert_eq!(
        next_four(Event::FileDeleted),
        ["FlashSpaceFreed", "Render", "UnflashSpaceFreed", "Render"]
    );

    event_tx.send(Event::AppExit).expect("send exit");
    worker.join().expect("join event handler");
}
