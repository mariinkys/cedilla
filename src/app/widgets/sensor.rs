#![allow(dead_code)]

// This is an adapted version of the official iced sensor widget: https://github.com/iced-rs/iced/blob/master/widget/src/sensor.rs
// it has been modified to work with the current version of libcosmic since it does not exist in libcosmic atm, original code by @hecrj

use cosmic::Renderer;
use cosmic::Theme;
use cosmic::iced_core::layout;
use cosmic::iced_core::mouse;
use cosmic::iced_core::overlay;
use cosmic::iced_core::renderer;
use cosmic::iced_core::time::{Duration, Instant};
use cosmic::iced_core::widget;
use cosmic::iced_core::widget::tree::{self, Tree};
use cosmic::iced_core::window;
use cosmic::iced_core::{
    self as core, Element, Event, Layout, Length, Pixels, Point, Rectangle, Shell, Size, Vector,
    Widget,
};

/// A widget that can generate messages when its content pops in and out of view.
pub fn sensor<'a, Message>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Sensor<'a, (), Message> {
    Sensor::new(content)
}

pub struct Sensor<'a, Key, Message> {
    content: Element<'a, Message, Theme, Renderer>,
    key: Key,
    on_show: Option<Box<dyn Fn(Size) -> Message + 'a>>,
    on_resize: Option<Box<dyn Fn(Size) -> Message + 'a>>,
    on_hide: Option<Message>,
    anticipate: Pixels,
    delay: Duration,
}

impl<'a, Message> Sensor<'a, (), Message> {
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self {
        Self {
            content: content.into(),
            key: (),
            on_show: None,
            on_resize: None,
            on_hide: None,
            anticipate: Pixels::ZERO,
            delay: Duration::ZERO,
        }
    }
}

