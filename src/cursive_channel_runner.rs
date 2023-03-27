use cursive::{backend, event::Event, theme, Cursive, Vec2};
use std::borrow::{Borrow, BorrowMut};
use std::time::Duration;
use tokio;
use crate::{Tup,TuiUpdate,ViewSpec};
use crate::{Multiaddr,Arguments};

// Cursive TUI api
use cursive;

use cursive::direction::Orientation::{Horizontal, Vertical};
use cursive::theme::*;
use cursive::traits::*;
use cursive::view::{Nameable, Position, Scrollable};
use cursive::views::{
    Button, Dialog, EditView, LinearLayout, Panel, ResizedView, ScrollView, TextView,
};

// How long we wait between two empty input polls
const INPUT_POLL_DELAY_MS: u64 = 30;

/// Event loop runner for a cursive instance.
///
/// You can get one from `Cursive::runner`, then either call `.run()`, or
/// manually `.step()`.
///
/// The `C` type is usually either `Cursive` or `&mut Cursive`.
pub struct CursiveChannelRunner<C> {
    siv: C,
    backend: Box<dyn backend::Backend>,
    boring_frame_count: u32,
    // Last layer sizes of the stack view.
    // If it changed, clear the screen.
    last_sizes: Vec<Vec2>,
    // look here! lol new needs this now. need to add it!
    update_receiver: std::sync::mpsc::Receiver<Box<TuiUpdate>>,
}

impl<C> std::ops::Deref for CursiveChannelRunner<C>
where
    C: Borrow<Cursive>,
{
    type Target = Cursive;

    fn deref(&self) -> &Cursive {
        self.siv.borrow()
    }
}

impl<C> std::ops::DerefMut for CursiveChannelRunner<C>
where
    C: BorrowMut<Cursive>,
{
    fn deref_mut(&mut self) -> &mut Cursive {
        self.siv.borrow_mut()
    }
}

impl<C> CursiveChannelRunner<C> {
    /// Creates a new cursive runner wrapper.
    pub fn new(siv: C, backend: Box<dyn backend::Backend>,
               update_receiver: std::sync::mpsc::Receiver<Box<TuiUpdate>>,
    ) -> Self {
        CursiveChannelRunner {
            siv,
            backend,
            update_receiver,
            boring_frame_count: 0,
            last_sizes: Vec::new(),
        }
    }

    /// Returns the size of the screen, in characters.
    fn screen_size(&self) -> Vec2 {
        self.backend.screen_size()
    }

    /// Clean out the terminal and get back the wrapped object.
    pub fn into_inner(self) -> C {
        self.siv
    }
}

