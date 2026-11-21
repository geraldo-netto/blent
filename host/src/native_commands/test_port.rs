//! Only for isolated fake commands whose termination settles every local effect.
//! Never use this port for real clients that delegate work to another service.
use super::*;
pub(crate) struct AcknowledgedCommands;
impl CommandPort for AcknowledgedCommands {
    fn set_property(
        &self,
        request: Property,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        Box::pin(async move {
            let backend = crate::kwin::backend()
                .await
                .ok_or(io::ErrorKind::NotFound)?;
            let (program, arguments) = property_command(backend, request);
            let args = arguments.iter().map(String::as_str).collect::<Vec<_>>();
            let output = SystemCommands.output(program, &args).await?;
            if output.status.success() {
                Ok(())
            } else {
                Err(io::ErrorKind::Other.into())
            }
        })
    }
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

fn property_command(
    backend: crate::kwin::Backend,
    request: Property,
) -> (&'static str, Vec<String>) {
    let (signature, value) = match request.value {
        property::Scalar::Text(value) => ("s", value),
        property::Scalar::Integer(value) => ("i", value.to_string()),
    };
    let arguments = match backend {
        crate::kwin::Backend::Busctl => vec![
            "--user".into(),
            "--timeout=infinity".into(),
            "set-property".into(),
            "org.kde.KWin".into(),
            request.path,
            request.interface,
            request.name,
            signature.into(),
            value,
        ],
        crate::kwin::Backend::Qdbus(_) => vec![
            "--literal".into(),
            "org.kde.KWin".into(),
            request.path,
            "org.freedesktop.DBus.Properties.Set".into(),
            request.interface,
            request.name,
            value,
        ],
    };
    (backend.name(), arguments)
}

pub(crate) struct ScriptedX11<'a> {
    pub commands: &'a dyn CommandPort,
    pub program: &'a str,
}
impl AcknowledgedCommands {
    pub(crate) fn x11<'a>(&'a self, program: &'a str) -> ScriptedX11<'a> {
        ScriptedX11 {
            commands: self,
            program,
        }
    }
}
impl CommandPort for ScriptedX11<'_> {
    fn output<'a>(
        &'a self,
        program: &'a str,
        args: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = io::Result<Output>> + Send + 'a>> {
        self.commands.output(program, args)
    }
    fn map_output<'a>(
        &'a self,
        device: &'a str,
        output: &'a str,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let result = self
                .commands
                .mutate(self.program, &["map-to-output", device, output])
                .await?;
            if result.status.success() {
                Ok(())
            } else {
                Err(io::ErrorKind::Other.into())
            }
        })
    }
    fn set_property(
        &self,
        request: Property,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        self.commands.set_property(request)
    }
}
