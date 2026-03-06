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

    println!("cargo:rerun-if-changed=./localization/zh_CN/LC_MESSAGES/cfg-ip.po");

    let config =
        slint_build::CompilerConfiguration::new().with_bundled_translations("./localization");
    slint_build::compile_with_config("ui/main-ui.slint", config).unwrap();
    // slint_build::compile("ui/main-ui.slint").unwrap();
}
