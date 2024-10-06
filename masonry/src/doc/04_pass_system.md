**TODO - This is copy-pasted from the pass spec RFC, needs to be edited.**

Masonry has a set of **passes**, which are computations run over a subset of the widget tree during a frame.


## Event passes

When a user interacts with the application in some way, like a mouse click, Masonry runs an **event pass** over the tree.
There are three types of event passes:

- **on_pointer_event:** covers positional events from the mouse and other pointing devices (pen, stylus, touchpad, etc).
- **on_text_event:** text input events like keyboard presses, IME, clipboard paste, etc.
- **on_access_event:** events from the OS's accessibility API.

When an event occurs, the application selects the widget targeted by the event.
For pointer events, this is either the widget under the pointer or the widget with pointer capture.
For text and accessibility events, this is the widget with focus.

The widget's event handling method (`on_pointer_event`, `on_text_event`, or `on_access_event`) is called.
Then, the same method is called for each of the widget's parents, up to the root.
This behavior is known in browsers as event bubbling.


## Animation pass

The **update_anim** pass runs an animation frame, which occurs at set intervals if the widget tree includes animated widgets.

It runs in depth-first preorder on all animated widgets in the tree.


## Rewrite passes

After an event pass, some flags may have been changed, and some values may have been invalidated and need to be recomputed.
To address these invalidations, Masonry runs a set of **rewrite passes** over the tree:

- **mutate** pass.
- **update_widget_tree** pass.
- **update_disabled** pass.
- **update_stashed** pass.
- **update_focus_chain** pass.
- **update_focus** pass.
- **layout** pass.
- **update_scrolls** pass.
- **compose** pass.
- **update_pointer** pass.

The layout and compose passes have methods with matching names in the Widget trait.
The update_xxx passes call the widgets' update method.

By default, each of these passes completes immediately, unless pass-dependent invalidation flags are set or work is requested.
Each pass can generally request work for later passes; for instance, the mutate pass can invalidate the layout of a widget, in which case the layout pass will run on that widget and its children and parents.

Passes may also request work for *previous* passes, in which case all rewrite passes are run again in sequence.
For instance, the update_pointer pass may change a widget's size, requiring another layout pass.

To avoid infinite loops in those cases, the number of reruns has a static limit.
If passes are still requested past that limit, they're delayed to a later frame.

### The mutate pass

The **mutate** pass runs a list of callbacks with mutable access to the widget tree.
These callbacks can be queued with the `mutate_later()` method of various context types.

"Mutable access" means that those callbacks are given a `WidgetMut` to the widget that requested them, something that is otherwise only accessible from the owner of the global `RenderRoot` object (see "External Mutation" section).

If a callback is scheduled to run on a widget which is deleted before the callback is run, that callback is silently dropped.

*Note:* The mutate pass is meant to be *an escape hatch*.
It covers widgets which don't quite fit into the pass system and future use-cases that we didn't foresee while developing Masonry.
It's more powerful and gives complete access to the tree, but is also slightly more expensive and less idiomatic than doing things in other passes.

Widgets should try to fit their logic into the other passes, and use `mutate_later()` sparsely.

### Update passes

Update passes mostly run internal calculations.
They compute if some widget's property has changed, and send it a matching `update` event (see "Status" section below).

For instance, if a user presses `Tab` and the event isn't handled in a widget, Masonry will run the `update_focus` pass, which will automatically switch keyboard focus to the next focus-accepting widget.
Both the previously-focused widget and the newly-focused widget will get an `update` call with relevant values.

### Update tree pass

The `update_widget_tree` pass is a special case.
It is called when new widgets are added to the tree, or existing widgets are removed.

It will call `register_children` widget method on container widgets whose children changed, then the `update` method with the `WidgetAdded` event on new widgets.

### Layout pass

The layout pass runs bidirectionally, passing constraints from the top down and getting back sizes and other layout info from the bottom up.

It is subject to be reworked in the future to be closer to the semantics of web layout engines and the Taffy crate.

Unlike with other passes, container widgets' `Widget::layout()` method must call `WidgetPod::layout()` on all of their children.

Not doing so is a logical bug.
When debug assertions are on, Masonry will actively try to detect cases where you forget to compute a child's layout and panic if it finds such a case.

### Compose pass

The **compose** pass runs top-down and assigns transforms to children.
Transform-only layout changes (e.g. scrolling) should request compose instead of requesting layout.

Compose is meant to be a cheaper way to position widgets than layout.
Because the compose pass is more limited than layout, it's easier to recompute in many situations.

For instance, if a widget in a list changes size, its siblings and parents must be re-laid out to account for the change; whereas changing a given widget's transform only affects its children.

Masonry automatically calls the `compose` methods of all widgets in the tree, in depth-first preorder, where child order is determined by their position in the `children_ids()` array.


## Render passes

Event and rewrite passes can invalidate how the widget tree is presented to the user.

If that happens, a redraw frame will be requested from the environment (e.g. the Winit event loop).
When the environment applies the redraw, it will run the **render passes** as needed:

- **paint:** The paint pass gets a Vello Scene description from each widget.
These scenes are then stitched together in pre-order: first the parent, then its first child, then *its* first child, etc.
- **accessibility:** The accessibility pass gets an AccessKit node description from each widget.
These nodes together form the accessibility tree.

Methods for these passes should be written under the assumption that they can be skipped or called multiple times for arbitrary reasons.
Therefore, their ability to affect the widget tree is limited.

Masonry automatically calls these methods for all widgets in the tree in depth-first preorder.

## External mutation

Code with mutable access to the `RenderRoot`, like the Xilem app runner, can get mutable access to the root widget and all its children through the `edit_root_widget()` method, which takes a callback and passes it a `WidgetMut` to the root widget.

This is in effect a MUTATE pass which only processes one callback.

External mutation is how Xilem applies any changes to the widget tree produced by its reactive step.

Calling the `edit_root_widget()` method, or any similar direct-mutation method, triggers the entire set of rewrite passes.
