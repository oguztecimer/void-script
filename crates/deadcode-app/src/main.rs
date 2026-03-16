mod app;

use winit::event_loop::EventLoop;

use app::App;
use deadcode_desktop::UserEvent;

fn main() {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("Failed to create event loop");

    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);
    event_loop.run_app(&mut app).expect("Event loop failed");
}
