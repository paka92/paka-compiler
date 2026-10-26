//! M5-08's library trial: one window, one text field, one button.
//!
//! `paka-gui-trial [--seconds N]` opens the window and exits by itself after N seconds.
//! `paka-gui-trial --png FILE [--scale S]` draws the screen into a PNG without a window.

mod png;
mod ui;

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};

use accesskit_winit::{Adapter, Event as A11yEvent, WindowEvent as A11y};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use ui::{Frame, Hit, Text, Ui};

struct Shown {
    window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    adapter: Adapter,
}

struct App {
    proxy: EventLoopProxy<A11yEvent>,
    shown: Option<Shown>,
    ui: Ui,
    text: Text,
    frame: Frame,
    pointer: (f64, f64),
    pressed: Option<Hit>,
    start: Instant,
    deadline: Option<Instant>,
    frames: u32,
    ime_area: Option<vello_cpu::kurbo::Rect>,
}

impl App {
    fn log(&self, what: &str) {
        eprintln!("[{:7.3}s] {what}", self.start.elapsed().as_secs_f64());
    }

    /// Redraws and sends the new tree after the model changed.
    fn changed(&mut self) {
        let Some(shown) = &mut self.shown else { return };
        shown.window.request_redraw();
        let scale = shown.window.scale_factor();
        let (frame, ui) = (&self.frame, &self.ui);
        shown.adapter.update_if_active(|| frame.tree(ui, scale));
        let cursor = self.frame.cursor(&self.ui, &mut self.text);
        // Each move makes the IME resend its preedit, so move only when the cursor moved.
        if self.ime_area == Some(cursor) {
            return;
        }
        self.ime_area = Some(cursor);
        shown.window.set_ime_cursor_area(
            LogicalPosition::new(cursor.x0, cursor.y0),
            LogicalSize::new(cursor.width(), cursor.height()),
        );
    }

    fn redraw(&mut self) {
        let Some(shown) = &mut self.shown else { return };
        let size = shown.window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        let scale = shown.window.scale_factor();
        let pixmap = ui::draw(
            &self.ui,
            &self.frame,
            &mut self.text,
            scale,
            w.get() as u16,
            h.get() as u16,
        );
        shown.surface.resize(w, h).expect("surface resizes");
        let mut buffer = shown.surface.buffer_mut().expect("buffer");
        for (out, px) in buffer.iter_mut().zip(pixmap.data()) {
            *out = u32::from_be_bytes([0, px.r, px.g, px.b]);
        }
        buffer.present().expect("present");
        self.frames += 1;
        if self.frames == 1 {
            self.log(&format!(
                "first frame presented, {}x{} px at scale {scale}",
                w, h
            ));
        }
    }
}

