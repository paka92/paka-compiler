//! Windows that show an element tree and pass on its input (M5-35, M5-36, M5-37). winit allows
//! one event loop per process, so it is built once, on its own thread, and kept; each window
//! opens in it and closes alone (M5-09). Each window also shows its tree to assistive tools
//! through AccessKit and takes their actions as input (M5-39).

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::thread;

use accesskit_winit::{Adapter, WindowEvent as AccessEvent};
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

use crate::access::{self, Ids};
use crate::events::{self, Pointer, focused_input};
use crate::text::Text;

type OnClose = Box<dyn FnOnce() + Send>;
type OnInput = Box<dyn Fn(Event) + Send>;

enum Order {
    Open(String, Node<()>, Size, OnClose, OnInput, Sender<WindowId>),
    Show(WindowId, Node<()>),
    Close(WindowId),
    Access(accesskit_winit::Event),
}

impl From<accesskit_winit::Event> for Order {
    fn from(event: accesskit_winit::Event) -> Order {
        Order::Access(event)
    }
}

/// An open window. Dropping it closes the window.
pub struct Window {
    id: WindowId,
    to_loop: EventLoopProxy<Order>,
    size: Size,
}

/// A window's size in units, kept current as it changes.
#[derive(Clone)]
pub struct Size(Arc<Mutex<(u64, u64)>>);

impl Size {
    pub fn get(&self) -> (u64, u64) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn set(&self, size: (u64, u64)) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = size;
    }
}

impl Window {
    /// Opens a window titled `title` at `crate::SIZE` units, showing `tree`; `None` with no
    /// display. Its clicks, keys and input-method text go to `on_input`; a close request closes
    /// it and calls `on_close`.
    pub fn open<M>(
        title: &str,
        tree: &Node<M>,
        on_close: impl FnOnce() + Send + 'static,
        on_input: impl Fn(Event) + Send + 'static,
    ) -> Option<Window> {
        let to_loop = event_loop()?.lock().ok()?.clone();
        let (reply, id) = mpsc::channel();
        let size = Size(Arc::new(Mutex::new(crate::SIZE)));
        let (on_close, on_input) = (Box::new(on_close), Box::new(on_input));
        let order = Order::Open(
            title.into(),
            plain(tree),
            size.clone(),
            on_close,
            on_input,
            reply,
        );
        to_loop.send_event(order).ok()?;
        Some(Window {
            id: id.recv().ok()?,
            to_loop,
            size,
        })
    }

    /// Shows `tree` from the next frame on; a tree unlike the shown one requests that frame.
    pub fn show<M>(&self, tree: &Node<M>) {
        let _ = self.to_loop.send_event(Order::Show(self.id, plain(tree)));
    }

