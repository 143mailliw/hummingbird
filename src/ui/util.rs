use std::sync::Arc;

use ahash::AHashMap;
use gpui::{App, Entity, Render, RenderImage};
use tracing::debug;

pub fn prune_views<T>(
    views_model: &Entity<AHashMap<usize, Entity<T>>>,
    render_counter: &Entity<usize>,
    current: usize,
    cx: &mut App,
) -> bool
where
    T: Render,
{
    let last = *render_counter.read(cx);
    let mut to_remove: Vec<usize> = Vec::new();
    let mut did_remove = false;

    // determine whether or not we are at the start of a new render cycle
    if current < last {
        // we are at the start of a new render cycle
        // prune views that are no longer in the bounds (current..last)
        // don't prune the first view so this still works with uniform_list
        for (idx, _) in views_model.read(cx).iter() {
            if (*idx < current || *idx >= (last + 1)) && *idx != 0_usize {
                to_remove.push(*idx);
            }
        }
    }

    for idx in to_remove {
        did_remove = true;
        views_model.update(cx, |m, _| {
            debug!("Removing view at index: {}", idx);
            m.remove(&idx);
        });
    }

    // update the render counter
    render_counter.update(cx, |m, _| {
        *m = current;
    });

    did_remove
}

pub fn create_or_retrieve_view<T>(
    views_model: &Entity<AHashMap<usize, Entity<T>>>,
    idx: usize,
    creation_fn: impl FnOnce(&mut App) -> Entity<T>,
    cx: &mut App,
) -> Entity<T>
where
    T: Render,
{
    let view = views_model.read(cx).get(&idx).cloned();
    match view {
        Some(view) => view,
        None => {
            let view = creation_fn(cx);
            views_model.update(cx, |m, _| {
                m.insert(idx, view.clone());
            });
            view
        }
    }
}

pub fn drop_image_from_app(cx: &mut App, image: Arc<RenderImage>) {
    for window in cx.windows() {
        let image = image.clone();

        debug!("dropping an image from {:?}", window.window_id());

        window
            .update(cx, move |_, window, _| {
                window.drop_image(image).expect("bruh");
            })
            .expect("couldn't get window");
    }
}