impl ApplicationHandler<A11yEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.shown.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Paka trial")
            .with_inner_size(LogicalSize::new(ui::WIDTH, ui::HEIGHT))
            .with_visible(false);
        let window = Rc::new(event_loop.create_window(attrs).expect("window opens"));
        let adapter = Adapter::with_event_loop_proxy(event_loop, &window, self.proxy.clone());
        window.set_visible(true);
        window.set_ime_allowed(true);
        let context = Context::new(window.clone()).expect("softbuffer context");
        let surface = Surface::new(&context, window.clone()).expect("softbuffer surface");
        self.shown = Some(Shown {
            window,
            surface,
            adapter,
        });
        self.log("window created");
        self.changed();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let Some(shown) = &mut self.shown {
            shown.adapter.process_event(&shown.window, &event);
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::Focused(on) => self.log(&format!("window focused: {on}")),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.log(&format!("scale factor {scale_factor}"));
                self.changed();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.shown.as_ref().map_or(1.0, |s| s.window.scale_factor());
                self.pointer = (position.x / scale, position.y / scale);
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                let hit = self.frame.hit(self.pointer.0, self.pointer.1);
                match state {
                    ElementState::Pressed => self.pressed = hit,
                    ElementState::Released => {
                        if let Some(h) = hit.filter(|&h| self.pressed == Some(h)) {
                            self.ui.click(h);
                            self.changed();
                        }
                        self.pressed = None;
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } if self.ui.focused => {
                if event.state != ElementState::Pressed {
                    return;
                }
                if event.logical_key == Key::Named(NamedKey::Backspace) {
                    self.ui.text.pop();
                } else if let Some(t) = &event.text {
                    self.ui.text.extend(t.chars().filter(|c| !c.is_control()));
                }
                self.changed();
            }
            WindowEvent::Ime(Ime::Preedit(text, _)) if text == self.ui.preedit => {}
            WindowEvent::Ime(ime) => {
                self.log(&format!("ime {ime:?}"));
                match ime {
                    Ime::Preedit(text, _) => self.ui.preedit = text,
                    Ime::Commit(text) => {
                        self.ui.preedit.clear();
                        self.ui.text.push_str(&text);
                    }
                    Ime::Enabled | Ime::Disabled => self.ui.preedit.clear(),
                }
                self.changed();
            }
            _ => {}
        }
    }

    fn user_event(&mut self, _: &ActiveEventLoop, event: A11yEvent) {
        match event.window_event {
            A11y::InitialTreeRequested => {
                self.log("accessibility tree requested");
                self.changed();
            }
            A11y::ActionRequested(req) => {
                self.log(&format!(
                    "accessibility action {:?} on {:?}",
                    req.action, req.target_node
                ));
                let value = match &req.data {
                    Some(accesskit::ActionData::Value(v)) => Some(&**v),
                    _ => None,
                };
                if self.ui.act(req.target_node, req.action, value) {
                    self.changed();
                }
            }
            A11y::AccessibilityDeactivated => self.log("accessibility deactivated"),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.deadline {
            Some(at) if Instant::now() >= at => {
                self.log(&format!("time is up after {} frames", self.frames));
                event_loop.exit();
            }
            Some(at) => event_loop.set_control_flow(ControlFlow::WaitUntil(at)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }
}

fn arg(args: &[String], name: &str) -> Option<String> {
    let at = args.iter().position(|a| a == name)?;
    Some(
        args.get(at + 1)
            .unwrap_or_else(|| panic!("{name} needs a value"))
            .clone(),
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let font = arg(&args, "--font").unwrap_or_else(|| ui::FONT_PATH.to_string());
    let mut text = Text::load(&font).unwrap_or_else(|e| panic!("{e}"));
    let frame = Frame::new(&mut text, ui::WIDTH);

    if let Some(path) = arg(&args, "--png") {
        let scale: f64 =
            arg(&args, "--scale").map_or(1.0, |s| s.parse().expect("scale is a number"));
        let ui = Ui {
            text: "milk".into(),
            focused: true,
            ..Ui::default()
        };
        let (w, h) = ((ui::WIDTH * scale) as u16, (ui::HEIGHT * scale) as u16);
        let pixmap = ui::draw(&ui, &frame, &mut text, scale, w, h);
        let bytes = png::encode(w.into(), h.into(), pixmap.data_as_u8_slice());
        std::fs::write(&path, bytes).expect("png written");
        return;
    }

    let seconds: Option<u64> =
        arg(&args, "--seconds").map(|s| s.parse().expect("seconds is a number"));
    let event_loop = EventLoop::<A11yEvent>::with_user_event()
        .build()
        .expect("event loop (needs a Wayland session)");
    let start = Instant::now();
    let mut app = App {
        proxy: event_loop.create_proxy(),
        shown: None,
        ui: Ui::default(),
        text,
        frame,
        pointer: (0.0, 0.0),
        pressed: None,
        start,
        deadline: seconds.map(|s| start + Duration::from_secs(s)),
        frames: 0,
        ime_area: None,
    };
    event_loop.run_app(&mut app).expect("event loop runs");
}
