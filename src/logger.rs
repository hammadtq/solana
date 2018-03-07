//! The `logger` crate provides an object for generating a Proof-of-History.
//! It logs Event items on behalf of its users. It continuously generates
//! new hashes, only stopping to check if it has been sent an Event item. It
//! tags each Event with an Entry and sends it back. The Entry includes the
//! Event, the latest hash, and the number of hashes since the last event.
//! The resulting stream of entries represents ordered events in time.

use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};
use std::time::{Duration, Instant};
use hash::Hash;
use entry::{create_entry_mut, Entry};
use event::Event;
use serde_json;

#[derive(Debug, PartialEq, Eq)]
pub enum ExitReason {
    RecvDisconnected,
    SendDisconnected,
}

pub struct Logger {
    pub sender: SyncSender<Entry>,
    pub receiver: Receiver<Event>,
    pub last_id: Hash,
    pub num_hashes: u64,
    pub num_ticks: u64,
}

impl Logger {
    pub fn new(receiver: Receiver<Event>, sender: SyncSender<Entry>, start_hash: Hash) -> Self {
        Logger {
            receiver,
            sender,
            last_id: start_hash,
            num_hashes: 0,
            num_ticks: 0,
        }
    }

    pub fn log_event(&mut self, event: Event) -> Result<Entry, ExitReason> {
        let entry = create_entry_mut(&mut self.last_id, &mut self.num_hashes, event);
        println!("{}", serde_json::to_string(&entry).unwrap());
        Ok(entry)
    }

    pub fn process_events(
        &mut self,
        epoch: Instant,
        ms_per_tick: Option<u64>,
    ) -> Result<(), ExitReason> {
        loop {
            if let Some(ms) = ms_per_tick {
                if epoch.elapsed() > Duration::from_millis((self.num_ticks + 1) * ms) {
                    self.log_event(Event::Tick)?;
                    self.num_ticks += 1;
                }
            }

            match self.receiver.try_recv() {
                Ok(event) => {
                    let entry = self.log_event(event)?;
                    self.sender
                        .send(entry)
                        .or(Err(ExitReason::SendDisconnected))?;
                }
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => return Err(ExitReason::RecvDisconnected),
            };
        }
    }
}