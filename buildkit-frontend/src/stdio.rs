use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper_util::rt::TokioIo;
use tokio::io::{stdin, stdout, AsyncRead, AsyncWrite, ReadBuf, Stdin, Stdout};
use tonic::transport::server::Connected;
use tonic::transport::Uri;

pub struct StdioSocket {
    reader: Stdin,
    writer: Stdout,
}

pub async fn stdio_connector(_: Uri) -> io::Result<TokioIo<StdioSocket>> {
    StdioSocket::try_new().map(TokioIo::new)
}

impl StdioSocket {
    pub fn try_new() -> io::Result<Self> {
        Ok(StdioSocket {
            reader: stdin(),
            writer: stdout(),
        })
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

impl Connected for StdioSocket {
    type ConnectInfo = ();

    fn connect_info(&self) -> Self::ConnectInfo {}
}
