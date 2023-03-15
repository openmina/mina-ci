use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    sync::{Arc, RwLock},
};

use tracing::{info, warn};

use crate::{
    executor::state::{AggregatorState, AggregatorStateInner},
    AggregatorResult,
};

use super::{AggregatorStorage, BuildStorage, BuildStorageDump, LockedBTreeMap};

#[derive(Debug, Clone, Default)]
pub struct RemoteStorage {
    url: String,
    user: String,
    password: String,
    file_path: String,
}

impl RemoteStorage {
    pub fn new(url: &str, user: &str, password: &str, file_path: &str) -> Self {
        Self {
            url: url.to_string(),
            user: user.to_string(),
            password: password.to_string(),
            file_path: file_path.to_ascii_lowercase(),
        }
    }

    fn download_storage(&self) -> AggregatorResult<String> {
        let conn = TcpStream::connect(&self.url)?;
        let mut session = ssh2::Session::new()?;
        session.set_tcp_stream(conn);
        session.set_blocking(true);
        session.handshake()?;
        session.userauth_password(&self.user, &self.password)?;

        let (mut channel, _) = session.scp_recv(Path::new(&self.file_path))?;
        // remote.close().expect("Failde to close channel");
        let mut data = String::new();
        channel.read_to_string(&mut data)?;

        channel.send_eof()?;
        channel.wait_eof()?;
        channel.close()?;
        channel.wait_close()?;

        Ok(data)
    }

    fn upload_storage(&self, data: &str) -> AggregatorResult<()> {
        let conn = TcpStream::connect(&self.url)?;
        let mut session = ssh2::Session::new()?;
        session.set_tcp_stream(conn);
        session.set_blocking(true);
        session.handshake()?;
        session.userauth_password(&self.user, &self.password)?;

        let raw_string_bytes = data.as_bytes();
        let mut channel = session.scp_send(
            Path::new(&self.file_path),
            0o644,
            raw_string_bytes.len() as u64,
            None,
        )?;
        println!("len: {}", raw_string_bytes.len());
        channel.write_all(raw_string_bytes)?;

        channel.send_eof()?;
        channel.wait_eof()?;
        channel.close()?;
        channel.wait_close()?;
        drop(channel);
        Ok(())
    }

    pub fn save_storage(&self, storage: AggregatorStorage) {
        let raw: BTreeMap<usize, BuildStorageDump> = storage
            .to_btreemap()
            .expect("Cannot convert to btreemap")
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();
        match serde_json::to_string(&raw) {
            Ok(deserialized) => {
                if let Err(e) = self.upload_storage(&deserialized) {
                    warn!("Failed to upload storage: {e}");
                } else {
                    info!("Storage uploaded to remote successfully");
                }
            }
            Err(e) => warn!("Failed to deserialize storage while sending to remote: {e}"),
        }
    }

    pub fn load_storage(&self) -> (AggregatorState, AggregatorStorage) {
        match self.download_storage() {
            Ok(data) => {
                match serde_json::from_str::<BTreeMap<usize, BuildStorageDump>>(&data) {
                    Ok(storage) => {
                        // println!("{:#?}", storage);
                        let storage = storage
                            .iter()
                            .map(|(k, v)| (*k, v.clone().into()))
                            .collect::<BTreeMap<usize, BuildStorage>>();
                        // println!("{:#?}", storage);
                        let state_inner = AggregatorStateInner {
                            build_number: storage
                                .last_key_value()
                                .map(|(k, _)| *k)
                                .unwrap_or_default(),
                            ..Default::default()
                        };
                        let state = Arc::new(RwLock::new(state_inner));
                        info!("Remote storage loaded successfully");
                        (state, LockedBTreeMap::new(storage))
                    }
                    Err(e) => {
                        warn!("{e}");
                        warn!("Failed to deserialize storage json, creating new storage");
                        (AggregatorState::default(), LockedBTreeMap::default())
                    }
                }
            }
            Err(e) => {
                warn!("Failed to download storage from remote storage: {}", e);
                (AggregatorState::default(), LockedBTreeMap::default())
            }
        }
    }
}
