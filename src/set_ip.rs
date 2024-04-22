use encoding::{all::GB18030, DecoderTrap, Encoding};
use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{anyhow, Result};
use net_adapters::adapter::Address;
use std::net::IpAddr;

fn format_ip_address(ip: &IpAddr) -> String {
    match ip {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip6) => ip6.to_string(),
    }
}

pub fn set_dynamic_ip(nic_name: &str) -> Result<String> {
    // netsh interface ip set address name="WLAN" source=dhcp
    // netsh interface ip set dns name="WLAN" source=dhcp
    let cmd_set_dynamic = [
        "netsh.exe",
        "interface",
        "ip",
        "set",
        "address",
        &format!("name=\"{}\"", nic_name),
        "source=dhcp",
    ]
    .join(" ");

    let cmd_set_dns = [
        "netsh.exe",
        "interface",
        "ip",
        "set",
        "dns",
        &format!("name=\"{}\"", nic_name),
        "source=dhcp",
    ]
    .join(" ");

    shell_batch(vec![cmd_set_dynamic, cmd_set_dns])
}

pub fn set_static_ip(
    nic_name: &str,
    address: &[Address],
    gateway: &[IpAddr],
    dns: &[IpAddr],
) -> Result<String> {
    // Netsh interface IP set address "WLAN" Static 10.8.4.159 255.255.255.0 10.8.4.1
    // netsh interface ipv4 add address name="WLAN" addr=192.168.5.16 mask=255.255.255.0

    // Netsh interface IP set dns "WLAN" static 222.246.129.81 primary
    // Netsh interface IP add dns "WLAN" 114.114.114.114
    // Netsh interface IP add dns "WLAN" 58.20.127.238

    assert!(!address.is_empty());
    let name_field = format!("name=\"{}\"", nic_name);

    let cmd_set_static = [
        "netsh.exe",
        "interface",
        "ip",
        "set",
        "address",
        &name_field,
        "static",
        &format_ip_address(&address[0].ip),
        &format_ip_address(&address[0].netmask),
    ]
    .join(" ");

    let mut cmd_set_more_ip = Vec::with_capacity(address.len() - 1);
    for more_ip in address.iter().skip(1) {
        let cmd = [
            "netsh.exe",
            "interface",
            "ip",
            "add",
            "address",
            &name_field,
            &format_ip_address(&more_ip.ip),
            &format_ip_address(&more_ip.netmask),
        ]
        .join(" ");

        cmd_set_more_ip.push(cmd);
    }

    let mut cmd_set_gateway = Vec::with_capacity(gateway.len());
    for gateway in gateway {
        // Netsh interface IP add gateway "WLAN" 114.114.114.114
        let cmd = [
            "netsh.exe",
            "interface",
            "ip",
            "add",
            "address",
            &name_field,
            &format!("gateway={}", format_ip_address(gateway)),
            "gwmetric=0",
        ]
        .join(" ");

        cmd_set_gateway.push(cmd);
    }

    let mut cmd_set_dns = Vec::with_capacity(dns.len());
    for dns in dns {
        // Netsh interface IP add dns "WLAN" 114.114.114.114
        let cmd = [
            "netsh.exe",
            "interface",
            "ip",
            "add",
            "dns",
            &name_field,
            &format_ip_address(dns),
        ]
        .join(" ");

        cmd_set_dns.push(cmd);
    }
    let mut commands = Vec::new();
    commands.push(cmd_set_static);
    commands.extend(cmd_set_more_ip);
    commands.extend(cmd_set_gateway);
    commands.extend(cmd_set_dns);

    shell_batch(commands)
}

fn shell_batch(commands: Vec<String>) -> Result<String> {
    let mut child = Command::new("cmd.exe")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    for cmd in commands {
        let s = GB18030
            .encode(&cmd, encoding::EncoderTrap::Strict)
            .map_err(|err| anyhow!(err.to_string()))?;
        stdin.write_all(&s[..])?;
        if !cmd.ends_with("\n") {
            stdin.write_all("\n".as_bytes())?;
        }
    }
    stdin.write_all("pause\n".as_bytes())?;
    stdin.write_all("exit\n".as_bytes())?;

    let _output = child.wait_with_output()?;
    let msg = GB18030
        .decode(&_output.stdout, DecoderTrap::Strict)
        .expect("format std output failed");

    Ok(msg)
}
