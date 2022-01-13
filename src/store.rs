use std::mem::size_of;
use std::path::Path;

use once_cell::sync::Lazy;
use smallvec::SmallVec;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::data::{Data, NetIo};

pub static BINCODE_CONFIG: Lazy<bincode::config::Configuration> =
    Lazy::new(bincode::config::Configuration::standard);

// 存储数据到中间文件的流
pub struct StoreStream {
    file: BufWriter<File>,
}

impl StoreStream {
    pub async fn create<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)
            .await?;
        Ok(StoreStream {
            file: BufWriter::new(file),
        })
    }

    pub async fn write(&mut self, data: &Data) -> anyhow::Result<()> {
        let mut buf = SmallVec::<[u8; size_of::<Data>() + size_of::<NetIo>() * 3]>::new_const();
        bincode::encode_into_std_write(data, &mut buf, *BINCODE_CONFIG)?;
        let mut encoder = async_compression::tokio::write::DeflateEncoder::with_quality(
            &mut self.file,
            async_compression::Level::Fastest,
        );
        encoder.write_all(&buf).await?;
        encoder.flush().await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> anyhow::Result<()> {
        self.file.flush().await?;
        self.file.get_mut().sync_data().await?;
        Ok(())
    }
}
