use std::{cell::RefCell, rc::Rc};

use gpui::*;

use crate::ui::theme::Theme;

type ClickHandler = dyn FnMut(f32, &mut Window, &mut App);

pub struct Slider {
    pub(self) id: Option<ElementId>,
    pub(self) style: StyleRefinement,
    pub(self) value: f32,
    pub(self) on_change: Option<Rc<RefCell<ClickHandler>>>,
    pub(self) hitbox: Option<Hitbox>,
}

impl Slider {
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    pub fn on_change(mut self, func: impl FnMut(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(RefCell::new(func)));
        self
    }
}

impl Styled for Slider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for Slider {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Slider {
    type RequestLayoutState = ();

    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        self.id.clone()
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.refine(&self.style);
        (window.request_layout(style, [], cx), ())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        self.hitbox = Some(window.insert_hitbox(bounds, HitboxBehavior::Normal));
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let theme = cx.global::<Theme>();
        let default_background = theme.slider_background;
        let default_foreground = theme.slider_foreground;

        let mut inner_bounds = bounds;
        inner_bounds.size.width = bounds.size.width * self.value;

        let mut corners = Corners::default();
        corners.refine(&self.style.corner_radii);

        window.set_cursor_style(CursorStyle::PointingHand, self.hitbox.as_ref().unwrap());

        window.paint_quad(quad(
            bounds,
            corners.to_pixels(window.rem_size()),
            self.style
                .background
                .clone()
                .and_then(|v| v.color())
                .unwrap_or(default_background.into()),
            Edges::all(px(0.0)),
            rgb(0x000000),
            BorderStyle::Solid,
        ));

        let mut borders = Edges::default();
        borders.refine(&self.style.border_widths);

        window.paint_quad(quad(
            inner_bounds,
            corners.to_pixels(window.rem_size()),
            self.style
                .text
                .clone()
                .and_then(|v| v.color)
                .unwrap_or(default_foreground.into()),
            borders.to_pixels(window.rem_size()),
            self.style.border_color.unwrap_or_default(),
            BorderStyle::Solid,
        ));

        if let Some(func) = self.on_change.as_ref() {
            window.with_optional_element_state(
                id,
                move |v: Option<Option<Rc<RefCell<bool>>>>, cx| {
                    let mouse_in = v.flatten().unwrap_or_else(|| Rc::new(RefCell::new(false)));
                    let func = func.clone();
                    let func_copy = func.clone();

                    let mouse_in_1 = mouse_in.clone();

                    cx.on_mouse_event(move |ev: &MouseDownEvent, _, window, cx| {
                        if !bounds.contains(&ev.position) {
                            return;
                        }

                        window.prevent_default();
                        cx.stop_propagation();

                        let relative = ev.position - bounds.origin;
                        let relative_x: f32 = relative.x.into();
                        let width: f32 = bounds.size.width.into();
                        let value = (relative_x / width).clamp(0.0, 1.0);

                        (func.borrow_mut())(value, window, cx);
                        (*mouse_in_1.borrow_mut()) = true;
                    });

                    let mouse_in_2 = mouse_in.clone();

                    cx.on_mouse_event(move |ev: &MouseMoveEvent, _, window, cx| {
                        if *mouse_in_2.borrow() {
                            let relative = ev.position - bounds.origin;
                            let relative_x: f32 = relative.x.into();
                            let width: f32 = bounds.size.width.into();
                            let value = (relative_x / width).clamp(0.0, 1.0);

                            (func_copy.borrow_mut())(value, window, cx);
                        }
                    });

                    let mouse_in_3 = mouse_in.clone();

                    cx.on_mouse_event(move |_: &MouseUpEvent, _, _, _| {
                        (*mouse_in_3.borrow_mut()) = false;
                    });

                    ((), Some(mouse_in))
                },
            )
        }
    }
}

pub fn slider() -> Slider {
    Slider {
        id: None,
        style: StyleRefinement::default(),
        value: 0.0,
        on_change: None,
        hitbox: None,
    }
}
