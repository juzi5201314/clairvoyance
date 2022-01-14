use std::mem::size_of;
use std::path::Path;

use integer_encoding::{VarIntAsyncReader, VarIntAsyncWriter};
use once_cell::sync::Lazy;
use smallvec::SmallVec;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufStream};

use crate::data::{Data, NetIo};

pub static BINCODE_CONFIG: Lazy<bincode::config::Configuration> =
    Lazy::new(bincode::config::Configuration::standard);

// 存储数据到中间文件的流
pub struct StoreStream {
    file: BufStream<File>,
}

impl StoreStream {
    // 创建并打开一个新的中间文件和存储流
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
            file: BufStream::new(file),
        })
    }

    // 打开一个已经存在的中间文件和存储流
    pub async fn open<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = OpenOptions::new().read(true).open(path).await?;
        Ok(StoreStream {
            file: BufStream::new(file),
        })
    }

    pub async fn write(&mut self, data: &Data) -> anyhow::Result<()> {
        let mut buf = SmallVec::<
            [u8; size_of::<Data>()
                + if cfg!(target_os = "linux") {
                    size_of::<NetIo>() * 3
                } else {
                    0
                }],
        >::new_const();
        bincode::encode_into_std_write(data, &mut buf, *BINCODE_CONFIG)?;

        // Data的长度
        self.file.write_varint_async(buf.len()).await?;
        self.file.write_all(&buf).await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> anyhow::Result<()> {
        self.file.flush().await?;
        self.file.get_mut().sync_data().await?;
        Ok(())
    }

    pub async fn read(&mut self) -> anyhow::Result<Option<Data>> {
        let size = match self.file.read_varint_async().await {
            Ok(n) => n,
            Err(err) => {
                if matches!(err.kind(), std::io::ErrorKind::UnexpectedEof) {
                    return Ok(None);
                } else {
                    return Err(err.into());
                }
            }
        };

        let mut buf = SmallVec::<
            [u8; size_of::<Data>()
                + if cfg!(target_os = "linux") {
                    size_of::<NetIo>() * 3
                } else {
                    0
                }],
        >::new_const();
        buf.reserve_exact(size);
        buf.resize(size, 0);

        self.file.read_exact(&mut buf).await?;

        Ok(Some(bincode::decode_from_slice(&buf, *BINCODE_CONFIG)?.0))
    }
}
