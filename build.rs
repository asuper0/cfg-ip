#[cfg(target_os = "windows")]
fn require_admin() {
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
        res.set_manifest(manifest);

        if let Err(error) = res.compile() {
            eprint!("{}", error);
            std::process::exit(1);
        }
    }
}

fn main() {
    #[cfg(target_os = "windows")]
    require_admin();

    slint_build::compile("ui/main-ui.slint").unwrap();
}
