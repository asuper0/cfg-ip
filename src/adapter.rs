pub use from_heim_net::*;
use crate::ui::generated_code::NetInterfaceItem;

pub fn get_adapters() -> Vec<Nic> {
    let available_adapters: Vec<_> = nic()
        .unwrap()
        .into_iter()
        .filter_map(|adapter| {
            // if adapter.is_up()
            //     && !adapter.is_loopback()
            match !adapter.is_loopback() {
                true => Some(adapter),
                false => None,
            }
        })
        .collect();

    available_adapters
}

/// Copy from heim_net
mod from_heim_net {
    use winapi::shared::ifdef::IfOperStatusUp;
    use winapi::shared::minwindef::ULONG;
    use winapi::shared::ntdef::NULL;
    use winapi::shared::winerror::{ERROR_BUFFER_OVERFLOW, NO_ERROR};
    use winapi::shared::ws2def::AF_UNSPEC;
    use winapi::shared::ws2def::SOCKADDR_IN;
    use winapi::shared::ws2def::SOCKET_ADDRESS;
    use winapi::shared::ws2def::{AF_INET, AF_INET6};
    use winapi::shared::ws2ipdef::SOCKADDR_IN6;
    use winapi::um::iphlpapi::GetAdaptersAddresses;
    use winapi::um::iptypes::IP_ADAPTER_ADDRESSES;
    use winapi::um::iptypes::PIP_ADAPTER_ADDRESSES;
    use winapi::um::iptypes::PIP_ADAPTER_DNS_SERVER_ADDRESS;
    use winapi::um::iptypes::PIP_ADAPTER_GATEWAY_ADDRESS;
    use winapi::um::iptypes::{GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_INCLUDE_PREFIX};

