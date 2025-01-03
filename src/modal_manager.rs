use gpui::*;

pub trait ModalView: ManagedView {}

trait ModalViewHandle {
    fn view(&self) -> AnyView;
}

impl<V: ModalView> ModalViewHandle for View<V> {
    fn view(&self) -> AnyView {
        self.clone().into()
    }
}

pub struct ActiveModal {
    modal: Box<dyn ModalViewHandle>,
    _subscriptions: [Subscription; 2],
    previous_focus_handle: Option<FocusHandle>,
    focus_handle: FocusHandle,
}

pub struct ModalManager {
    active_modal: Option<ActiveModal>,
}

impl ModalManager {
    pub fn new(_cx: &mut ViewContext<Self>) -> Self {
        Self { active_modal: None }
    }

    pub fn toggle_modal<V, B>(&mut self, cx: &mut ViewContext<Self>, build_view: B)
    where
        V: ModalView,
        B: FnOnce(&mut ViewContext<V>) -> V,
    {
        if let Some(active_modal) = &self.active_modal {
            let is_close = active_modal.modal.view().downcast::<V>().is_ok();
            let did_close = self.hide_modal(cx);
            if is_close || !did_close {
                return;
            }
        }

        let new_modal = cx.new_view(build_view);
        self.show_modal(new_modal, cx);
    }

    fn show_modal<V>(&mut self, new_modal: View<V>, cx: &mut ViewContext<Self>)
    where
        V: ModalView,
    {
        let focus_handle = cx.focus_handle();

        self.active_modal = Some(ActiveModal {
            modal: Box::new(new_modal.clone()),
            _subscriptions: [
                cx.subscribe(&new_modal, |this, _, _: &DismissEvent, cx| {
                    this.hide_modal(cx);
                }),
                cx.on_focus_out(&focus_handle, |this, _, cx| {
                    this.hide_modal(cx);
                }),
            ],
            previous_focus_handle: cx.focused(),
            focus_handle,
        });

        cx.defer(move |_, cx| cx.focus_view(&new_modal));
        cx.notify();
    }

    fn hide_modal(&mut self, cx: &mut ViewContext<Self>) -> bool {
        let Some(_) = self.active_modal.as_mut() else {
            return false;
        };

        if let Some(active_modal) = self.active_modal.take() {
            if let Some(previous_focus) = active_modal.previous_focus_handle {
                previous_focus.focus(cx);
            }
            cx.notify();
        }

        true
    }
}

impl Render for ModalManager {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let Some(active_modal) = &self.active_modal else {
            return div();
        };

        div()
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(rgba(0x000000aa))
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, cx| {
                    this.hide_modal(cx);
                }),
            )
            .child(
                div()
                    .top_20()
                    .flex()
                    .flex_col()
                    .items_center()
                    .track_focus(&active_modal.focus_handle)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .occlude()
                            .child(active_modal.modal.view()),
                    ),
            )
    }
}
