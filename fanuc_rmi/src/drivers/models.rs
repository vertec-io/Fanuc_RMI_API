
#[derive(Copy, Debug, Clone, PartialEq)]
pub enum DriverState{
    Running,
    Paused,
}
impl Default for DriverState {
    fn default() -> Self {
        Self::Running
    }
}