impl<C> CursiveChannelRunner<C>
where
    C: BorrowMut<Cursive>,
{
    // Handle messages from an mpsc channel without sharing mut references across threads

    fn layout(&mut self) {
        let size = self.screen_size();
        self.siv.borrow_mut().layout(size);
    }

    fn handle_channel_updates(&mut self){
        if let update = self.update_receiver.try_recv() {
            match *update {
                TuiUpdate::TerminalOutput( Tup::MessageText(output))=>{
                    self.siv.call_on_name("output_view", |view: &mut TextView| {
                        view.append(format!("{}\r", output));
                    });
                }
            }
        }
    }

    // Process any backend-requiring calls accumulated by the Cursive root.
    fn process_pending_backend_calls(&mut self) {
        let calls = std::mem::take(&mut self.backend_calls);
        for call in calls {
            (call)(&mut *self.backend);
        }
    }

    fn draw(&mut self) {
        let sizes = self.screen().layer_sizes();
        if self.last_sizes != sizes {
            // TODO: Maybe we only need to clear if the _max_ size differs?
            // Or if the positions change?
            self.clear();
            self.last_sizes = sizes;
        }

        if self.needs_clear {
            self.backend
                .clear(self.current_theme().palette[theme::PaletteColor::Background]);
            self.needs_clear = false;
        }

        let size = self.screen_size();

        self.siv.borrow_mut().draw(size, &*self.backend);
    }

    /// Performs the first half of `Self::step()`.
    ///
    /// This is an advanced method for fine-tuned manual stepping;
    /// you probably want [`run`][1] or [`step`][2].
    ///
    /// This processes any pending event or callback. After calling this,
    /// you will want to call [`post_events`][3] with the result from this
    /// function.
    ///
    /// Returns `true` if an event or callback was received,
    /// and `false` otherwise.
    ///
    /// [1]: CursiveChannelRunner::run()
    /// [2]: CursiveChannelRunner::step()
    /// [3]: CursiveChannelRunner::post_events()
    pub fn process_events(&mut self) -> bool {
        // Things are boring if nothing significant happened.
        let mut boring = true;

        // First, handle all available input
        while let Some(event) = self.backend.poll_event() {
            boring = false;
            self.on_event(event);
            self.process_pending_backend_calls();

            if !self.is_running() {
                return true;
            }
        }

        // Then, handle any available callback
        while self.process_callback() {
            boring = false;

            if !self.is_running() {
                return true;
            }
        }

        !boring
    }

    /// Performs the second half of `Self::step()`.
    ///
    /// This is an advanced method for fine-tuned manual stepping;
    /// you probably want [`run`][1] or [`step`][2].
    ///
    /// You should call this after [`process_events`][3].
    ///
    /// [1]: CursiveChannelRunner::run()
    /// [2]: CursiveChannelRunner::step()
    /// [3]: CursiveChannelRunner::process_events()
    pub fn post_events(&mut self, received_something: bool) {
        let boring = !received_something;
        // How many times should we try if it's still boring?
        // Total duration will be INPUT_POLL_DELAY_MS * repeats
        // So effectively fps = 1000 / INPUT_POLL_DELAY_MS / repeats
        if !boring
            || self
                .fps()
                .map(|fps| 1000 / INPUT_POLL_DELAY_MS as u32 / fps.get())
                .map(|repeats| self.boring_frame_count >= repeats)
                .unwrap_or(false)
        {
            // We deserve to draw something!

            if boring {
                // We're only here because of a timeout.
                self.on_event(Event::Refresh);
                self.process_pending_backend_calls();
            }

            self.refresh();
        }

        if boring {
            std::thread::sleep(Duration::from_millis(INPUT_POLL_DELAY_MS));
            self.boring_frame_count += 1;
        }
    }

    /// Refresh the screen with the current view tree state.
    pub fn refresh(&mut self) {
        self.boring_frame_count = 0;

        // Do we need to redraw everytime?
        // Probably, actually.
        // TODO: Do we need to re-layout everytime?
        self.layout();

        // TODO: Do we need to redraw every view every time?
        // (Is this getting repetitive? :p)
        self.draw();
        self.backend.refresh();
    }

    /// Return the name of the backend used.
    ///
    /// Mostly used for debugging.
    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }

    /// Performs a single step from the event loop.
    ///
    /// Useful if you need tighter control on the event loop.
    /// Otherwise, [`run(&mut self)`] might be more convenient.
    ///
    /// Returns `true` if an input event or callback was received
    /// during this step, and `false` otherwise.
    ///
    /// [`run(&mut self)`]: #method.run
    pub fn step(&mut self) -> bool {
        let received_something = self.process_events();
        // A CHANGE HERE added the function that handles channel messages
        // Channel Handling  like view updates from the channels.
        self.handle_channel_updates();

        self.post_events(received_something);
        received_something
    }

    /// Runs the event loop.
    ///
    /// It will wait for user input (key presses)
    /// and trigger callbacks accordingly.
    ///
    /// Internally, it calls [`step(&mut self)`] until [`quit(&mut self)`] is
    /// called.
    ///
    /// After this function returns, you can call it again and it will start a
    /// new loop.
    ///
    /// [`step(&mut self)`]: #method.step
    /// [`quit(&mut self)`]: #method.quit
    pub fn run(&mut self) {
        self.refresh();

        // And the big event loop begins!
        while self.is_running() {
            self.step();
        }
    }
}
