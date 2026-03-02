use iocraft::prelude::*;

#[derive(Default, Props)]
pub struct SpinnerProps {
    pub active: bool,
    pub frame: usize,
}

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

#[component]
pub fn Spinner(props: &SpinnerProps) -> impl Into<AnyElement<'static>> {
    if !props.active {
        return element! { Text(content: " ") };
    }

    let frame = FRAMES[props.frame % FRAMES.len()];
    element! {
        Text(content: format!("{frame} "), color: Color::Cyan)
    }
}
