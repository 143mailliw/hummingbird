use gpui::*;
use smallvec::SmallVec;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::ui::theme::Theme;

pub struct ContextMenu {
    pub(self) id: ElementId,
    pub(self) style: StyleRefinement,
    pub(self) element: Option<AnyElement>,
    pub(self) menu: Option<Div>,
}

impl ContextMenu {
    pub fn with(mut self, element: impl IntoElement) -> Self {
        self.element = Some(element.into_any_element());
        self
    }
}

impl Styled for ContextMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl IntoElement for ContextMenu {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl ParentElement for ContextMenu {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.menu.as_mut().unwrap().extend(elements);
    }
}

struct ContextMenuState {
    pub clicked_in: AtomicBool,
    pub position: RefCell<Option<Point<Pixels>>>,
}

impl ContextMenuState {
    pub fn new() -> Self {
        ContextMenuState {
            clicked_in: AtomicBool::new(false),
            position: RefCell::new(None),
        }
    }
}

impl Element for ContextMenu {
    type RequestLayoutState = Option<(Anchored, LayoutId, AnchoredState)>;

    type PrepaintState = Option<Bounds<Pixels>>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style::default();

        let mut layout_ids: SmallVec<[LayoutId; 2]> = SmallVec::new();

        if let Some(element) = self.element.as_mut() {
            layout_ids.push(element.request_layout(window, cx));
        }

        let menu = self.menu.take();

        let theme = cx.global::<Theme>();

        let anchored =
            window.with_element_state(id.unwrap(), move |prev: Option<Rc<ContextMenuState>>, _| {
                let state = prev.unwrap_or_else(|| Rc::new(ContextMenuState::new()));

                let point = *state.position.borrow();

                if let (Some(position), Some(menu)) = (point, menu) {
                    let state_clone = state.clone();
                    let state_clone_2 = state.clone();

                    let new = anchored().position(position).child(deferred(
                        menu.occlude()
                            .border_1()
                            .shadow_sm()
                            .rounded(px(6.0))
                            .border_color(theme.elevated_border_color)
                            .bg(theme.elevated_background)
                            .id("menu")
                            .on_click(move |_, window, _| {
                                (*state_clone.position.borrow_mut()) = None;
                                window.refresh()
                            })
                            .on_mouse_down_out(move |_, window, _| {
                                (*state_clone_2.position.borrow_mut()) = None;
                                window.refresh()
                            }),
                    ));
                    (Some(new), state)
                } else {
                    (None, state)
                }
            });

        let state = if let Some(mut anchored) = anchored {
            let layout = anchored.request_layout(None, None, window, cx);
            layout_ids.push(layout.0);
            Some((anchored, layout.0, layout.1))
        } else {
            None
        };

        (window.request_layout(style, layout_ids, cx), state)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(element) = self.element.as_mut() {
            element.prepaint(window, cx);
        }

        if let Some(anchored) = request_layout {
            let bounds = window.layout_bounds(anchored.1);

            anchored.0.prepaint(
                None,
                None,
                window.layout_bounds(anchored.1),
                &mut anchored.2,
                window,
                cx,
            );

            Some(bounds)
        } else {
            None
        }
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(element) = self.element.as_mut() {
            element.paint(window, cx);
        }

        if let Some(anchored) = request_layout {
            anchored.0.paint(
                None,
                None,
                prepaint.unwrap(),
                &mut anchored.2,
                &mut (),
                window,
                cx,
            );
        }

        window.with_element_state(id.unwrap(), |prev: Option<Rc<ContextMenuState>>, cx| {
            let state = prev.unwrap_or_else(|| Rc::new(ContextMenuState::new()));
            let state_clone = state.clone();

            cx.on_mouse_event(move |ev: &MouseDownEvent, phase, _, _| {
                if ev.button == MouseButton::Right
                    && phase == DispatchPhase::Bubble
                    && bounds.contains(&ev.position)
                {
                    state_clone.clicked_in.store(true, Ordering::Release);
                }
            });

            let state_clone_2 = state.clone();

            cx.on_mouse_event(move |ev: &MouseUpEvent, phase, _, _| {
                if phase == DispatchPhase::Bubble {
                    let clicked_in = state_clone_2.clicked_in.swap(false, Ordering::AcqRel);

                    if ev.button == MouseButton::Right
                        && bounds.contains(&ev.position)
                        && clicked_in
                    {
                        (*state_clone_2.position.borrow_mut()) = Some(ev.position)
                    }
                }
            });

            ((), state)
        })
    }
}

pub fn context(id: impl Into<ElementId>) -> ContextMenu {
    ContextMenu {
        id: id.into(),
        style: StyleRefinement::default(),
        element: None,
        menu: Some(div()),
    }
}
