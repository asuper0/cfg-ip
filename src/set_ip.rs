use encoding::all::GB18030;
use encoding::{DecoderTrap, Encoding};

use std::process::{Command, Stdio};

use net_adapters::adapter::{Address, Nic};
use anyhow::Result;
use std::net::IpAddr;
use std::sync::mpsc;

pub fn set_dynamic_ip(
    nic: &Nic,
    output_tx: mpsc::Sender<(usize, &'static str, String)>,
) -> Result<()> {
    // netsh interface ip set address name="WLAN" source=dhcp
    // netsh interface ip set dns name="WLAN" source=dhcp
    let mut cmd_set_dynamic = Command::new("netsh.exe");
    cmd_set_dynamic.args([
        "interface",
        "ip",
        "set",
        "address",
        &format!("name=\"{}\"", nic.name()),
        "source=dhcp",
    ]);

    let mut cmd_set_dns = Command::new("netsh.exe");
    cmd_set_dns.args([
        "interface",
        "ip",
        "set",
        "dns",
        &format!("name=\"{}\"", nic.name()),
        "source=dhcp",
    ]);

    let commands = vec![("ip", cmd_set_dynamic), ("dns", cmd_set_dns)];
    shell_batch(commands, output_tx)?;

    Ok(())
}

pub fn set_static_ip(
    nic: &Nic,
    address: &[Address],
    gateway: &[IpAddr],
    dns: &[IpAddr],
    output_tx: mpsc::Sender<(usize, &'static str, String)>,
) -> Result<()> {
    // Netsh interface IP set address "WLAN" Static 10.8.4.159 255.255.255.0 10.8.4.1
    // netsh interface ipv4 add address name="WLAN" addr=192.168.5.16 mask=255.255.255.0

    // Netsh interface IP set dns "WLAN" static 222.246.129.81 primary
    // Netsh interface IP add dns "WLAN" 114.114.114.114
    // Netsh interface IP add dns "WLAN" 58.20.127.238

    assert!(!address.is_empty());

    let name_field = format!("name=\"{}\"", nic.name());
    let mut cmd_set_static = Command::new("netsh.exe");
    let arg1 = vec![
        "interface".to_string(),
        "ip".to_string(),
        "set".to_string(),
        "address".to_string(),
        name_field.clone(),
        "static".to_string(),
        format_ip_address(&address[0].ip),
        format_ip_address(&address[0].netmask),
    ];
    // if !gateway.is_empty() {
    //     arg1.push(format_ip_address(&gateway[0]));
    // }
    cmd_set_static.args(arg1);

    let mut cmd_set_more_ip = Vec::with_capacity(address.len() - 1);
    for more_ip in address.iter().skip(1) {
        let mut cmd = Command::new("netsh.exe");

        cmd.args([
            "interface",
            "ip",
            "add",
            "address",
            &name_field,
            &format_ip_address(&more_ip.ip),
            &format_ip_address(&more_ip.netmask),
        ]);

        cmd_set_more_ip.push(("ip", cmd));
    }

    let mut cmd_set_gateway = Vec::with_capacity(gateway.len());
    for  gateway in gateway {
        let mut cmd = Command::new("netsh.exe");

        // Netsh interface IP add gateway "WLAN" 114.114.114.114
        cmd.args([
            "interface",
            "ip",
            "add",
            "address",
            &name_field,
            &format!("gateway={}", format_ip_address(&gateway)),
            "gwmetric=0",
        ]);

        cmd_set_gateway.push(("gateway", cmd));
    }

    let mut cmd_set_dns = Vec::with_capacity(dns.len());
    for  dns in dns{
        let mut cmd = Command::new("netsh.exe");

        // Netsh interface IP add dns "WLAN" 114.114.114.114
        cmd.args([
            "interface",
            "ip",
            "add",
            "dns",
            &name_field,
            &format_ip_address(&dns),
        ]);

        cmd_set_dns.push(("dns", cmd));
    }
    let mut commands = Vec::new();
    commands.push(("ip", cmd_set_static));
    commands.extend(cmd_set_more_ip);
    commands.extend(cmd_set_gateway);
    commands.extend(cmd_set_dns);

    shell_batch(commands, output_tx)?;

    Ok(())
}

fn format_ip_address(ip: &IpAddr) -> String {
    match ip {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip6) => ip6.to_string(),
    }
}

fn shell_batch(
    commands: Vec<(&'static str, Command)>,
    output_tx: mpsc::Sender<(usize, &'static str, String)>,
) -> Result<()> {
    std::thread::spawn(move || {
        for (i, (desc, mut cmd)) in commands.into_iter().enumerate() {
            cmd.stdout(Stdio::piped());
            match cmd.output() {
                Ok(output) => {
                    let msg = GB18030
                        .decode(&output.stdout, DecoderTrap::Strict)
                        .expect("format std output failed");
                    // let msg =                 String::from_utf8(output.stdout).expect("format std output failed");
                    // let msg = OsStr::
                    if output_tx.send((i, desc, msg)).is_err() {
                        break;
                    }
                }
                Err(err) => {
                    let _ = output_tx.send((i, desc, err.to_string()));
                    break;
                    // return Err(anyhow::Error::new(err));
                }
            };
        }
    });

    Ok(())
}
