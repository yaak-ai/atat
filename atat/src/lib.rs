//! A helper crate to abstract away the state management and string parsing of
//! AT command communication.
//!
//! It works by creating structs for each AT command, that each implements
//! [`AtatCmd`]. With corresponding response structs that each implements
//! [`AtatResp`].
//!
//! This can be simplified alot using the [`atat_derive`] crate!
//!
//! [`AtatCmd`]: trait.AtatCmd.html
//! [`AtatResp`]: trait.AtatResp.html
//! [`atat_derive`]: <https://crates.io/crates/atat_derive>
//!
//! # Examples
//!
//! ### Command and response example without `atat_derive`:
//! ```
//! use atat::{AtatCmd, AtatResp, Error, InternalError};
//! use core::fmt::Write;
//! use heapless::{String, Vec};
//!
//! pub struct SetGreetingText<'a> {
//!     pub text: &'a str,
//! }
//!
//! pub struct GetGreetingText;
//!
//! pub struct NoResponse;
//!
//! impl AtatResp for NoResponse {};
//!
//! pub struct GreetingText {
//!     pub text: String<64>,
//! };
//!
//! impl AtatResp for GreetingText {};
//!
//! impl<'a> AtatCmd<64> for SetGreetingText<'a> {
//!     type Response = NoResponse;
//!
//!     fn as_bytes(&self) -> Vec<u8, 64> {
//!         let mut buf: Vec<u8, 64> = Vec::new();
//!         write!(buf, "AT+CSGT={}", self.text);
//!         buf
//!     }
//!
//!     fn parse(&self, resp: Result<&[u8], InternalError>) -> Result<Self::Response, Error> {
//!         Ok(NoResponse)
//!     }
//! }
//!
//! impl AtatCmd<8> for GetGreetingText {
//!     type Response = GreetingText;
//!
//!     fn as_bytes(&self) -> Vec<u8, 8> {
//!         Vec::from_slice(b"AT+CSGT?").unwrap()
//!     }
//!
//!     fn parse(&self, resp: Result<&[u8], InternalError>) -> Result<Self::Response, Error> {
//!         // Parse resp into `GreetingText`
//!         Ok(GreetingText {
//!             text: String::from(core::str::from_utf8(resp.unwrap()).unwrap()),
//!         })
//!     }
//! }
//! ```
//!
//! ### Same example with `atat_derive`:
//! ```
//! use atat::atat_derive::{AtatCmd, AtatResp};
//! use heapless::String;
//!
//! #[derive(Clone, AtatCmd)]
//! #[at_cmd("+CSGT", NoResponse)]
//! pub struct SetGreetingText<'a> {
//!     #[at_arg(position = 0, len = 32)]
//!     pub text: &'a str,
//! }
//!
//! #[derive(Clone, AtatCmd)]
//! #[at_cmd("+CSGT?", GreetingText)]
//! pub struct GetGreetingText;
//!
//! #[derive(Clone, AtatResp)]
//! pub struct NoResponse;
//!
//! #[derive(Clone, AtatResp)]
//! pub struct GreetingText {
//!     #[at_arg(position = 0)]
//!     pub text: String<64>,
//! };
//! ```
//!
//! ### Basic usage example (More available in examples folder):
//! ```ignore
//!
//! use cortex_m::asm;
//! use hal::{
//!     gpio::{
//!         gpioa::{PA2, PA3},
//!         Alternate, Floating, Input, AF7,
//!     },
//!     pac::{interrupt, Peripherals, USART2},
//!     prelude::*,
//!     serial::{Config, Event::Rxne, Rx, Serial},
//!     timer::{Event, Timer},
//! };
//!
//! use atat::{atat_derive::{AtatResp, AtatCmd}};
//!
//! use heapless::{spsc::Queue, String};
//!
//! use crate::rt::entry;
//! static mut INGRESS: Option<atat::IngressManager> = None;
//! static mut RX: Option<Rx<USART2>> = None;
//!
//!
//! #[derive(Clone, AtatResp)]
//! pub struct NoResponse;
//!
//! #[derive(Clone, AtatCmd)]
//! #[at_cmd("", NoResponse, timeout_ms = 1000)]
//! pub struct AT;
//!
//! #[entry]
//! fn main() -> ! {
//!     let p = Peripherals::take().unwrap();
//!
//!     let mut flash = p.FLASH.constrain();
//!     let mut rcc = p.RCC.constrain();
//!     let mut pwr = p.PWR.constrain(&mut rcc.apb1r1);
//!
//!     let mut gpioa = p.GPIOA.split(&mut rcc.ahb2);
//!
//!     let clocks = rcc.cfgr.freeze(&mut flash.acr, &mut pwr);
//!
//!     let tx = gpioa.pa2.into_af7(&mut gpioa.moder, &mut gpioa.afrl);
//!     let rx = gpioa.pa3.into_af7(&mut gpioa.moder, &mut gpioa.afrl);
//!
//!     let mut timer = Timer::tim7(p.TIM7, 1.hz(), clocks, &mut rcc.apb1r1);
//!     let at_timer = Timer::tim6(p.TIM6, 100.hz(), clocks, &mut rcc.apb1r1);
//!
//!     let mut serial = Serial::usart2(
//!         p.USART2,
//!         (tx, rx),
//!         Config::default().baudrate(115_200.bps()),
//!         clocks,
//!         &mut rcc.apb1r1,
//!     );
//!
//!     serial.listen(Rxne);
//!
//!     static mut RES_QUEUE: ResQueue<256> = Queue::new();
//!     static mut URC_QUEUE: UrcQueue<256, 10> = Queue::new();
//!     static mut COM_QUEUE: ComQueue = Queue::new();
//!
//!     let queues = Queues {
//!         res_queue: unsafe { RES_QUEUE.split() },
//!         urc_queue: unsafe { URC_QUEUE.split() },
//!         com_queue: unsafe { COM_QUEUE.split() },
//!     };
//!
//!     let (tx, rx) = serial.split();
//!     let (mut client, ingress) =
//!         ClientBuilder::new(tx, timer, atat::Config::new(atat::Mode::Timeout)).build(queues);
//!
//!     unsafe { INGRESS = Some(ingress) };
//!     unsafe { RX = Some(rx) };
//!
//!     // configure NVIC interrupts
//!     unsafe { cortex_m::peripheral::NVIC::unmask(hal::stm32::Interrupt::TIM7) };
//!     timer.listen(Event::TimeOut);
//!
//!     // if all goes well you should reach this breakpoint
//!     asm::bkpt();
//!
//!     loop {
//!         asm::wfi();
//!
//!         match client.send(&AT) {
//!             Ok(response) => {
//!                 // Do something with response here
//!             }
//!             Err(e) => {}
//!         }
//!     }
//! }
//!
//! #[interrupt]
//! fn TIM7() {
//!     let ingress = unsafe { INGRESS.as_mut().unwrap() };
//!     ingress.digest();
//! }
//!
//! #[interrupt]
//! fn USART2() {
//!     let ingress = unsafe { INGRESS.as_mut().unwrap() };
//!     let rx = unsafe { RX.as_mut().unwrap() };
//!     if let Ok(d) = nb::block!(rx.read()) {
//!         ingress.write(&[d]);
//!     }
//! }
//! ```
//! # Optional Cargo Features
//!
//! - **`derive`** *(enabled by default)* - Re-exports [`atat_derive`] to allow
//!   deriving `Atat__` traits.

