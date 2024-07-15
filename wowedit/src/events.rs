use std::marker::PhantomData;
use std::thread;
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender};

pub struct Event<T> {
    _marker : PhantomData<T>,
}