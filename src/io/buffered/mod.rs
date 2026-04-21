//! Buffering wrappers for I/O traits

mod bufreader;
mod bufwriter;
mod linewriter;
mod linewritershim;

#[cfg(test)]
#[cfg(feature = "alloc")]
mod tests;

use core::{error, fmt};

use crate::io::Error;


pub use self::{
    bufreader::BufReader,
    bufwriter::{BufWriter, WriterPanicked},
    linewriter::LineWriter,
    linewritershim::LineWriterShim,
};
