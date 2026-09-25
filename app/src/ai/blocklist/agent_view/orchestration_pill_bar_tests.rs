#[test]
fn pill_bar_scrollable_finite_under_capped_drag_preview() {
    use pathfinder_geometry::vector::vec2f;
    use warpui::elements::new_scrollable::{NewScrollable, SingleAxisConfig};
    use warpui::elements::{
        ClippedScrollStateHandle, ConstrainedBox, Container, CrossAxisAlignment, Fill, Flex,
        MainAxisSize, ParentElement, Rect,
    };
    use warpui::platform::WindowStyle;
    use warpui::{
        App, Element, Entity, Presenter, TypedActionView, View, ViewContext, WindowInvalidation,
    };

    // Mirror of `PaneView::DRAG_PREVIEW_HEADER_MAX_WIDTH`; kept local so the test
    // documents the finite cap the fix relies on.
    const DRAG_PREVIEW_HEADER_MAX_WIDTH: f32 = 400.;

    struct DragPreviewTestView {
        scroll_state: ClippedScrollStateHandle,
    }

    impl DragPreviewTestView {
        fn new(_ctx: &mut ViewContext<Self>) -> Self {
            Self {
                scroll_state: ClippedScrollStateHandle::new(),
            }
        }
    }

    impl Entity for DragPreviewTestView {
        type Event = ();
    }

    impl View for DragPreviewTestView {
        fn ui_name() -> &'static str {
            "DragPreviewTestView"
        }

        fn render(&self, _app: &warpui::AppContext) -> Box<dyn Element> {
            // Overflowing content so the clipped scrollable has something to clip.
            let content = ConstrainedBox::new(Rect::new().finish())
                .with_width(2000.)
                .with_height(22.)
                .finish();
            let scrollable = NewScrollable::horizontal(
                SingleAxisConfig::Clipped {
                    handle: self.scroll_state.clone(),
                    child: Container::new(content).finish(),
                },
                Fill::None,
                Fill::None,
                Fill::None,
            )
            .finish();
            let bar = Container::new(scrollable).finish();
            // The pane-header content column stretches the bar to the header width.
            let header_column = Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_main_axis_size(MainAxisSize::Min)
                .with_child(bar)
                .finish();
            // The fix: the drag-preview column caps the header to a finite width.
            let drag_preview = Flex::column()
                .with_main_axis_size(MainAxisSize::Min)
                .with_child(
                    ConstrainedBox::new(header_column)
                        .with_max_width(DRAG_PREVIEW_HEADER_MAX_WIDTH)
                        .finish(),
                )
                .finish();
            // Outer row hands the drag-preview column an unbounded max width.
            Flex::row()
                .with_main_axis_size(MainAxisSize::Min)
                .with_child(drag_preview)
                .finish()
        }
    }

    impl TypedActionView for DragPreviewTestView {
        type Action = ();
    }

    App::test((), |mut app| async move {
        let (window_id, _view) =
            app.add_window(WindowStyle::NotStealFocus, DragPreviewTestView::new);
        let root_view_id = app
            .root_view_id(window_id)
            .expect("window should have a root view");

        let mut presenter = Presenter::new(window_id);
        let invalidation = WindowInvalidation {
            updated: [root_view_id].into_iter().collect(),
            ..Default::default()
        };

        app.update(move |ctx| {
            presenter.invalidate(invalidation, ctx);
            // Before the fix this panics in `Scene::validate_rect` while painting
            // the scrollable at an infinite/NaN width.
            let scene = presenter.build_scene(vec2f(400., 300.), 1., None, ctx);

            // Every painted rect must have a finite size. Without the width cap,
            // the clipped scrollable would paint an infinite/NaN-width rect.
            for layer in scene.layers() {
                for rect in &layer.rects {
                    let size = rect.bounds.size();
                    assert!(
                        size.x().is_finite() && size.y().is_finite(),
                        "painted rect should be finite under a capped drag preview, got {size:?}",
                    );
                }
            }
        });
    });
}
