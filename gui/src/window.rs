//! Windows that show an element tree and pass on its input (M5-35, M5-36, M5-37). winit allows
//! one event loop per process, so it is built once, on its own thread, and kept; each window
//! opens in it and closes alone (M5-09).

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;

use paka_rt::layout::{Rect, layout};
use paka_rt::route::Event;
use paka_rt::tree::{Node, Step};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::ModifiersState;
use winit::platform::wayland::EventLoopBuilderExtWayland;
use winit::window::{self, WindowId};

use crate::events::{self, Pointer, focused_input};
use crate::text::Text;

type OnClose = Box<dyn FnOnce() + Send>;
type OnInput = Box<dyn Fn(Event) + Send>;

enum Order {
    Open(String, OnClose, OnInput, Sender<WindowId>),
    Show(WindowId, Node<()>),
    Close(WindowId),
}

/// An open window. Dropping it closes the window.
pub struct Window {
    id: WindowId,
    to_loop: EventLoopProxy<Order>,
}

impl Window {
    /// Opens a window titled `title` at `crate::SIZE` units; `None` with no display. Its clicks,
    /// keys and input-method text go to `on_input`; a close request closes it and calls
    /// `on_close`.
    pub fn open(
        title: &str,
        on_close: impl FnOnce() + Send + 'static,
        on_input: impl Fn(Event) + Send + 'static,
    ) -> Option<Window> {
        let to_loop = event_loop()?.lock().ok()?.clone();
        let (reply, id) = mpsc::channel();
        let order = Order::Open(title.into(), Box::new(on_close), Box::new(on_input), reply);
        to_loop.send_event(order).ok()?;
        Some(Window {
            id: id.recv().ok()?,
            to_loop,
        })
    }

    /// Shows `tree` from the next frame on.
    pub fn show<M>(&self, tree: &Node<M>) {
        let _ = self.to_loop.send_event(Order::Show(self.id, plain(tree)));
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        let _ = self.to_loop.send_event(Order::Close(self.id));
    }
}

/// The kept event loop, started on first use; `None` when it cannot start.
fn event_loop() -> Option<&'static Mutex<EventLoopProxy<Order>>> {
    static LOOP: OnceLock<Option<Mutex<EventLoopProxy<Order>>>> = OnceLock::new();
    let started = LOOP.get_or_init(|| {
        let (reply, proxy) = mpsc::channel();
        thread::spawn(move || {
            let built = EventLoop::with_user_event().with_any_thread(true).build();
            let _ = reply.send(built.as_ref().ok().map(EventLoop::create_proxy));
            if let Ok(event_loop) = built {
                let mut shown = Shown {
                    windows: BTreeMap::new(),
                    text: Text::default(),
                };
                let _ = event_loop.run_app(&mut shown);
            }
        });
        proxy.recv().ok().flatten().map(Mutex::new)
    });
    started.as_ref()
}

/// The tree with its messages dropped: a window only draws it.
fn plain<M>(node: &Node<M>) -> Node<()> {
    let kids = |children: &[(Step, Node<M>)]| {
        let kids = children
            .iter()
            .map(|(step, child)| (step.clone(), plain(child)));
        kids.collect()
    };
    match node {
        Node::Text(s) => Node::Text(s.clone()),
        Node::Button { label, focus, .. } => Node::Button {
            label: label.clone(),
            on: (),
            focus: *focus,
        },
        Node::Input { text, edit, .. } => Node::Input {
            text: text.clone(),
            on_change: (),
            edit: edit.clone(),
        },
        Node::Row { gap, children } => Node::Row {
            gap: *gap,
            children: kids(children),
        },
        Node::Col { gap, children } => Node::Col {
            gap: *gap,
            children: kids(children),
        },
        Node::Hole(name) => Node::Hole(name.clone()),
        Node::Error(message) => Node::Error(message.clone()),
    }
}

/// The open windows, on the loop's thread.
struct Shown {
    windows: BTreeMap<WindowId, Open>,
    text: Text,
}

struct Open {
    window: Rc<window::Window>,
    surface: Surface<Rc<window::Window>, Rc<window::Window>>,
    tree: Node<()>,
    /// The tree laid out at the window's width, for hit testing and the IME area.
    rects: Vec<Rect>,
    on_close: Option<OnClose>,
    on_input: OnInput,
    pointer: Pointer,
    mods: ModifiersState,
    /// The IME cursor area last set, in units; `None` while no Input has focus.
    ime_area: Option<(f64, f64)>,
    presented: bool,
}