    use anyhow::Result;
    use serde_derive::{Deserialize, Serialize};
    use std::hash::Hash;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
    pub struct Address {
        pub ip: IpAddr,
        pub netmask: IpAddr,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Eq)]
    pub struct Nic {
        index: u32,
        guid: String,
        friendly_name: String,
        is_up: bool,
        address: Vec<Address>,
        gateway: Vec<IpAddr>,
        dns: Vec<IpAddr>,
    }

    pub fn nic() -> Result<Vec<Nic>> {
        let mut results = Vec::new();

        // Step 1 - get the size of the routing infos
        let family = AF_INET; // retrieve IPv4
        let flags: ULONG = GAA_FLAG_INCLUDE_PREFIX | GAA_FLAG_INCLUDE_GATEWAYS;
        let mut empty_list = IP_ADAPTER_ADDRESSES::default();
        let mut data_size: ULONG = 0;
        let res = unsafe {
            GetAdaptersAddresses(family as _, flags, NULL, &mut empty_list, &mut data_size)
        };
        if res != ERROR_BUFFER_OVERFLOW {
            // Unable to get the size of routing infos
            let e = std::io::Error::from_raw_os_error(res as _);
            return Err(anyhow::Error::from(e));
        }

        // Step 2 - get the interfaces infos
        let mut not_init_buffer = Vec::with_capacity(data_size as usize);

        let res = unsafe {
            GetAdaptersAddresses(
                family as _,
                flags,
                NULL,
                not_init_buffer.as_mut_ptr() as _,
                &mut data_size,
            )
        };
        if res != NO_ERROR {
            // Unable to get the routing infos
            let e = std::io::Error::from_raw_os_error(res as _);
            return Err(anyhow::Error::from(e));
        }

        unsafe {
            not_init_buffer.set_len(data_size as usize);
        }

        // Step 3 - walk through the list and populate our interfaces
        let mut cur_iface = unsafe {
            let p = not_init_buffer.as_ptr() as PIP_ADAPTER_ADDRESSES;
            if p.is_null() {
                // Unable to list interfaces
                let e = std::io::Error::from_raw_os_error(res as _);
                return Err(anyhow::Error::from(e));
            }
            *p
        };

        loop {
            let iface_index;
            let iface_guid_cstr;
            let iface_fname_ucstr;
            let is_up;
            let mut cur_address;

            unsafe {
                iface_index = cur_iface.u.s().IfIndex;
                iface_guid_cstr = std::ffi::CStr::from_ptr(cur_iface.AdapterName);
                // iface_fname_ucstr = UCStr::from_ptr_str(cur_iface.FriendlyName);
                iface_fname_ucstr =
                    widestring::ucstring::WideCString::from_ptr_str(cur_iface.FriendlyName);
                cur_address = *(cur_iface.FirstUnicastAddress);
                is_up = cur_iface.OperStatus == IfOperStatusUp;
            }
            let iface_guid = iface_guid_cstr
                .to_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|_| "".into());
            let iface_friendly_name = iface_fname_ucstr.to_string_lossy();

            let mut address = Vec::with_capacity(2);

            // Walk through every IP address of this interface
            loop {
                let this_socket_address = cur_address.Address;
                let this_netmask_length = cur_address.OnLinkPrefixLength;
                let this_sa_family = unsafe { (*this_socket_address.lpSockaddr).sa_family };

                let (this_address, this_netmask) = match this_sa_family as i32 {
                    AF_INET => (
                        sockaddr_to_ipv4(this_socket_address),
                        Some(ipv4_netmask_address_from(this_netmask_length)),
                    ),
                    AF_INET6 => (
                        sockaddr_to_ipv6(this_socket_address),
                        Some(ipv6_netmask_address_from(this_netmask_length)),
                    ),
                    _ => (None, None),
                };

                if let (Some(ip), Some(netmask)) = (this_address, this_netmask) {
                    address.push(Address { ip, netmask });
                }

                let next_address = cur_address.Next;
                if next_address.is_null() {
                    break;
                }
                cur_address = unsafe { *next_address };
            }

            let dns = get_dns(cur_iface.FirstDnsServerAddress);
            let gateway = get_gateway(cur_iface.FirstGatewayAddress);
            let base_nic = Nic {
                index: iface_index,
                friendly_name: iface_friendly_name,
                guid: iface_guid,
                is_up,
                address,
                dns,
                gateway,
            };
            results.push(base_nic);

            let next_item = cur_iface.Next;
            if next_item.is_null() {
                break;
            }
            cur_iface = unsafe { *next_item };
        }

        Ok(results)
    }

    fn get_dns(p: PIP_ADAPTER_DNS_SERVER_ADDRESS) -> Vec<IpAddr> {
        let mut dns_list = Vec::with_capacity(3);
        if p.is_null() {
            return dns_list;
        }

        let mut cur_address = unsafe { *p };
        loop {
            let this_socket_address = cur_address.Address;
            let this_sa_family = unsafe { (*this_socket_address.lpSockaddr).sa_family };

            let this_address = match this_sa_family as i32 {
                AF_INET => sockaddr_to_ipv4(this_socket_address),
                AF_INET6 => sockaddr_to_ipv6(this_socket_address),
                _ => None,
            };

            if let Some(address) = this_address {
                dns_list.push(address);
            }

            let next_address = cur_address.Next;
            if next_address.is_null() {
                break;
            }
            cur_address = unsafe { *next_address };
        }

        dns_list
    }

    fn get_gateway(p: PIP_ADAPTER_GATEWAY_ADDRESS) -> Vec<IpAddr> {
        let mut gateway_list = Vec::with_capacity(1);
        if p.is_null() {
            return gateway_list;
        }

        let mut cur_address = unsafe { *p };
        loop {
            let this_socket_address = cur_address.Address;
            let this_sa_family = unsafe { (*this_socket_address.lpSockaddr).sa_family };

            let this_address = match this_sa_family as i32 {
                AF_INET => sockaddr_to_ipv4(this_socket_address),
                AF_INET6 => sockaddr_to_ipv6(this_socket_address),
                _ => None,
            };

            if let Some(address) = this_address {
                gateway_list.push(address);
            }

            let next_address = cur_address.Next;
            if next_address.is_null() {
                break;
            }
            cur_address = unsafe { *next_address };
        }

        gateway_list
    }

    impl Nic {
        pub fn name(&self) -> &str {
            &self.friendly_name
        }

        pub fn index(&self) -> Option<u32> {
            Some(self.index)
        }

        pub fn guid(&self) -> &str {
            &self.guid
        }

        pub fn address(&self) -> &[Address] {
            &self.address
        }

        pub fn dns(&self) -> &[IpAddr] {
            &self.dns
        }

        pub fn gateway(&self) -> &[IpAddr] {
            &self.gateway
        }

        pub fn destination(&self) -> Option<std::net::IpAddr> {
            // TODO: we could implement something one day
            None
        }

        pub fn is_up(&self) -> bool {
            self.is_up
        }

        pub fn is_running(&self) -> bool {
            // TODO: not sure how to tell on Windows
            true
        }

        pub fn is_loopback(&self) -> bool {
            self.address.iter().any(|ada| ada.is_loopback())
        }

        pub fn is_multicast(&self) -> bool {
            self.address.iter().any(|ada| ada.is_multicast())
        }

        pub fn update(&mut self) -> Result<()> {
            // Step 1 - get the size of the routing infos
            let family = AF_UNSPEC; // retrieve both IPv4 and IPv6 interfaces
            let flags: ULONG = GAA_FLAG_INCLUDE_PREFIX | GAA_FLAG_INCLUDE_GATEWAYS;
            let mut empty_list = IP_ADAPTER_ADDRESSES::default();
            let mut data_size: ULONG = 0;
            let res = unsafe {
                GetAdaptersAddresses(family as _, flags, NULL, &mut empty_list, &mut data_size)
            };
            if res != ERROR_BUFFER_OVERFLOW {
                // Unable to get the size of routing infos
                let e = std::io::Error::from_raw_os_error(res as _);
                return Err(anyhow::Error::from(e));
            }

            // Step 2 - get the interfaces infos
            let mut not_init_buffer = Vec::with_capacity(data_size as usize);
            let res = unsafe {
                GetAdaptersAddresses(
                    family as _,
                    flags,
                    NULL,
                    not_init_buffer.as_mut_ptr() as _,
                    &mut data_size,
                )
            };
            if res != NO_ERROR {
                // Unable to get the routing infos
                let e = std::io::Error::from_raw_os_error(res as _);
                return Err(anyhow::Error::from(e));
            }

            unsafe {
                not_init_buffer.set_len(data_size as usize);
            }
            // Step 3 - walk through the list and populate our interfaces
            let mut cur_iface = unsafe {
                let p = not_init_buffer.as_ptr() as PIP_ADAPTER_ADDRESSES;
                if p.is_null() {
                    // Unable to list interfaces
                    let e = std::io::Error::from_raw_os_error(res as _);
                    return Err(anyhow::Error::from(e));
                }
                *p
            };

            let mut match_iface = None;
            loop {
                let iface_index;
                let iface_guid_cstr;

                unsafe {
                    iface_index = cur_iface.u.s().IfIndex;
                    iface_guid_cstr = std::ffi::CStr::from_ptr(cur_iface.AdapterName);
                }
                let iface_guid = iface_guid_cstr
                    .to_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| "".into());

                if iface_guid == self.guid && iface_index == self.index {
                    match_iface = Some(cur_iface);
                    break;
                }

                let next_item = cur_iface.Next;
                if next_item.is_null() {
                    break;
                }
                cur_iface = unsafe { *next_item };
            }

            if let Some(cur_iface) = match_iface {
                let mut cur_address = unsafe { *(cur_iface.FirstUnicastAddress) };
                let is_up = cur_iface.OperStatus == IfOperStatusUp;

                let mut address = Vec::with_capacity(2);

                // Walk through every IP address of this interface
                loop {
                    let this_socket_address = cur_address.Address;
                    let this_netmask_length = cur_address.OnLinkPrefixLength;
                    let this_sa_family = unsafe { (*this_socket_address.lpSockaddr).sa_family };

                    let (this_address, this_netmask) = match this_sa_family as i32 {
                        AF_INET => (
                            sockaddr_to_ipv4(this_socket_address),
                            Some(ipv4_netmask_address_from(this_netmask_length)),
                        ),
                        AF_INET6 => (
                            sockaddr_to_ipv6(this_socket_address),
                            Some(ipv6_netmask_address_from(this_netmask_length)),
                        ),
                        _ => (None, None),
                    };

                    if let (Some(ip), Some(netmask)) = (this_address, this_netmask) {
                        address.push(Address { ip, netmask });
                    }

                    let next_address = cur_address.Next;
                    if next_address.is_null() {
                        break;
                    }
                    cur_address = unsafe { *next_address };
                }

                let dns = get_dns(cur_iface.FirstDnsServerAddress);
                let gateway = get_gateway(cur_iface.FirstGatewayAddress);

                self.is_up = is_up;
                self.address = address;
                self.gateway = gateway;
                self.dns = dns;
            }
            Ok(())
        }
    }

    impl Nic {
        pub fn convert_for_ui(&self) -> super::NetInterfaceItem {
            // super::NetInterfaceItem { address: (), dns: (), gateway: (), guid: (), index: (), is_up: (), name: () }
            todo!()
        }
    }

    impl PartialEq for Nic {
        fn eq(&self, other: &Self) -> bool {
            self.guid == other.guid
                && self.friendly_name == other.friendly_name
                && vec_compare(&self.address, &other.address)
                && vec_compare(&self.gateway, &other.gateway)
                && vec_compare(&self.dns, &other.dns)
        }
    }

    impl Hash for Nic {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.guid.hash(state);
            self.friendly_name.hash(state);
            self.address.hash(state);
            self.gateway.hash(state);
            self.dns.hash(state);
        }
    }

    fn vec_compare<T: PartialEq>(va: &[T], vb: &[T]) -> bool {
        if va.len() != vb.len() {
            return false;
        }

        // 开始实现了排序，后面考虑到item顺序也是区分不同配置的一项，所以不做排序了
        // let mut va = Vec::from_iter(va);
        // let mut vb = Vec::from_iter(vb);
        // va.sort();
        // vb.sort();

        va.iter().zip(vb.iter()).all(|(a, b)| *a ==*b)
    }

    impl Address {
        pub fn is_loopback(&self) -> bool {
            match self.ip {
                IpAddr::V4(sa) => sa.is_loopback(),
                IpAddr::V6(sa6) => sa6.is_loopback(),
            }
        }

        pub fn is_multicast(&self) -> bool {
            match self.ip {
                IpAddr::V4(sa) => sa.is_multicast(),
                IpAddr::V6(sa6) => sa6.is_multicast(),
            }
        }
    }

    fn sockaddr_to_ipv4(sa: SOCKET_ADDRESS) -> Option<IpAddr> {
        // Check this sockaddr can fit one short and a char[14]
        // (see https://docs.microsoft.com/en-us/windows/win32/winsock/sockaddr-2)
        // This should always happen though, this is guaranteed by winapi's interface
        if (sa.iSockaddrLength as usize) < std::mem::size_of::<SOCKADDR_IN>() {
            return None;
        }

        if sa.lpSockaddr.is_null() {
            return None;
        }
        let arr = unsafe { (*sa.lpSockaddr).sa_data };
        let ip4 = Ipv4Addr::new(arr[2] as _, arr[3] as _, arr[4] as _, arr[5] as _);
        // let port = (arr[0] as u16) + (arr[1] as u16) * 0x100;

        // Some(SocketAddr::V4(SocketAddrV4::new(ip4, port)))
        Some(IpAddr::V4(ip4))
    }

    fn sockaddr_to_ipv6(sa: SOCKET_ADDRESS) -> Option<IpAddr> {
        // Check this sockaddr can fit a SOCKADDR_IN6 (two shorts, two longs, and a 16-byte struct)
        // (see https://docs.microsoft.com/en-us/windows/win32/winsock/sockaddr-2)
        if (sa.iSockaddrLength as usize) < std::mem::size_of::<SOCKADDR_IN6>() {
            return None;
        }

        let p_sa6 = sa.lpSockaddr as *const SOCKADDR_IN6;
        if p_sa6.is_null() {
            return None;
        }
        let sa6 = unsafe { *p_sa6 };

        let ip6_data = unsafe { sa6.sin6_addr.u.Byte() };
        let ip6 = Ipv6Addr::from(*ip6_data);
        // let port = sa6.sin6_port;
        // let flow_info = sa6.sin6_flowinfo;
        // let scope_id = unsafe { *sa6.u.sin6_scope_id() };

        // Some(SocketAddr::V6(SocketAddrV6::new(
        //     ip6, port, flow_info, scope_id,
        // )))
        Some(IpAddr::V6(ip6))
    }

    /// Generate an IPv4 netmask from a prefix length (Rust equivalent of ConvertLengthToIpv4Mask())
    fn ipv4_netmask_from(length: u8) -> Ipv4Addr {
        let mask = match length {
        len if len <= 32 => u32::max_value().checked_shl(32 - len as u32).unwrap_or(0),
        _ /* invalid value */ => u32::max_value(),
    };
        Ipv4Addr::from(mask)
    }

    /// Generate an IPv6 netmask from a prefix length
    fn ipv6_netmask_from(length: u8) -> Ipv6Addr {
        let mask = match length {
        len if len <= 128 => u128::max_value().checked_shl(128 - len as u32).unwrap_or(0),
        _ /* invalid value */ => u128::max_value(),
    };
        Ipv6Addr::from(mask)
    }

    fn ipv4_netmask_address_from(length: u8) -> IpAddr {
        IpAddr::V4(ipv4_netmask_from(length))
    }
    fn ipv6_netmask_address_from(length: u8) -> IpAddr {
        IpAddr::V6(ipv6_netmask_from(length))
    }
}