impl<'a, Key, Message> Sensor<'a, Key, Message>
where
    Key: self::Key,
{
    pub fn on_show(mut self, on_show: impl Fn(Size) -> Message + 'a) -> Self {
        self.on_show = Some(Box::new(on_show));
        self
    }

    pub fn on_resize(mut self, on_resize: impl Fn(Size) -> Message + 'a) -> Self {
        self.on_resize = Some(Box::new(on_resize));
        self
    }

    pub fn on_hide(mut self, on_hide: Message) -> Self {
        self.on_hide = Some(on_hide);
        self
    }

    pub fn key<K>(self, key: K) -> Sensor<'a, impl self::Key, Message>
    where
        K: Clone + PartialEq + 'static,
    {
        Sensor {
            content: self.content,
            key: OwnedKey(key),
            on_show: self.on_show,
            on_resize: self.on_resize,
            on_hide: self.on_hide,
            anticipate: self.anticipate,
            delay: self.delay,
        }
    }

    pub fn key_ref<K>(self, key: &'a K) -> Sensor<'a, &'a K, Message>
    where
        K: ToOwned + PartialEq<K::Owned> + ?Sized,
        K::Owned: 'static,
    {
        Sensor {
            content: self.content,
            key,
            on_show: self.on_show,
            on_resize: self.on_resize,
            on_hide: self.on_hide,
            anticipate: self.anticipate,
            delay: self.delay,
        }
    }

    pub fn anticipate(mut self, distance: impl Into<Pixels>) -> Self {
        self.anticipate = distance.into();
        self
    }

    pub fn delay(mut self, delay: impl Into<Duration>) -> Self {
        self.delay = delay.into();
        self
    }
}

#[derive(Debug, Clone)]
struct State<Key> {
    has_popped_in: bool,
    should_notify_at: Option<(bool, Instant)>,
    last_size: Option<Size>,
    last_key: Key,
}

// Helper function to calculate distance from a point to a rectangle
fn distance_to_rect(rect: &Rectangle, point: Point) -> f32 {
    let dx = (rect.x - point.x)
        .max(0.0)
        .max(point.x - (rect.x + rect.width));
    let dy = (rect.y - point.y)
        .max(0.0)
        .max(point.y - (rect.y + rect.height));
    (dx * dx + dy * dy).sqrt()
}

impl<Key, Message> Widget<Message, Theme, Renderer> for Sensor<'_, Key, Message>
where
    Key: self::Key,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Key::Owned>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            has_popped_in: false,
            should_notify_at: None,
            last_size: None,
            last_key: self.key.to_owned(),
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &cosmic::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut cosmic::Renderer,
        theme: &cosmic::Theme,
        style: &renderer::Style,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &self,
        tree: &mut Tree,
        layout: core::Layout<'_>,
        renderer: &cosmic::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.content
            .as_widget()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &cosmic::Renderer,
        clipboard: &mut dyn core::Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> core::event::Status {
        if let Event::Window(window::Event::RedrawRequested(now)) = &event {
            let state = tree.state.downcast_mut::<State<Key::Owned>>();

            if state.has_popped_in && !self.key.eq(&state.last_key) {
                state.has_popped_in = false;
                state.should_notify_at = None;
                state.last_key = self.key.to_owned();
            }

            let bounds = layout.bounds();

            // Calculate distance from viewport to bounds
            let top_left_distance = distance_to_rect(viewport, bounds.position());
            let bottom_right_distance =
                distance_to_rect(viewport, bounds.position() + Vector::from(bounds.size()));
            let distance = top_left_distance.min(bottom_right_distance);

            if self.on_show.is_none() {
                if let Some(on_resize) = &self.on_resize {
                    let size = bounds.size();
                    if Some(size) != state.last_size {
                        state.last_size = Some(size);
                        shell.publish(on_resize(size));
                    }
                }
            } else if state.has_popped_in {
                if distance <= self.anticipate.0 {
                    if let Some(on_resize) = &self.on_resize {
                        let size = bounds.size();
                        if Some(size) != state.last_size {
                            state.last_size = Some(size);
                            shell.publish(on_resize(size));
                        }
                    }
                } else if self.on_hide.is_some() {
                    state.has_popped_in = false;
                    state.should_notify_at = Some((false, *now + self.delay));
                }
            } else if distance <= self.anticipate.0 {
                let size = bounds.size();
                state.has_popped_in = true;
                state.should_notify_at = Some((true, *now + self.delay));
                state.last_size = Some(size);
            }

            match &state.should_notify_at {
                Some((has_popped_in, at)) if at <= now => {
                    if *has_popped_in {
                        if let Some(on_show) = &self.on_show {
                            shell.publish(on_show(layout.bounds().size()));
                        }
                    } else if let Some(on_hide) = self.on_hide.take() {
                        shell.publish(on_hide);
                    }
                    state.should_notify_at = None;
                }
                Some((_, at)) => {
                    shell.request_redraw(window::RedrawRequest::At(*at));
                }
                None => {}
            }
        }

        self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: core::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &cosmic::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: core::Layout<'_>,
        renderer: &cosmic::Renderer,
        translation: core::Vector,
    ) -> Option<overlay::Element<'b, Message, cosmic::Theme, cosmic::Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(&mut tree.children[0], layout, renderer, translation)
    }
}

impl<'a, Key, Message> From<Sensor<'a, Key, Message>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Key: self::Key + 'a,
{
    fn from(sensor: Sensor<'a, Key, Message>) -> Self {
        Element::new(sensor)
    }
}

pub trait Key {
    type Owned: 'static;
    fn to_owned(&self) -> Self::Owned;
    fn eq(&self, other: &Self::Owned) -> bool;
}

impl<T> Key for &T
where
    T: ToOwned + PartialEq<T::Owned> + ?Sized,
    T::Owned: 'static,
{
    type Owned = T::Owned;

    fn to_owned(&self) -> <Self as Key>::Owned {
        ToOwned::to_owned(*self)
    }

    fn eq(&self, other: &Self::Owned) -> bool {
        *self == other
    }
}

struct OwnedKey<T>(T);

impl<T> Key for OwnedKey<T>
where
    T: PartialEq + Clone + 'static,
{
    type Owned = T;

    fn to_owned(&self) -> Self::Owned {
        self.0.clone()
    }

    fn eq(&self, other: &Self::Owned) -> bool {
        &self.0 == other
    }
}

impl Key for () {
    type Owned = ();
    fn to_owned(&self) -> Self::Owned {}
    fn eq(&self, _other: &Self::Owned) -> bool {
        true
    }
}
