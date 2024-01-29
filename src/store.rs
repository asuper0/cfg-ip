use itertools::Itertools;
use net_adapters::adapter::Nic;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct IpConfigList {
    inner: HashMap<String, Vec<Nic>>,
}

#[allow(unused)]
impl IpConfigList {
    pub fn get_by_guid(&self, key: &str) -> Option<&Vec<Nic>> {
        self.inner.get(key)
    }

    pub fn contains(&self, nic: &Nic) -> bool {
        self.try_get(nic).is_some()
    }

    pub fn try_get(&self, nic: &Nic) -> Option<&Nic> {
        if let Some(v) = self.inner.get(nic.guid()) {
            v.iter().find(|item| *item == nic)
        } else {
            None
        }
    }

    pub fn try_get_mut(&mut self, nic: &Nic) -> Option<&mut Nic> {
        if let Some(v) = self.inner.get_mut(nic.guid()) {
            v.iter_mut().find(|item| *item == nic)
        } else {
            None
        }
    }

    pub fn remove_by_guid(&mut self, key: &str) -> Option<Vec<Nic>> {
        self.inner.remove(key)
    }

    pub fn remove(&mut self, nic: &Nic) -> Option<Nic> {
        if let Some(v) = self.inner.get_mut(nic.guid()) {
            if let Some((i, _)) = v.iter().enumerate().find(|(i, item)| *item == nic) {
                let removed_val = v.remove(i);
                if v.is_empty() {
                    self.inner.remove(nic.guid());
                }
                return Some(removed_val);
            }
        }

        None
    }

    pub fn remove_at(&mut self, index: i32) -> Option<Nic> {
        assert!(index >= 0);
        if let Some(nic) = self.inner.values().flatten().nth(index as usize) {
            let nic = nic.clone();
            self.remove(&nic)
        } else {
            None
        }
    }

    pub fn insert(&mut self, nic: Nic) -> bool {
        let entry = self.inner.entry(nic.guid().to_string());
        match entry {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if !entry.get().iter().any(|item| nic.eq(item)) {
                    entry.get_mut().push(nic);
                    true
                } else {
                    false
                }
            }
            std::collections::hash_map::Entry::Vacant(v) => {
                v.insert(vec![nic]);
                true
            }
        }
    }

    pub fn get_list(&self) -> Vec<Nic> {
        self.inner.values().flatten().cloned().collect_vec()
    }
}
