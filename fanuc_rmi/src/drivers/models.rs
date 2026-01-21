
#[derive(Copy, Debug, Clone, PartialEq)]
pub enum DriverState{
    Running,
    Paused,
    /// Program is paused (RMI_MOVE aborted) but robot can be jogged.
    /// In-flight instructions are preserved for replay on resume.
    ProgramPaused,
}
impl Default for DriverState {
    fn default() -> Self {
        Self::Running
    }
}