//! Only for isolated fake commands whose termination settles every local effect.
//! Never use this port for real clients that delegate work to another service.
use super::*;
pub(crate) struct AcknowledgedCommands;
impl CommandPort for AcknowledgedCommands {
    fn output<'a>(
        &'a self,
        program: &'a str,
        args: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = io::Result<Output>> + Send + 'a>> {
        SystemCommands.output(program, args)
    }
    fn mutate<'a>(
        &'a self,
        program: &'a str,
        args: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = io::Result<Output>> + Send + 'a>> {
        SystemCommands.output(program, args)
    }
}
