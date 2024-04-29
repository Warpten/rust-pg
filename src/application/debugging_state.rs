/***
 * Configuration that is only active in debug
 */
#[derive(Debug)]
pub struct DebuggingState {
    pub memory : bool,
    pub profiler : bool
}

impl Default for DebuggingState {
    fn default() -> Self {
        DebuggingState { memory : false, profiler : false }
    }
}
