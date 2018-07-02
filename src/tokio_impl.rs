extern crate tokio_io;
extern crate tokio_proto;
extern crate tokio_core;
extern crate futures;

use super::*;
use self::tokio_io::{AsyncRead, AsyncWrite};
use self::futures::{Future, Poll, Async};


impl<S: AsyncRead + AsyncWrite> Future for ConnectAsync<S> {
    type Item = TlsStream<S, ClientSession>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

impl<S: AsyncRead + AsyncWrite> Future for AcceptAsync<S> {
    type Item = TlsStream<S, ServerSession>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

impl<S, C> Future for MidHandshake<S, C>
    where S: io::Read + io::Write, C: Session
{
    type Item = TlsStream<S, C>;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let stream = self.inner.as_mut().unwrap();
            if !stream.session.is_handshaking() { debug!("not handshake"); break };

            let (io, session) = stream.get_mut();

            match session.complete_io(io) {
                Ok(_) => (),
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(Async::NotReady),
                Err(e) => {
                    debug!("complete io failed: {:?}", e);
                    return Err(e)
                }
            }
        }

        Ok(Async::Ready(self.inner.take().unwrap()))
    }
}

impl<S, C> AsyncRead for TlsStream<S, C>
    where
        S: AsyncRead + AsyncWrite,
        C: Session
{}

impl<S, C> AsyncWrite for TlsStream<S, C>
    where
        S: AsyncRead + AsyncWrite,
        C: Session
{
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        if !self.is_shutdown {
            self.session.send_close_notify();
            self.is_shutdown = true;
        }

        match self.session.complete_io(&mut self.io) {
            Ok(_) => (),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(Async::NotReady),
            Err(e) => {
                debug!("stream complete io failed: {:?}", e);
                return Err(e)
            }
        }

        self.io.shutdown()
    }
}
