#[cfg(target_os = "windows")]
use winres;

#[cfg(target_os = "windows")]
fn require_admin() {
    use std::io::Write;
    // only build the resource for release builds
    // as calling rc.exe might be slow
    if std::env::var("PROFILE").unwrap() == "release" {
        let manifest = r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
<trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
        <requestedPrivileges>
            <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
        </requestedPrivileges>
    </security>
</trustInfo>
</assembly>
"#;

        let mut res = winres::WindowsResource::new();
        // res.set_icon("resources\\ico\\fiscalidade_server.ico")
        res.set_manifest(manifest);

        match res.compile() {
            Err(error) => {
                write!(std::io::stderr(), "{}", error).unwrap();
                std::process::exit(1);
            }
            Ok(_) => {}
        }
    }

}

fn main() {
    #[cfg(target_os = "windows")]
    require_admin();

    slint_build::compile("ui/main-ui.slint").unwrap();
}
