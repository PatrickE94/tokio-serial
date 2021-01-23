//! Bindings for serial port I/O and futures
//!
//! This crate provides bindings between `mio_serial`, a mio crate for
//! serial port I/O, and `futures`.  The API is very similar to the
//! bindings in `mio_serial`
//!
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]

// Re-export serialport types and traits from mio_serial
pub use mio_serial::{
    new, ClearBuffer, DataBits, Error, ErrorKind, FlowControl, Parity, SerialPort,
    SerialPortBuilder, StopBits,
};

/// A type for results generated by interacting with serial ports.
pub type Result<T> = mio_serial::Result<T>;

#[cfg(unix)]
use tokio::io::unix::AsyncFd;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use futures::ready;
use std::io::{self, Read, Write};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// Serial port I/O struct.
pub struct Serial {
    io: AsyncFd<mio_serial::Serial>,
}

impl Serial {
    /// Open serial port from a provided builder, using the default reactor.
    pub fn from_builder(builder: mio_serial::SerialPortBuilder) -> io::Result<Serial> {
        let port = mio_serial::Serial::from_builder(builder)?;
        let io = AsyncFd::new(port)?;

        Ok(Serial { io })
    }

    /// Create a pair of pseudo serial terminals using the default reactor
    ///
    /// ## Returns
    /// Two connected, unnamed `Serial` objects.
    ///
    /// ## Errors
    /// Attempting any IO or parameter settings on the slave tty after the master
    /// tty is closed will return errors.
    ///
    #[cfg(unix)]
    pub fn pair() -> Result<(Self, Self)> {
        let (master, slave) = mio_serial::Serial::pair()?;

        let master = Serial {
            io: AsyncFd::new(master)?,
        };
        let slave = Serial {
            io: AsyncFd::new(slave)?,
        };
        Ok((master, slave))
    }

    /// Sets the exclusivity of the port
    ///
    /// If a port is exclusive, then trying to open the same device path again
    /// will fail.
    ///
    /// See the man pages for the tiocexcl and tiocnxcl ioctl's for more details.
    ///
    /// ## Errors
    ///
    /// * `Io` for any error while setting exclusivity for the port.
    #[cfg(unix)]
    pub fn set_exclusive(&mut self, exclusive: bool) -> Result<()> {
        self.io.get_mut().set_exclusive(exclusive)
    }

    /// Returns the exclusivity of the port
    ///
    /// If a port is exclusive, then trying to open the same device path again
    /// will fail.
    #[cfg(unix)]
    pub fn exclusive(&self) -> bool {
        self.io.get_ref().exclusive()
    }
}

impl SerialPort for Serial {
    fn name(&self) -> Option<String> {
        self.io.get_ref().name()
    }

    fn baud_rate(&self) -> Result<u32> {
        self.io.get_ref().baud_rate()
    }

    fn data_bits(&self) -> Result<DataBits> {
        self.io.get_ref().data_bits()
    }

    fn flow_control(&self) -> Result<FlowControl> {
        self.io.get_ref().flow_control()
    }

    fn parity(&self) -> Result<Parity> {
        self.io.get_ref().parity()
    }

    fn stop_bits(&self) -> Result<StopBits> {
        self.io.get_ref().stop_bits()
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(0)
    }

    fn set_baud_rate(&mut self, baud_rate: u32) -> Result<()> {
        self.io.get_mut().set_baud_rate(baud_rate)
    }

    fn set_data_bits(&mut self, data_bits: DataBits) -> Result<()> {
        self.io.get_mut().set_data_bits(data_bits)
    }

    fn set_flow_control(&mut self, flow_control: FlowControl) -> Result<()> {
        self.io.get_mut().set_flow_control(flow_control)
    }

    fn set_parity(&mut self, parity: Parity) -> Result<()> {
        self.io.get_mut().set_parity(parity)
    }

    fn set_stop_bits(&mut self, stop_bits: StopBits) -> Result<()> {
        self.io.get_mut().set_stop_bits(stop_bits)
    }

    fn set_timeout(&mut self, _: Duration) -> Result<()> {
        Ok(())
    }

    fn set_break(&self) -> Result<()> {
        self.io.get_ref().set_break()
    }

    fn clear_break(&self) -> Result<()> {
        self.io.get_ref().clear_break()
    }

    fn write_request_to_send(&mut self, level: bool) -> Result<()> {
        self.io.get_mut().write_request_to_send(level)
    }

    fn write_data_terminal_ready(&mut self, level: bool) -> Result<()> {
        self.io.get_mut().write_data_terminal_ready(level)
    }

    fn read_clear_to_send(&mut self) -> Result<bool> {
        self.io.get_mut().read_clear_to_send()
    }

    fn read_data_set_ready(&mut self) -> Result<bool> {
        self.io.get_mut().read_data_set_ready()
    }

    fn read_ring_indicator(&mut self) -> Result<bool> {
        self.io.get_mut().read_ring_indicator()
    }

    fn read_carrier_detect(&mut self) -> Result<bool> {
        self.io.get_mut().read_carrier_detect()
    }

    fn bytes_to_read(&self) -> Result<u32> {
        self.io.get_ref().bytes_to_read()
    }

    fn bytes_to_write(&self) -> Result<u32> {
        self.io.get_ref().bytes_to_write()
    }

    fn clear(&self, buffer_to_clear: ClearBuffer) -> Result<()> {
        self.io.get_ref().clear(buffer_to_clear)
    }

    fn try_clone(&self) -> Result<Box<dyn SerialPort>> {
        self.io.get_ref().try_clone()
    }
}

impl Read for Serial {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.io.get_mut().read(buf)
    }
}

impl Write for Serial {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.io.get_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.io.get_mut().flush()
    }
}

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(unix)]
impl AsRawFd for Serial {
    fn as_raw_fd(&self) -> RawFd {
        self.io.get_ref().as_raw_fd()
    }
}

impl AsyncRead for Serial {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            let mut guard = ready!(self.io.poll_read_ready(cx))?;

            match guard.try_io(|_| {
                let read = self.io.get_ref().read(buf.initialize_unfilled())?;
                return Ok(buf.advance(read));
            }) {
                Ok(result) => return Poll::Ready(result),
                Err(_would_block) => continue,
            }
        }
    }
}

impl AsyncWrite for Serial {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        loop {
            let mut guard = ready!(self.io.poll_write_ready(cx))?;

            match guard.try_io(|_| self.io.get_ref().write(buf)) {
                Ok(result) => return Poll::Ready(result),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        loop {
            let mut guard = ready!(self.io.poll_write_ready(cx))?;
            match guard.try_io(|_| self.io.get_ref().flush()) {
                Ok(_) => return Poll::Ready(Ok(())),
                Err(_would_block) => continue,
            }
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
