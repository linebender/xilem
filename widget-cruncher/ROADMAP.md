# Design stuff

What referential are various values using?

Flex algo
- Does Tennent's Correspondence Principle apply to Wrappers in a Flex container?
- Visualize how much "spring" each element has

transform_scroll translates mouse events

WidgetAdded and similar events are an anti-pattern. Constructors should stand on their own. (is it actually possible though?)



Passes have access to
- on_event
-> mut self, mut ContextState, WidgetState.<layout>, mut WidgetState.<passes>, mut WidgetState.<Status>, get_widget_view()
- on_status_change
-> mut self, ContextState, WidgetState.<layout>, mut WidgetState.<passes>, WidgetState.<Status>
- lifecycle
-> mut self, mut ContextState, WidgetState.<layout>, mut WidgetState.<passes>, mut WidgetState.<Status>, get_widget_view()
- layout
-> self, mut ContextState, mut WidgetState.<layout>, WidgetState.<passes>, WidgetState.<Status>
- paint
-> self, mut ContextState, mut WidgetState.<layout>, WidgetState.<passes>, WidgetState.<Status>
- WidgetView
-> mut self, mut ContextState, WidgetState.<layout>, mut WidgetState.<passes>, mut WidgetState.<Status>, get_widget_view()

Druid invariants
- layout != <prev>.layout                       => request_layout() has been called
- child.origin != <prev>.child.origin           => request_layout() has been called
- <paint-calls> != <prev>.<paint-calls>         => request_paint() has been called
- children() != <prev>.children()               => children_changed is set
- !event.handled && hovered || self.has_focus   => recurse event
- lifecycle                                     => recurse lifecycle
- !is_hidden()                                  => recurse layout
- !is_hidden() && is_visible()                  => recurse paint
- is_focusable                                  => must be const
- ids are unique




# Profiling

Record spans, with:
- pass name
- widget id
- work type
- useful/redundant/testing
- include-recursed-time-in-total-time

-> percentage of work that is useful
-> what type of work


# Trace handling

- Isolate subgraph of trace during errors



# TODO - MVP

- [ ] Add impl Deref for WidgetRef
- [ ] Remove WidgetId::reserved, move WidgetId::new
- [ ] Add downcast to WidgetRef and WidgetView
- [ ] Add WidgetView::as_ref
- [ ] Rename ContextState to GlobalPassCtx
- [ ] Remove ExtendDrain

- [ ] Rewrite tests
 - [X] Create coverage profile
 - [X] Look at coverage of every file, write a list of target coverage
 - [ ] Add timers to harness

- [ ] Add action queue
- [ ] Re-add ExtEvent + AppDelegate

- [ ] Write examples
 - [ ] To-do List
 - [X] Fix blocking_function example
 - [ ] Rewrite Disney example

- [ ] Use WidgetView everywhere
 - [ ] Button
 - [ ] Checkbox
 - [ ] Click
 - [ ] ClipBox
 - [ ] Flex
 - [ ] Image
 - [X] Label
 - [ ] SizedBox
 - [ ] Spinner
 - [ ] TextBox

- [ ] Refactor WidgetState
 - [ ] Sort WidgetState fields
 - [ ] Add WidgetTmpState
 - [ ] Extract WidgetState fields to WidgetTmpState + global state

- [ ] Refactor ScrollComponent / paint_rect / paint_insets
 - [ ] No paint_insets, set paint_rect automatically or manually
 - [ ] Rework set_origin
 - [ ] Check that paint_rect has children paint_rect
 - [ ] Disambiguate concept of hidden/stashed widgets
 - [ ] Fix invalidation

- [ ] Re-add Dialog
- [ ] Refactor TextLayout

- [ ] Rewrite License bits
- [ ] Write ARCHITECTURE.md
- [ ] Publish

# TODO - Long term

- [ ] Port Panoramix

- [ ] Move debug logger to use tracing infrastructure

- [ ] Refactor Mouse Events and hot status changes

- [ ] Write analysis of different layout footguns
 - Flex interacting with viewport
 - Does Tennent's Correspondence principle apply to Wrappers in a Flex container?
 - Difference in Harness(Button) vs Harness(Flex(Button))
 - Flex alignment vs text alignment

- [ ] Trait-ify druid-shell
 - [ ] ??? Start at the bottom?

- [ ] Add replay mode
 - [ ] Record platform events
 - [ ] Record timers
 - [ ] Add way to rr tests

- [ ] Add fuzzing
 - [ ] Check Druid invariants
 - [ ] Create fuzzing suite
 - [ ] Test it with intentional bugs

- [ ] Add profiling
 - [ ] Add artinasal timings manager
 - [ ] Start in separate crate with artificial benchmarks
 - [ ] Add "median/worst FPS" log at the end of each run.

- [ ] Write benchmarks
 - [ ] Try to find ways to bust the framework's framerate
 - [ ] Huge list
 - [ ] Huge list with editable/focusable boxes
 - [ ] ???

- [ ] Refactor Command infrastructure
 - [ ] Broadcast (used multiple times)
 - [ ] Command (used once)
 - [ ] ???
 - [ ] Add on_command() trait method

- [ ] Refactor Druid env
 - [ ] Remove Data trait
 - [ ] Make it so that required keys are known at launch time for better error messages.
 - [ ] Re-add Localization

- [ ] Switch to ECS
- [ ] Drag 'n Drop

## Tests to write

- [X] event
- [X] promise
- [X] utils
- [X] widget_pod
 -> [X] 'active' status
 -> [ ] viewport_offset
 -> [ ] paint_insets
 -> [X] debug_validate
 -> [X] check_initialized
 -> [X] check set_origin
 -> [X] MouseLeave
 -> [X] layout changes hot status
 -> [ ] pan_to_child
 -> [ ] ParentWindowOrigin
 -> [ ] debug_widget_text
 -> [ ] check for WidgetAdded
-> [X] widget utils
-> [ ] FillStrat
-> [ ] Widgets
 -> [ ] Button
 -> [ ] Checkbox
 -> [ ] Click
 -> [ ] ClipBox
 -> [ ] Flex
 -> [ ] Image
 -> [X] Label
 -> [ ] SizedBox
 -> [ ] Spinner
 -> [ ] TextBox
-> [ ] text
