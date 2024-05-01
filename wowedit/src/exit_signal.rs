use std::process::ExitCode;

#[derive(Debug, Clone)]
pub struct ExitSignal {
    tx: std::sync::mpsc::Sender<ExitCode>,
}

impl ExitSignal {
    /// send exit signal.
    pub fn send(&self, exit_code: ExitCode) {
        self.tx.send(exit_code).unwrap();
    }
}