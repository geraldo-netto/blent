//! Process entry keeps the GUI workflow on its original calling thread.
pub(crate) fn run_gui(work: impl FnOnce() -> eframe::Result) -> eframe::Result {
    #[cfg(target_os = "linux")]
    {
        blent::runtime_owner::run(async { work() })
            .map_err(|error| eframe::Error::AppCreation(Box::new(error)))?
    }
    #[cfg(not(target_os = "linux"))]
    work()
}

#[cfg(all(test, target_os = "linux"))]
#[path = "lifetime/tests.rs"]
mod tests;