impl ApplicationHandler<Order> for Shown {
    fn resumed(&mut self, _: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, order: Order) {
        match order {
            Order::Open(title, on_close, on_input, reply) => {
                let Some(open) = open(event_loop, &title, on_close, on_input) else {
                    return;
                };
                let id = open.window.id();
                self.windows.insert(id, open);
                let _ = reply.send(id);
            }
            Order::Show(id, tree) => {
                if let Some(open) = self.windows.get_mut(&id) {
                    open.tree = tree;
                    open.lay_out(&self.text);
                    open.window.request_redraw();
                }
            }
            Order::Close(id) => {
                self.windows.remove(&id);
            }
        }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(open) = self.windows.get_mut(&id) else {
            return;
        };
        match event {
            WindowEvent::RedrawRequested => open.redraw(&self.text),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                open.window.request_redraw()
            }
            WindowEvent::CloseRequested => {
                let on_close = open.on_close.take();
                self.windows.remove(&id);
                if let Some(f) = on_close {
                    f()
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = open.window.scale_factor();
                open.pointer.moved(position.x, position.y, scale);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pointer = &mut open.pointer;
                if let Some(click) = pointer.button(button, state, &open.tree, &open.rects) {
                    (open.on_input)(click);
                }
            }
            WindowEvent::ModifiersChanged(mods) => open.mods = mods.state(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        text,
                        state: ElementState::Pressed,
                        ..
                    },
                is_synthetic: false,
                ..
            } => {
                for event in events::key(&logical_key, text.as_deref(), open.mods) {
                    (open.on_input)(event);
                }
            }
            WindowEvent::Ime(ime) => {
                let shown = focused_input(&open.tree, &open.rects);
                let preedit = shown.map_or("", |(_, _, edit)| edit.preedit.as_str());
                if let Some(event) = events::ime(ime, preedit) {
                    (open.on_input)(event);
                }
            }
            _ => {}
        }
    }
}

fn open(
    event_loop: &ActiveEventLoop,
    title: &str,
    on_close: OnClose,
    on_input: OnInput,
) -> Option<Open> {
    let (w, h) = crate::SIZE;
    let attributes = window::Window::default_attributes()
        .with_title(title)
        .with_inner_size(LogicalSize::new(w as f64, h as f64));
    let window = Rc::new(event_loop.create_window(attributes).ok()?);
    let context = Context::new(window.clone()).ok()?;
    let surface = Surface::new(&context, window.clone()).ok()?;
    Some(Open {
        window,
        surface,
        tree: Node::Col {
            gap: 0,
            children: vec![],
        },
        rects: vec![],
        on_close: Some(on_close),
        on_input,
        pointer: Pointer::default(),
        mods: ModifiersState::empty(),
        ime_area: None,
        presented: false,
    })
}

impl Open {
    /// Lays the tree out at the window's width in units, and moves the IME cursor area to the
    /// focused Input's cursor when it moved there. The input method is on only while an Input
    /// has focus.
    fn lay_out(&mut self, text: &Text) {
        let width = f64::from(self.window.inner_size().width) / self.window.scale_factor();
        self.rects = layout(&self.tree, width as u64, text);
        let area = focused_input(&self.tree, &self.rects).map(|(r, shown, edit)| {
            let before: String = shown.chars().take(edit.cursor).collect();
            let x = r.x as f64 + 8.0 + f64::from(text.advance(&before));
            (x, r.y as f64 + 6.0)
        });
        if area == self.ime_area {
            return;
        }
        if area.is_some() != self.ime_area.is_some() {
            self.window.set_ime_allowed(area.is_some());
        }
        // Each move makes the input method send its preedit again (M5-08).
        if let Some((x, y)) = area {
            let (at, size) = (LogicalPosition::new(x, y), LogicalSize::new(1.0, 20.0));
            self.window.set_ime_cursor_area(at, size);
        }
        self.ime_area = area;
    }

    /// Lays the tree out at the window's width and presents it.
    fn redraw(&mut self, text: &Text) {
        let size = self.window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        let scale = self.window.scale_factor();
        self.lay_out(text);
        let pixmap = crate::draw::draw(
            &self.tree,
            &self.rects,
            text,
            scale,
            w.get() as u16,
            h.get() as u16,
        );
        let Ok(()) = self.surface.resize(w, h) else {
            return;
        };
        let Ok(mut buffer) = self.surface.buffer_mut() else {
            return;
        };
        for (out, px) in buffer.iter_mut().zip(pixmap.data()) {
            *out = u32::from_be_bytes([0, px.r, px.g, px.b]);
        }
        if buffer.present().is_ok() && !self.presented {
            self.presented = true;
            eprintln!("first frame presented, {w}x{h} px at scale {scale}");
        }
    }
}