    /// Its size in units, kept current.
    pub fn size(&self) -> Size {
        self.size.clone()
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
                    proxy: event_loop.create_proxy(),
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
    /// For AccessKit's events.
    proxy: EventLoopProxy<Order>,
}

struct Open {
    window: Rc<window::Window>,
    title: String,
    adapter: Adapter,
    ids: Ids,
    /// The scale the tree's AccessKit form was last given at; `None` since the tree changed.
    told: Option<f64>,
    surface: Surface<Rc<window::Window>, Rc<window::Window>>,
    tree: Node<()>,
    /// The tree laid out at the window's width, for hit testing and the IME area.
    rects: Vec<Rect>,
    on_close: Option<OnClose>,
    on_input: OnInput,
    size: Size,
    pointer: Pointer,
    mods: ModifiersState,
    /// The IME cursor area last set, in units; `None` while no Input has focus.
    ime_area: Option<(f64, f64)>,
    presented: bool,
    /// Whether the tree changed since the last frame presented.
    fresh: bool,
}

impl ApplicationHandler<Order> for Shown {
    fn resumed(&mut self, _: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, order: Order) {
        match order {
            Order::Open(title, tree, size, on_close, on_input, reply) => {
                let proxy = self.proxy.clone();
                let Some(open) = open(event_loop, proxy, title, tree, size, on_close, on_input)
                else {
                    return;
                };
                let id = open.window.id();
                self.windows.insert(id, open);
                let _ = reply.send(id);
            }
            Order::Show(id, tree) => {
                if let Some(open) = self.windows.get_mut(&id).filter(|o| o.tree != tree) {
                    open.tree = tree;
                    (open.fresh, open.told) = (true, None);
                    open.lay_out(&self.text);
                    open.window.request_redraw();
                }
            }
            Order::Close(id) => {
                self.windows.remove(&id);
            }
            Order::Access(event) => {
                if let Some(open) = self.windows.get_mut(&event.window_id) {
                    open.access(event.window_event);
                }
            }
        }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(open) = self.windows.get_mut(&id) else {
            return;
        };
        open.adapter.process_event(&open.window, &event);
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
    proxy: EventLoopProxy<Order>,
    title: String,
    tree: Node<()>,
    size: Size,
    on_close: OnClose,
    on_input: OnInput,
) -> Option<Open> {
    let (w, h) = crate::SIZE;
    let attributes = window::Window::default_attributes()
        .with_title(&title)
        .with_inner_size(LogicalSize::new(w as f64, h as f64))
        .with_visible(false);
    let window = Rc::new(event_loop.create_window(attributes).ok()?);
    // AccessKit must meet the window before it is shown.
    let adapter = Adapter::with_event_loop_proxy(event_loop, &window, proxy);
    window.set_visible(true);
    let context = Context::new(window.clone()).ok()?;
    let surface = Surface::new(&context, window.clone()).ok()?;
    Some(Open {
        window,
        title,
        adapter,
        ids: Ids::default(),
        told: None,
        surface,
        tree,
        rects: vec![],
        on_close: Some(on_close),
        on_input,
        size,
        pointer: Pointer::default(),
        mods: ModifiersState::empty(),
        ime_area: None,
        presented: false,
        fresh: false,
    })
}

impl Open {
    /// Lays the tree out at the window's width in units, and moves the IME cursor area to the
    /// focused Input's cursor when it moved there. The input method is on only while an Input
    /// has focus.
    fn lay_out(&mut self, text: &Text) {
        let (size, scale) = (self.window.inner_size(), self.window.scale_factor());
        let units = |px: u32| (f64::from(px) / scale) as u64;
        let (width, height) = (units(size.width), units(size.height));
        if width > 0 && height > 0 {
            self.size.set((width, height));
        }
        let rects = layout(&self.tree, width, text);
        if self.told != Some(scale) || rects != self.rects {
            self.rects = rects;
            let (ids, title, tree) = (&mut self.ids, &self.title, &self.tree);
            let update = || ids.update(title, tree, &self.rects, scale);
            self.adapter.update_if_active(update);
            self.told = Some(scale);
        }
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

    /// Answers an assistive tool: the whole tree when it asks, its action as input.
    fn access(&mut self, event: AccessEvent) {
        match event {
            AccessEvent::InitialTreeRequested => {
                let scale = self.window.scale_factor();
                let (ids, title, tree, rects) =
                    (&mut self.ids, &self.title, &self.tree, &self.rects);
                self.adapter
                    .update_if_active(|| ids.update(title, tree, rects, scale));
            }
            AccessEvent::ActionRequested(request) => {
                let path = self.ids.path(request.target_node);
                if let Some(event) =
                    path.and_then(|p| access::action(&self.tree, p, request.action))
                {
                    (self.on_input)(event);
                }
            }
            AccessEvent::AccessibilityDeactivated => {}
        }
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
        // Each new tree's frame is logged, so a test can see one follow each change.
        if buffer.present().is_ok() && (self.fresh || !self.presented) {
            let first = if self.presented { "" } else { "first " };
            eprintln!("{first}frame presented, {w}x{h} px at scale {scale}");
            (self.presented, self.fresh) = (true, false);
        }
    }
}
