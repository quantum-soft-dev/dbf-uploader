// Simple test to verify TextInput works
#![windows_subsystem = "windows"]

use native_windows_gui as nwg;

fn main() {
    nwg::init().expect("Failed to init NWG");

    let mut window = Default::default();
    let mut input = Default::default();
    let mut button = Default::default();

    nwg::Window::builder()
        .size((400, 200))
        .position((300, 300))
        .title("Input Test")
        .build(&mut window)
        .expect("Failed to build window");

    nwg::TextInput::builder()
        .text("Type here...")
        .position((20, 20))
        .size((360, 25))
        .parent(&window)
        .build(&mut input)
        .expect("Failed to build input");

    nwg::Button::builder()
        .text("Show Text")
        .position((20, 60))
        .size((120, 30))
        .parent(&window)
        .build(&mut button)
        .expect("Failed to build button");

    let window_handle = window.handle;
    let handler = nwg::full_bind_event_handler(&window_handle, move |evt, _evt_data, handle| {
        match evt {
            nwg::Event::OnWindowClose => {
                if &handle == &window {
                    nwg::stop_thread_dispatch();
                }
            }
            nwg::Event::OnButtonClick => {
                if &handle == &button {
                    let text = input.text();
                    nwg::modal_info_message(&window, "Input", &format!("You typed: {}", text));
                }
            }
            _ => {}
        }
    });

    nwg::dispatch_thread_events();
    nwg::unbind_event_handler(&handler);
}
