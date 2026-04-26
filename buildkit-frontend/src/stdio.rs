use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper_util::client::legacy::connect::Connected;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tonic::transport::Uri;

pub struct StdioSocket {
    reader: tokio::io::Stdin,
    writer: tokio::io::Stdout,
}

pub async fn stdio_connector(_: Uri) -> io::Result<StdioSocket> {
    Ok(StdioSocket {
        reader: tokio::io::stdin(),
        writer: tokio::io::stdout(),
    })
}

impl hyper_util::client::legacy::connect::Connection for StdioSocket {
    fn connected(&self) -> Connected {
        Connected::new()
    }
}

impl AsyncRead for StdioSocket {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}

impl AsyncWrite for StdioSocket {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.writer).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.writer).poll_shutdown(cx)
    }
}
