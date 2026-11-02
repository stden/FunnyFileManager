//! Purpose: `NoopTerminalOps`, the inert `TerminalOps` double that session and suspension tests run on.

use ratatui::{backend::Backend, Terminal};
use std::io::{self, Write};

use super::TerminalOps;

pub struct NoopTerminalOps;

impl<B> TerminalOps<B> for NoopTerminalOps
where
    B: Backend + Write,
{
    fn enable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn disable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn enter_alternate(&mut self, _terminal: &mut Terminal<B>) -> io::Result<()> {
        Ok(())
    }

    fn leave_alternate(&mut self, _terminal: &mut Terminal<B>) -> io::Result<()> {
        Ok(())
    }

    fn enable_mouse(&mut self, _terminal: &mut Terminal<B>) -> io::Result<()> {
        Ok(())
    }

    fn disable_mouse(&mut self, _terminal: &mut Terminal<B>) -> io::Result<()> {
        Ok(())
    }

    fn clear(&mut self, _terminal: &mut Terminal<B>) -> io::Result<()> {
        Ok(())
    }

    fn show_cursor(&mut self, _terminal: &mut Terminal<B>) -> io::Result<()> {
        Ok(())
    }

    fn wait_for_key(&mut self, _terminal: &mut Terminal<B>, _prompt: &str) -> io::Result<()> {
        Ok(())
    }
}