// #![deny(warnings)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unused_unit)]
#![allow(clippy::use_self)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::type_complexity)]
#![allow(clippy::fallible_impl_from)]
#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

mod builder;
mod client;
pub mod clock;
pub mod digest;
mod error;
pub mod helpers;
mod ingress_manager;
mod queues;
mod traits;

pub use bbqueue;
pub use nom;

#[cfg(feature = "bytes")]
pub use serde_bytes;

#[cfg(feature = "bytes")]
pub use heapless_bytes;

#[cfg(feature = "derive")]
pub use atat_derive;
#[cfg(feature = "derive")]
pub mod derive;

#[cfg(feature = "derive")]
pub use self::derive::AtatLen;

#[cfg(feature = "derive")]
pub use serde_at;

#[cfg(feature = "derive")]
pub use heapless;

pub use builder::ClientBuilder;
pub use client::{Client, Mode};
pub use digest::{AtDigester, AtDigester as DefaultDigester, DigestResult, Digester, Parser};
pub use error::{Error, InternalError, Response};
pub use ingress_manager::IngressManager;
pub use queues::Queues;
pub use traits::{AtatClient, AtatCmd, AtatResp, AtatUrc};

/// Configuration of both the ingress manager, and the AT client. Some of these
/// parameters can be changed on the fly, through issuing a [`Command`] from the
/// client.
///
/// [`Command`]: enum.Command.html
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Config {
    mode: Mode,
    cmd_cooldown: u32,
    tx_timeout: u32,
    flush_timeout: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Blocking,
            cmd_cooldown: 20,
            tx_timeout: 0,
            flush_timeout: 0,
        }
    }
}

impl Config {
    #[must_use]
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }

    #[must_use]
    pub const fn tx_timeout(mut self, ms: u32) -> Self {
        self.tx_timeout = ms;
        self
    }

    #[must_use]
    pub const fn flush_timeout(mut self, ms: u32) -> Self {
        self.flush_timeout = ms;
        self
    }

    #[must_use]
    pub const fn cmd_cooldown(mut self, ms: u32) -> Self {
        self.cmd_cooldown = ms;
        self
    }
}

#[cfg(test)]
#[cfg(feature = "defmt")]
mod tests {
    //! This module is required in order to satisfy the requirements of defmt, while running tests.
    //! Note that this will cause all log `defmt::` log statements to be thrown away.

    use core::ptr::NonNull;

    #[defmt::global_logger]
    struct Logger;
    impl defmt::Write for Logger {
        fn write(&mut self, _bytes: &[u8]) {}
    }

    unsafe impl defmt::Logger for Logger {
        fn acquire() -> Option<NonNull<dyn defmt::Write>> {
            Some(NonNull::from(&Logger as &dyn defmt::Write))
        }

        unsafe fn release(_: NonNull<dyn defmt::Write>) {}
    }

    defmt::timestamp!("");

    #[export_name = "_defmt_panic"]
    fn panic() -> ! {
        panic!()
    }
}
